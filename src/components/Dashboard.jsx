import { useState } from "react";
import MotorPort from "./MotorPort";
import SensorPort from "./SensorPort";
import "./Dashboard.css";
import KeyboardControl from "./KeyboardControl";

export default function Dashboard({ status }) {
  const [editingIp, setEditingIp] = useState(false);
  const [ipInput, setIpInput] = useState("");

  const ports   = ["outA", "outB", "outC", "outD"];
  const sensors = ["in1",  "in2",  "in3",  "in4"];

  // La función ya no llama a Tauri, solo cierra el input visualmente
  function applyIp() {
    if (ipInput.trim()) {
      console.log("[Mock Red] IP simulada cambiada a:", ipInput.trim());
      // Aquí en el futuro enviaríamos la IP al contexto si fuera necesario
    }
    setEditingIp(false);
  }

  // Simulamos la reconexión
  function handleReconnect() {
    console.log("[Mock Red] Solicitando reconexión al orquestador...");
  }

  return (
    <div className="dashboard">

      {/* Status bar */}
      <div className="status-bar">
        <div className="status-left">
          <span className={`status-dot ${status?.connected ? "online" : "offline"}`} />
          <span>{status?.connected ? "Conectado" : "Sin conexión"}</span>
          {editingIp ? (
            <div className="ip-edit">
              <input
                autoFocus
                value={ipInput}
                onChange={e => setIpInput(e.target.value)}
                onKeyDown={e => { if (e.key === "Enter") applyIp(); if (e.key === "Escape") setEditingIp(false); }}
                className="ip-input"
                placeholder="192.168.x.x"
              />
              <button className="btn btn-primary" onClick={applyIp}>OK</button>
              <button className="btn" onClick={() => setEditingIp(false)}>✕</button>
            </div>
          ) : (
            <span className="ip-display" onClick={() => { setIpInput(status?.ip || ""); setEditingIp(true); }}>
              {status?.ip} <span className="ip-edit-hint">✎</span>
            </span>
          )}
        </div>
        <div className="status-right">
          <span className="battery">🔋 {status?.battery?.toFixed(2) ?? "—"}V</span>
          {/* Botón desconectado de Tauri */}
          <button className="btn" onClick={handleReconnect}>Reconectar</button>
        </div>
      </div>

      {/* Motores */}
      <section className="section">
        <h2 className="section-title">OUTPUT — Motores</h2>
        <div className="ports-grid">
          {ports.map(port => {
            const motor = status?.motors?.find(m => m.port === port);
            return <MotorPort key={port} port={port} motor={motor} />;
          })}
        </div>
      </section>

      {/* Sensores */}
      <section className="section">
        <h2 className="section-title">INPUT — Sensores</h2>
        <div className="ports-grid">
          {sensors.map(port => {
            const sensor = status?.sensors?.find(s => s.port === port);
            return <SensorPort key={port} port={port} sensor={sensor} />;
          })}
        </div>
      </section>

      {/* Control por teclado */}
      <KeyboardControl status={status} />

      {/* Alertas */}
      {status?.alerts?.length > 0 && (
        <div className="alerts">
          {status.alerts.map((a, i) => (
            <div key={i} className="alert">⚠ {a}</div>
          ))}
        </div>
      )}
    </div>
  );
}