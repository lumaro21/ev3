import { useState, useEffect } from "react";
// 1. Importamos el contexto
import { useRobot } from "../context/RobotContext";
import "./Port.css";

export default function MotorPort({ port, motor }) {
  // 2. Traemos la función de nuestro cerebro simulado
  const { sendMotorCommand } = useRobot(); 
  
  const connected = motor?.connected ?? false;

  // 3. El slider es la CONSIGNA en % (-100..100). No se puede inicializar ni
  //    sincronizar con motor.speed, que es la medida real en rpm: meter 700 rpm
  //    en un control de -100..100 lo satura y muestra "+700%".
  const [speed, setSpeed] = useState(0);

  // 4. Al desconectarse el motor la consigna deja de ser válida
  useEffect(() => {
    if (!connected) setSpeed(0);
  }, [connected]);

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