import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import Dashboard from "./components/Dashboard";
import Terminal from "./components/Terminal";
import Ev3Twin from "./components/Ev3Twin";
import "./App.css";

function App() {
  const [status, setStatus] = useState(null);
  const [view, setView] = useState("dashboard");

  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const s = await invoke("get_status");
        setStatus(s);
      } catch (e) {
        console.error(e);
      }
    }, 400);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="app">
      <nav className="navbar">
        <div className="nav-brand">
          <span className="nav-dot" style={{ background: status?.connected ? "#4ec94e" : "#e05252" }} />
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
          {status && (
            <>
              <span className="nav-battery">🔋 {status.battery.toFixed(2)}V</span>
              <span className="nav-ip">{status.ip}</span>
            </>
          )}
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