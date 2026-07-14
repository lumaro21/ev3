from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn

app = FastAPI(title="EV3 Controller Bridge")

# Configuración de CORS: Permite que tu web en React (puerto 1420) le hable a Python
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"], # En producción deberías poner ["http://localhost:1420"]
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# --- MODELOS DE DATOS ---
class MotorCommand(BaseModel):
    port: str
    speed: int

# Estado simulado inicial del robot (Telemetría)
robot_state = {
    "connected": True,
    "ip": "192.168.1.100", # IP simulada
    "battery": 8.2,
    "alerts": [],
    "motors": [
        {"port": "outA", "connected": True, "speed": 0, "position": 0},
        {"port": "outB", "connected": True, "speed": 0, "position": 0},
        {"port": "outC", "connected": True, "speed": 0, "position": 0},
        {"port": "outD", "connected": True, "speed": 0, "position": 0}
    ],
    "sensors": [
        {"port": "in1", "sensor_type": "Touch", "value": 0},
        {"port": "in2", "sensor_type": "Ultrasonic", "value": 15.2},
        {"port": "in3", "sensor_type": "none", "value": 0},
        {"port": "in4", "sensor_type": "none", "value": 0}
    ]
}

# --- ENDPOINTS ---

@app.get("/api/status")
async def get_status():
    """Devuelve la telemetría actual del EV3."""
    # Aquí en el futuro leeremos los datos reales por SSH
    return robot_state

@app.post("/api/motor")
async def set_motor(command: MotorCommand):
    """Recibe un comando para mover un motor."""
    # Actualizamos nuestro estado simulado
    for motor in robot_state["motors"]:
        if motor["port"] == command.port:
            motor["speed"] = command.speed
            break
            
    print(f"📡 Comando recibido desde la web -> Motor: {command.port} | Velocidad: {command.speed}")
    
    # Aquí en el futuro enviaremos el comando SSH al EV3 real
    
    return {"status": "success", "message": f"Motor {command.port} set to {command.speed}"}

@app.post("/api/stop_all")
async def stop_all():
    """Freno de emergencia para todos los motores."""
    for motor in robot_state["motors"]:
        motor["speed"] = 0
    print("🛑 Freno de emergencia activado.")
    return {"status": "success"}

# --- INICIO DEL SERVIDOR ---
if __name__ == "__main__":
    print("🚀 Iniciando EV3 Bridge en el puerto 8000...")
    uvicorn.run(app, host="0.0.0.0", port=8000)