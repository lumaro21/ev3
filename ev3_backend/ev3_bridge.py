import asyncio
import os
import time
from typing import Optional

import requests
import uvicorn
from fastapi import FastAPI, File, Form, HTTPException, UploadFile, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

try:
    import asyncssh
except ImportError:  # pragma: no cover - se resuelve en el entorno real del EV3
    asyncssh = None

app = FastAPI(title="EV3 Controller Bridge Real")

config = {
    "mode": "simulated",
    "ip": "127.0.0.1",
    "port": os.environ.get("EV3_PORT", "8080"),
    "ssh_port": int(os.environ.get("EV3_SSH_PORT", "22")),
    "ssh_user": os.environ.get("EV3_SSH_USER", "robot"),
    "ssh_password": os.environ.get("EV3_SSH_PASSWORD", "maker"),
}

TIMEOUT = 2
STATUS_TIMEOUT = 1
MOTOR_PORTS = ["outA", "outB", "outC", "outD"]
SESSION = requests.Session()
FAIL_COOLDOWN = 1.5
_retry_after = 0.0

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


class MotorCommand(BaseModel):
    port: str
    speed: int


class StopCommand(BaseModel):
    port: Optional[str] = None


class ConfigPayload(BaseModel):
    mode: str
    ip: str


def get_ev3_url() -> str:
    return "http://{}:{}".format(config["ip"], config["port"])


@app.post("/api/config")
def update_config(payload: ConfigPayload):
    global _retry_after
    config["mode"] = payload.mode
    config["ip"] = "127.0.0.1" if payload.mode == "simulated" else payload.ip
    _retry_after = 0.0
    print("🔄 Modo cambiado a: '{}' | Apuntando a: {}".format(config["mode"], config["ip"]))
    return {"status": "success", "current_config": config}


def normalize_port(port: str) -> str:
    p = str(port).strip().split(":")[-1]
    if not p.startswith("out"):
        p = "out" + p.upper()
    return p[:3] + p[3:].upper()


def offline_status(reason: str):
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
    global _retry_after
    if time.monotonic() < _retry_after:
        return offline_status("Sin conexión con el robot ({})".format(config["ip"]))

    try:
        response = SESSION.get("{}/status".format(get_ev3_url()), timeout=STATUS_TIMEOUT)
        response.raise_for_status()
        data = response.json()
        _retry_after = 0.0
    except requests.RequestException:
        _retry_after = time.monotonic() + FAIL_COOLDOWN
        return offline_status("Sin conexión con el robot ({})".format(config["ip"]))
    except ValueError:
        _retry_after = time.monotonic() + FAIL_COOLDOWN
        return offline_status("El robot devolvió una respuesta no válida")

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

    sensors = [
        {
            "port": str(s.get("port", "")),
            "sensor_type": str(s.get("sensor_type", "Unknown")),
            "value": float(s.get("value", 0)),
        }
        for s in data.get("sensors", [])
        if isinstance(s, dict) and s.get("port")
    ]

    alerts = ["Motor {} desconectado".format(m["port"]) for m in motors if not m["connected"]]

    return {
        "connected": True,
        "ip": config["ip"],
        "battery": float(data.get("battery", 0.0)),
        "motors": motors,
        "sensors": sensors,
        "alerts": alerts,
    }


async def open_ssh_shell(host: str, port: int, username: str, password: str):
    if asyncssh is None:
        raise RuntimeError("Falta la dependencia 'asyncssh'. Instálala con: pip install asyncssh")

    conn = await asyncssh.connect(
        host=host,
        port=port,
        username=username,
        password=password,
        known_hosts=None,
        client_keys=None,
        connect_timeout=10,
    )
    process = await conn.create_shell(term_type="xterm", encoding="utf-8")
    return conn, process


@app.websocket("/ws/terminal")
async def terminal_websocket(websocket: WebSocket):
    await websocket.accept()

    # El destino SSH es siempre el robot configurado vía /api/config: no se
    # aceptan credenciales por query string para no exponer la contraseña en la URL.
    host = config["ip"]
    port = config["ssh_port"]
    username = config["ssh_user"]
    password = config["ssh_password"]

    try:
        conn, process = await open_ssh_shell(host, port, username, password)
    except Exception as exc:  # pragma: no cover - depende del robot real
        await websocket.send_text(f"\r\n[ERROR] No se pudo abrir la sesión SSH: {exc}\r\n")
        await websocket.close(code=1011)
        return

    async def read_from_ssh():
        try:
            while True:
                chunk = await process.stdout.read(4096)
                if not chunk:
                    break
                await websocket.send_text(chunk)
        except Exception:
            pass
        finally:
            try:
                await websocket.close()
            except Exception:
                pass

    reader_task = asyncio.create_task(read_from_ssh())

    try:
        while True:
            incoming = await websocket.receive()
            if "text" not in incoming:
                continue
            data = incoming["text"]
            if not data:
                continue
            process.stdin.write(data)
            await process.stdin.drain()
    except WebSocketDisconnect:
        pass
    finally:
        reader_task.cancel()
        try:
            process.close()
        except Exception:
            pass
        try:
            conn.close()
        except Exception:
            pass


@app.post("/api/terminal/upload")
async def upload_terminal_script(
    file: UploadFile = File(...),
    remote_dir: str = Form("/home/robot"),
):
    if asyncssh is None:
        raise HTTPException(status_code=503, detail="Falta la dependencia 'asyncssh' en el backend")

    if not file.filename or not file.filename.lower().endswith((".py", ".sh", ".bash")):
        raise HTTPException(status_code=400, detail="Solo se admiten archivos .py, .sh o .bash")

    content = await file.read()
    remote_path = "{}/{}".format(remote_dir.rstrip("/"), file.filename)

    try:
        async with asyncssh.connect(
            host=config["ip"],
            port=config["ssh_port"],
            username=config["ssh_user"],
            password=config["ssh_password"],
            known_hosts=None,
            client_keys=None,
            connect_timeout=10,
        ) as conn:
            async with conn.start_sftp_client() as sftp:
                async with sftp.open(remote_path, "wb") as remote_file:
                    await remote_file.write(content)
            if remote_path.lower().endswith((".sh", ".bash")):
                await conn.run("chmod +x {}".format(remote_path))
    except Exception as exc:
        raise HTTPException(status_code=502, detail="No se pudo subir el archivo al EV3: {}".format(exc))

    print("📤 Archivo subido al EV3: {}".format(remote_path))
    return {"status": "success", "path": remote_path}


if __name__ == "__main__":
    print("🔌 Puente EV3 escuchando en :8000")
    uvicorn.run(app, host="0.0.0.0", port=8000)