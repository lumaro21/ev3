import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import "./Port.css";

export default function MotorPort({ port, motor }) {
  const connected = motor?.connected ?? false;
  const [speed, setSpeed] = useState(0);

  function handleSlider(val) {
    setSpeed(val);
    invoke("set_motor_speed", { port, speed: parseInt(val) });
  }

  function handleRun() {
    const s = speed === 0 ? 50 : speed;
    setSpeed(s);
    invoke("set_motor_speed", { port, speed: s });
  }

  function handleStop() {
    setSpeed(0);
    invoke("stop_motor", { port });
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
