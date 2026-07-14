import { useState, useEffect } from "react";
// 1. Importamos el contexto
import { useRobot } from "../context/RobotContext";
import "./Port.css";

export default function MotorPort({ port, motor }) {
  // 2. Traemos la función de nuestro cerebro simulado
  const { sendMotorCommand } = useRobot(); 
  
  const connected = motor?.connected ?? false;
  
  // 3. Inicializamos con la velocidad del motor si existe, o 0
  const [speed, setSpeed] = useState(motor?.speed || 0);

  // 4. Sincronizamos el slider si la velocidad cambia mientras estamos en otra pestaña
  useEffect(() => {
    if (motor?.speed !== undefined) {
      setSpeed(motor.speed);
    }
  }, [motor?.speed]);

  function handleSlider(val) {
    setSpeed(val);
    // Reemplazamos invoke por la función del contexto
    sendMotorCommand(port, parseInt(val));
  }

  function handleRun() {
    const s = speed === 0 ? 50 : speed;
    setSpeed(s);
    // Reemplazamos invoke por la función del contexto
    sendMotorCommand(port, s);
  }

  function handleStop() {
    setSpeed(0);
    // Reemplazamos invoke por la función del contexto enviando velocidad 0
    sendMotorCommand(port, 0);
  }

  const label = port.replace("out", "");

  return (
    <div className={`port-card ${connected ? "connected" : ""}`}>
      <div className="port-label motor-label">{label}</div>
      {connected ? (
        <>
          <div className="port-value">{motor.speed} rpm</div>
          <input
            type="range"
            min="-100" max="100"
            value={speed}
            onChange={e => handleSlider(e.target.value)}
            className="speed-slider"
          />
          <div className="speed-display">{speed > 0 ? "+" : ""}{speed}%</div>
          <div className="port-actions">
            <button className="btn btn-success" onClick={handleRun}>▶</button>
            <button className="btn btn-danger"  onClick={handleStop}>■</button>
          </div>
        </>
      ) : (
        <div className="port-empty">vacío</div>
      )}
    </div>
  );
}