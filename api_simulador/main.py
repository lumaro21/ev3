import os
import signal
import uvicorn
from fastapi import FastAPI
from ev3dev2.motor import LargeMotor, OUTPUT_B, OUTPUT_D # type: ignore

app = FastAPI()

# Inicializamos los motores físicos. 
# Usamos un bloque try-except para que el servidor no falle al arrancar 
# si los estudiantes olvidan conectar algún cable en los puertos B o D.
try:
    motor_b = LargeMotor(OUTPUT_B)
except Exception:
    motor_b = None

try:
    motor_d = LargeMotor(OUTPUT_D)
except Exception:
    motor_d = None

def get_motor(idx: int):
    """Mapea el índice que envía Rust al motor físico correspondiente."""
    return motor_b if idx == 0 else motor_d

@app.get("/")
async def command(cmd: str = "", idx: int = -1, speed: int = 0, pid: int = 0):
    if cmd == "motor":
        motor = get_motor(idx)
        if motor:
            # ev3dev2 acepta velocidades de -100 a 100
            # Limitamos el valor por seguridad
            safe_speed = max(-100, min(100, speed))
            motor.on(safe_speed)
        return {"ok": True, "cmd": cmd, "idx": idx, "speed": speed}
        
    elif cmd == "stop":
        motor = get_motor(idx)
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
    # Ejecuta el servidor en el puerto 8080 que espera el Rust
    uvicorn.run(app, host="0.0.0.0", port=8080)