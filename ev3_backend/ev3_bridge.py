import os
import time
import requests
import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import Optional

app = FastAPI(title="EV3 Controller Bridge Real")

# Estado global de configuración dinámico.
# Sustituye a las constantes fijas EV3_IP y EV3_URL.
config = {
    "mode": "simulated",
    "ip": "127.0.0.1",
    "port": os.environ.get("EV3_PORT", "8080")
}

TIMEOUT = 2          # segundos, para los comandos de motor
STATUS_TIMEOUT = 1   # segundos, para el polling de telemetría
MOTOR_PORTS = ["outA", "outB", "outC", "outD"]

# Reutilizar la conexión TCP evita rehacer el handshake 2,5 veces por segundo
SESSION = requests.Session()

# Cortacircuitos: si el robot no responde, dejamos de intentarlo durante un
# instante. El frontend pregunta cada 400 ms y cada intento fallido cuesta
# segundos; sin esto las peticiones se acumulan y el puente deja de responder.
FAIL_COOLDOWN = 1.5  # segundos
_retry_after = 0.0   # instante (monotónico) a partir del cual volver a probar

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


class ConfigPayload(BaseModel):
    mode: str
    ip: str


def get_ev3_url() -> str:
    """Calcula la URL objetivo en tiempo real basada en la configuración actual."""
    return "http://{}:{}".format(config["ip"], config["port"])


@app.post("/api/config")
def update_config(payload: ConfigPayload):
    """Recibe la nueva configuración desde el frontend (React) y cambia el objetivo."""
    global _retry_after
    config["mode"] = payload.mode
    config["ip"] = "127.0.0.1" if payload.mode == "simulated" else payload.ip
    # Al cambiar de objetivo reintentamos de inmediato, sin esperar el cortacircuitos
    _retry_after = 0.0
    print("🔄 Modo cambiado a: '{}' | Apuntando a: {}".format(config["mode"], config["ip"]))
    return {"status": "success", "current_config": config}


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
        "ip": config["ip"],
        "battery": 0.0,
        "motors": [{"port": p, "connected": False, "speed": 0} for p in MOTOR_PORTS],
        "sensors": [],
        "alerts": [reason],
    }


@app.post("/api/motor")
def set_motor(command: MotorCommand):
    """Envía el comando directamente al robot físico."""
    port = normalize_port(command.port)
    try:
        response = SESSION.post(
            "{}/move".format(get_ev3_url()),
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
def stop_all(command: StopCommand = StopCommand()):
    """Detiene un motor concreto, o todos los motores del robot real."""
    payload = {"motor": normalize_port(command.port)} if command.port else {}
    try:
        response = SESSION.post("{}/stop".format(get_ev3_url()), json=payload, timeout=TIMEOUT)
        response.raise_for_status()
        return {"status": "success", "message": "Motores detenidos"}
    except requests.RequestException as e:
        print("❌ Error al detener el EV3: {}".format(e))
        raise HTTPException(status_code=503, detail="No se pudo detener el robot")


@app.get("/api/status")
def get_status():
    """Telemetría del robot, normalizada al formato que consume el frontend."""
    global _retry_after

    # Si acabamos de fallar, respondemos al instante sin tocar la red. Así el
    # polling del frontend nunca se encola detrás de intentos que van a fallar.
    if time.monotonic() < _retry_after:
        return offline_status("Sin conexión con el robot ({})".format(config["ip"]))

    try:
        response = SESSION.get("{}/status".format(get_ev3_url()), timeout=STATUS_TIMEOUT)
        response.raise_for_status()
        data = response.json()
        _retry_after = 0.0
    except requests.RequestException:
        # Quitamos el print de error de estado para no hacer spam en la terminal
        # cuando el robot físico se apaga.
        _retry_after = time.monotonic() + FAIL_COOLDOWN
        return offline_status("Sin conexión con el robot ({})".format(config["ip"]))
    except ValueError:
        _retry_after = time.monotonic() + FAIL_COOLDOWN
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
        "ip": config["ip"],
        "battery": float(data.get("battery", 0.0)),
        "motors": motors,
        "sensors": sensors,
        "alerts": alerts,
    }


if __name__ == "__main__":
    print("🔌 Puente EV3 escuchando en :8000")
    uvicorn.run(app, host="0.0.0.0", port=8000)