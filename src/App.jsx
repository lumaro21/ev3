import { useState } from "react";
// 1. Importamos el contexto en lugar de Tauri
import { useRobot } from "./context/RobotContext";
import Dashboard from "./components/Dashboard";
import Terminal from "./components/Terminal";
import Ev3Twin from "./components/Ev3Twin";
import "./App.css";

function App() {
  const [view, setView] = useState("dashboard");
  // 2. Extraemos la telemetría de nuestro "cerebro falso"
  const { telemetry } = useRobot();

  // 3. Adaptamos los datos simulados al formato que tu aplicación original espera (arrays)
  const status = {
    connected: true, // Simulado como siempre conectado
    ip: "192.168.1.100", // IP simulada
    battery: telemetry.battery,
    alerts: [],
    motors: ["A", "B", "C", "D"].map(p => ({
      port: `out${p}`,
      connected: true, // Simulamos que todos están conectados
      speed: telemetry.motors[p]?.speed || 0,
      position: telemetry.motors[p]?.position || 0
    })),
    sensors: ["1", "2", "3", "4"].map(p => ({
      port: `in${p}`,
      connected: telemetry.sensors[p]?.type !== "none",
      type: telemetry.sensors[p]?.type || "none",
      value: telemetry.sensors[p]?.value || 0
    }))
  };

  return (
    <div className="app">
      <nav className="navbar">
        <div className="nav-brand">
          <span className="nav-dot" style={{ background: status.connected ? "#4ec94e" : "#e05252" }} />
          EV3 Controller
        </div>
        <div className="nav-tabs">
          {[
            { id: "dashboard", label: "Dashboard" },
            { id: "twin",      label: "Gemelo 3D" },
            { id: "terminal",  label: "Terminal"  },
          ].map(tab => (
            <button
              key={tab.id}
              className={`nav-tab ${view === tab.id ? "active" : ""}`}
              onClick={() => setView(tab.id)}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <div className="nav-info">
          <span className="nav-battery">🔋 {status.battery.toFixed(2)}V</span>
          <span className="nav-ip">{status.ip}</span>
        </div>
      </nav>

      <main className="main-content">
        {view === "dashboard" && <Dashboard status={status} />}
        {view === "twin"      && <Ev3Twin   status={status} />}
        {view === "terminal"  && <Terminal  status={status} />}
      </main>
    </div>
  );
}

export default App;