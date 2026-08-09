"""EV3 simulado: permite probar la interfaz sin el ladrillo físico.

No reimplementa el robot. Ejecuta el servidor real de `api_simulador/main.py`
sobre un `ev3dev2` stubeado y un árbol sysfs falso en un directorio temporal,
de modo que los contratos de la API son exactamente los del hardware.

El lazo se cierra de verdad: un comando de motor escribe el `speed` en el sysfs
falso y la telemetría lo devuelve en el siguiente ciclo de polling.

    React (:1420) -> ev3_bridge (:8000) -> este simulador (:8080) -> sysfs falso
                  <-------------------- telemetría --------------------

Uso:
    python tools/fake_ev3.py

Hardware simulado (elegido para cubrir los dos estados de cada tarjeta):
    motores  outA, outB, outD conectados   ·  outC ausente -> "vacío" + alerta
    sensores in1 ultrasónico, in4 táctil   ·  in2, in3 ausentes -> "vacío"
    batería  7.85 V
"""
import os
import sys
import tempfile
import types

# Raíz del repo: este archivo vive en <repo>/tools/
REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.join(tempfile.gettempdir(), "fake_ev3_sys")

# Puertos con hardware "conectado". outC se deja fuera a propósito para poder
# ver el estado "vacío" y la alerta de motor desconectado.
MOTOR_NODES = {"outA": "motor0", "outB": "motor1", "outD": "motor2"}

# rpm por cada 1% de consigna, para que la telemetría se parezca a la real
RPM_POR_PORCIENTO = 10


def write(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(str(content))


def set_speed_in_sysfs(port, speed):
    node = MOTOR_NODES.get(port)
    if node:
        write("{}/tacho-motor/{}/speed".format(ROOT, node), int(speed))


# ─── Stub de ev3dev2.motor ────────────────────────────────────────────────────
# La librería solo existe dentro del ladrillo; la sustituimos antes de importar
# api_simulador/main.py para que ese módulo se cargue sin cambios.

ev3dev2 = types.ModuleType("ev3dev2")
motor_mod = types.ModuleType("ev3dev2.motor")


class LargeMotor:
    def __init__(self, port):
        if port not in MOTOR_NODES:
            # Mismo comportamiento que ev3dev2 con un puerto sin motor
            raise Exception("no hay motor en {}".format(port))
        self.port = port

    def on(self, speed):
        set_speed_in_sysfs(self.port, int(speed) * RPM_POR_PORCIENTO)
        print("[motor {}] on({}%)".format(self.port, speed), flush=True)

    def off(self):
        set_speed_in_sysfs(self.port, 0)
        print("[motor {}] off()".format(self.port), flush=True)


motor_mod.LargeMotor = LargeMotor
for _p in ("A", "B", "C", "D"):
    setattr(motor_mod, "OUTPUT_" + _p, "out" + _p)

sys.modules["ev3dev2"] = ev3dev2
sys.modules["ev3dev2.motor"] = motor_mod

# ─── Árbol sysfs falso ────────────────────────────────────────────────────────
# Reproduce la estructura que main.py lee de /sys en el robot real.

for _port, _node in MOTOR_NODES.items():
    write("{}/tacho-motor/{}/address".format(ROOT, _node), "ev3-ports:" + _port)
    write("{}/tacho-motor/{}/speed".format(ROOT, _node), 0)

write("{}/lego-sensor/sensor0/address".format(ROOT), "ev3-ports:in1")
write("{}/lego-sensor/sensor0/driver_name".format(ROOT), "lego-ev3-us")
write("{}/lego-sensor/sensor0/value0".format(ROOT), 42)      # 42 cm
write("{}/lego-sensor/sensor1/address".format(ROOT), "ev3-ports:in4")
write("{}/lego-sensor/sensor1/driver_name".format(ROOT), "lego-ev3-touch")
write("{}/lego-sensor/sensor1/value0".format(ROOT), 0)       # suelto
write("{}/battery/voltage_now".format(ROOT), 7_850_000)      # microvoltios

# ─── Arranque del servidor real del robot ─────────────────────────────────────

sys.path.insert(0, os.path.join(REPO, "api_simulador"))
import main  # noqa: E402  (requiere el stub de ev3dev2 ya instalado)

main.TACHO_DIR = ROOT + "/tacho-motor"
main.SENSOR_DIR = ROOT + "/lego-sensor"
main.BATTERY_FILE = ROOT + "/battery/voltage_now"

if __name__ == "__main__":
    import uvicorn

    print("EV3 simulado escuchando en http://127.0.0.1:8080")
    print("sysfs falso en {}".format(ROOT))
    uvicorn.run(main.app, host="127.0.0.1", port=8080, log_level="warning")
