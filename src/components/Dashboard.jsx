import { useState, useEffect } from "react";
import MotorPort from "./MotorPort";
import SensorPort from "./SensorPort";
import "./Dashboard.css";
import KeyboardControl from "./KeyboardControl";
import { useRobot } from "../context/RobotContext";

export default function Dashboard({ status }) {
  // Traemos las funciones y variables globales del contexto
  const { connectionMode, targetIp, updateConnectionConfig } = useRobot();

  const [isEditingConfig, setIsEditingConfig] = useState(false);
  const [draftMode, setDraftMode] = useState(connectionMode);
  const [draftIp, setDraftIp] = useState(targetIp);

  // Mantenemos sincronizados los estados locales por si cambian externamente
  useEffect(() => {
    setDraftMode(connectionMode);
    setDraftIp(targetIp);
  }, [connectionMode, targetIp]);

  const ports   = ["outA", "outB", "outC", "outD"];
  const sensors = ["in1",  "in2",  "in3",  "in4"];

  // Guarda la configuración y la envía al puente FastAPI
  function applyConfig() {
    updateConnectionConfig(draftMode, draftIp);
    setIsEditingConfig(false);
  }

  // Usamos el botón de reconectar para forzar la re-aplicación de la red actual
  function handleReconnect() {
    console.log("[Red] Forzando reconexión...");
    updateConnectionConfig(connectionMode, targetIp);
  }

  return (
    <div className="dashboard">

      {/* Status bar */}
      <div className="status-bar">
        <div className="status-left">
          <span className={`status-dot ${status?.connected ? "online" : "offline"}`} />
          <span>{status?.connected ? "Conectado" : "Sin conexión"}</span>
          
          {isEditingConfig ? (
            <div className="ip-edit" style={{ display: 'flex', gap: '8px', alignItems: 'center', marginLeft: '10px' }}>
              
              {/* Selector de Modo */}
              <select 
                value={draftMode} 
                onChange={(e) => setDraftMode(e.target.value)}
                className="ip-input"
                style={{ padding: '4px', cursor: 'pointer' }}
              >
                <option value="simulated">Simulador Local</option>
                <option value="real">Hardware Real (EV3)</option>
              </select>

              {/* Input de IP (Solo visible si el modo es "real") */}
              {draftMode === 'real' && (
                <input
                  autoFocus
                  value={draftIp}
                  onChange={e => setDraftIp(e.target.value)}
                  onKeyDown={e => { if (e.key === "Enter") applyConfig(); if (e.key === "Escape") setIsEditingConfig(false); }}
                  className="ip-input"
                  placeholder="192.168.x.x"
                  style={{ width: '130px' }}
                />
              )}
              
              <button className="btn btn-primary" onClick={applyConfig}>OK</button>
              <button className="btn" onClick={() => setIsEditingConfig(false)}>✕</button>
            </div>
          ) : (
            <span className="ip-display" onClick={() => { setIsEditingConfig(true); }} style={{ marginLeft: '10px' }}>
              — {connectionMode === 'simulated' ? 'Simulador Local' : `Hardware (${targetIp})`} 
              <span className="ip-edit-hint">✎</span>
            </span>
          )}
        </div>
        
        <div className="status-right">
          <span className="battery">🔋 {status?.battery?.toFixed(2) ?? "—"}V</span>
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