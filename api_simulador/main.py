import os
import signal
import uvicorn
from fastapi import FastAPI
from pydantic import BaseModel
from typing import Optional
from ev3dev2.motor import LargeMotor, OUTPUT_A, OUTPUT_B, OUTPUT_C, OUTPUT_D  # type: ignore

app = FastAPI(title="EV3 On-Brick Server")

# ─── Motores físicos ──────────────────────────────────────────────────────────
# Inicializamos los cuatro puertos con try-except para que el servidor arranque
# aunque los estudiantes olviden conectar algún cable.

_PORT_CONSTANTS = {
    "outA": OUTPUT_A,
    "outB": OUTPUT_B,
    "outC": OUTPUT_C,
    "outD": OUTPUT_D,
}

MOTOR_PORTS = ["outA", "outB", "outC", "outD"]
SENSOR_PORTS = ["in1", "in2", "in3", "in4"]

motors = {}
for _port, _const in _PORT_CONSTANTS.items():
    try:
        motors[_port] = LargeMotor(_const)
    except Exception:
        motors[_port] = None


def normalize_port(port: str) -> str:
    """Acepta "A", "outA" o "ev3-ports:outA" y devuelve siempre "outA"."""
    p = str(port).strip().split(":")[-1]
    if not p.startswith("out"):
        p = "out" + p.upper()
    return p[:3] + p[3:].upper()


def get_motor(port: str):
    return motors.get(normalize_port(port))


def get_motor_by_index(idx: int):
    """Mapea el índice legacy que envía Rust (0=B, 1=D) al motor físico."""
    return motors.get("outB") if idx == 0 else motors.get("outD")


# ─── Lectura del estado real desde sysfs ──────────────────────────────────────
# Leemos directamente de /sys en lugar de usar ev3dev2 para no depender de que
# cada sensor esté instanciado con la clase correcta.

TACHO_DIR = "/sys/class/tacho-motor"
SENSOR_DIR = "/sys/class/lego-sensor"
BATTERY_FILE = "/sys/class/power_supply/lego-ev3-battery/voltage_now"

DRIVER_TO_TYPE = {
    "lego-ev3-touch": "Touch",
    "lego-ev3-color": "Color",
    "lego-ev3-us": "Ultrasonic",
    "lego-ev3-gyro": "Gyro",
    "lego-ev3-ir": "Infrared",
}


def _read(path: str) -> str:
    try:
        with open(path) as f:
            return f.read().strip()
    except OSError:
        return ""


def _read_number(path: str, default: float = 0.0) -> float:
    raw = _read(path)
    try:
        return float(raw)
    except ValueError:
        return default


def _listdir(path: str):
    try:
        return sorted(os.listdir(path))
    except OSError:
        return []


def read_motors():
    """Devuelve los 4 puertos de salida, marcando cuáles tienen motor conectado."""
    found = {}
    for node in _listdir(TACHO_DIR):
        base = "{}/{}".format(TACHO_DIR, node)
        port = _read("{}/address".format(base)).split(":")[-1]
        if not port:
            continue
        found[port] = {
            "port": port,
            "connected": True,
            "speed": int(_read_number("{}/speed".format(base))),
        }

    return [
        found.get(p, {"port": p, "connected": False, "speed": 0})
        for p in MOTOR_PORTS
    ]


def read_sensors():
    """Devuelve solo los sensores realmente conectados."""
    sensors = []
    for node in _listdir(SENSOR_DIR):
        base = "{}/{}".format(SENSOR_DIR, node)
        port = _read("{}/address".format(base)).split(":")[-1]
        if not port:
            continue
        driver = _read("{}/driver_name".format(base))
        sensors.append({
            "port": port,
            "sensor_type": DRIVER_TO_TYPE.get(driver, driver or "Unknown"),
            "value": _read_number("{}/value0".format(base)),
        })
    return sensors


def read_battery() -> float:
    """Voltaje en voltios (sysfs lo expone en microvoltios)."""
    return round(_read_number(BATTERY_FILE) / 1_000_000.0, 2)


# ─── API ──────────────────────────────────────────────────────────────────────

class MoveCommand(BaseModel):
    motor: str
    speed: int


class StopCommand(BaseModel):
    motor: Optional[str] = None


@app.get("/status")
async def status():
    """Telemetría completa del robot: motores, sensores y batería."""
    return {
        "connected": True,
        "battery": read_battery(),
        "motors": read_motors(),
        "sensors": read_sensors(),
    }


@app.post("/move")
async def move(command: MoveCommand):
    """Mueve un motor. Acepta "A" o "outA" como identificador de puerto."""
    port = normalize_port(command.motor)
    motor = get_motor(port)
    if motor is None:
        return {"ok": False, "port": port, "error": "motor no conectado"}

    # ev3dev2 acepta velocidades de -100 a 100. Limitamos por seguridad.
    safe_speed = max(-100, min(100, command.speed))
    if safe_speed == 0:
        motor.off()
    else:
        motor.on(safe_speed)
    return {"ok": True, "port": port, "speed": safe_speed}


@app.post("/stop")
async def stop(command: StopCommand = StopCommand()):
    """Detiene un motor concreto, o todos si no se indica ninguno."""
    targets = [normalize_port(command.motor)] if command.motor else MOTOR_PORTS
    stopped = []
    for port in targets:
        motor = motors.get(port)
        if motor is not None:
            motor.off()
            stopped.append(port)
    return {"ok": True, "stopped": stopped}


@app.get("/")
async def command(cmd: str = "", idx: int = -1, speed: int = 0, pid: int = 0):
    """Interfaz legacy por query params que espera el backend en Rust."""
    if cmd == "motor":
        motor = get_motor_by_index(idx)
        if motor:
            safe_speed = max(-100, min(100, speed))
            motor.on(safe_speed)
        return {"ok": True, "cmd": cmd, "idx": idx, "speed": speed}

    elif cmd == "stop":
        motor = get_motor_by_index(idx)
        if motor:
            motor.off()
        return {"ok": True, "cmd": cmd, "idx": idx}

    elif cmd == "killpid":
        if pid > 0:
            try:
                # Envía la señal SIGKILL al proceso para detener la ejecución
                os.kill(pid, signal.SIGKILL)
            except OSError:
                # El proceso ya terminó o no existe
                pass
        return {"ok": True, "cmd": cmd, "pid": pid}

    return {"ok": False, "error": "comando desconocido"}


if __name__ == "__main__":
    # Ejecuta el servidor en el puerto 8080 que esperan el puente y el Rust
    uvicorn.run(app, host="0.0.0.0", port=8080)
