import os

import requests
import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import Optional

app = FastAPI(title="EV3 Controller Bridge Real")

# Configuración IP del robot físico.
# Se puede sobrescribir sin tocar el código: EV3_IP=192.168.1.50 python ev3_bridge.py
EV3_IP = os.environ.get("EV3_IP", "192.168.202.20")
EV3_PORT = os.environ.get("EV3_PORT", "8080")  # Puerto de main.py dentro del robot
EV3_URL = "http://{}:{}".format(EV3_IP, EV3_PORT)
TIMEOUT = 2  # segundos

MOTOR_PORTS = ["outA", "outB", "outC", "outD"]

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


class MotorCommand(BaseModel):
    port: str   # Ejemplo: "A" o "outA"
    speed: int  # Ejemplo: 50


class StopCommand(BaseModel):
    port: Optional[str] = None  # None = todos los motores


def normalize_port(port: str) -> str:
    """Acepta "A", "outA" o "ev3-ports:outA" y devuelve siempre "outA"."""
    p = str(port).strip().split(":")[-1]
    if not p.startswith("out"):
        p = "out" + p.upper()
    return p[:3] + p[3:].upper()


def offline_status(reason: str):
    """Estado con la forma que espera el frontend cuando el robot no responde.

    Devolver siempre la misma estructura evita que el React reviente al
    iterar sobre motors/sensors."""
    return {
        "connected": False,
        "ip": EV3_IP,
        "battery": 0.0,
        "motors": [{"port": p, "connected": False, "speed": 0} for p in MOTOR_PORTS],
        "sensors": [],
        "alerts": [reason],
    }


@app.post("/api/motor")
async def set_motor(command: MotorCommand):
    """Envía el comando directamente al robot físico."""
    port = normalize_port(command.port)
    try:
        response = requests.post(
            "{}/move".format(EV3_URL),
            json={"motor": port, "speed": command.speed},
            timeout=TIMEOUT,
        )
        response.raise_for_status()
        print("✅ Robot movido: {} a {}".format(port, command.speed))
        return {"status": "success", "message": "Motor movido exitosamente"}
    except requests.RequestException as e:
        print("❌ Error al conectar con el EV3: {}".format(e))
        raise HTTPException(status_code=503, detail="No se pudo conectar con el robot")


@app.post("/api/stop_all")
async def stop_all(command: StopCommand = StopCommand()):
    """Detiene un motor concreto, o todos los motores del robot real."""
    payload = {"motor": normalize_port(command.port)} if command.port else {}
    try:
        response = requests.post("{}/stop".format(EV3_URL), json=payload, timeout=TIMEOUT)
        response.raise_for_status()
        return {"status": "success", "message": "Motores detenidos"}
    except requests.RequestException as e:
        print("❌ Error al detener el EV3: {}".format(e))
        raise HTTPException(status_code=503, detail="No se pudo detener el robot")


@app.get("/api/status")
async def get_status():
    """Telemetría del robot, normalizada al formato que consume el frontend.

    Si el robot no responde devolvemos un estado "offline" con la misma forma
    en lugar de un error, para que el dashboard siga renderizando."""
    try:
        response = requests.get("{}/status".format(EV3_URL), timeout=TIMEOUT)
        response.raise_for_status()
        data = response.json()
    except requests.RequestException as e:
        print("❌ Error al leer el estado del EV3: {}".format(e))
        return offline_status("Sin conexión con el robot ({})".format(EV3_IP))
    except ValueError:
        return offline_status("El robot devolvió una respuesta no válida")

    # Motores: garantizamos los cuatro puertos aunque el robot informe menos
    reported = {
        normalize_port(m.get("port", "")): m
        for m in data.get("motors", [])
        if isinstance(m, dict) and m.get("port")
    }
    motors = []
    for port in MOTOR_PORTS:
        m = reported.get(port, {})
        motors.append({
            "port": port,
            "connected": bool(m.get("connected", False)),
            "speed": int(m.get("speed", 0)),
        })

    # Sensores: solo los que el robot reporta como presentes
    sensors = [
        {
            "port": str(s.get("port", "")),
            "sensor_type": str(s.get("sensor_type", "Unknown")),
            "value": float(s.get("value", 0)),
        }
        for s in data.get("sensors", [])
        if isinstance(s, dict) and s.get("port")
    ]

    # Sin icono: el Dashboard ya antepone "⚠" al renderizar cada alerta
    alerts = ["Motor {} desconectado".format(m["port"]) for m in motors if not m["connected"]]

    return {
        "connected": True,
        "ip": EV3_IP,
        "battery": float(data.get("battery", 0.0)),
        "motors": motors,
        "sensors": sensors,
        "alerts": alerts,
    }


if __name__ == "__main__":
    print("🔌 Puente EV3 escuchando en :8000 — robot esperado en {}".format(EV3_URL))
    uvicorn.run(app, host="0.0.0.0", port=8000)
