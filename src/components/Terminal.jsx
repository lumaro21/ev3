import { invoke } from "@tauri-apps/api/core";
import { useState, useRef, useEffect } from "react";
import "./Terminal.css";

export default function Terminal() {
  const [code, setCode]       = useState("# Escribe tu programa aqui\nprint('Hola desde el EV3!')\n");
  const [lang, setLang]       = useState("python");
  const [filename, setFilename] = useState("programa.py");
  const [dir, setDir]         = useState("/home/robot");
  const [output, setOutput]   = useState("");
  const [running, setRunning] = useState(false);
  const [pid, setPid]         = useState(null);
  const [bashInput, setBashInput] = useState("");
  const [cwd, setCwd]         = useState("/home/robot");
  const [history, setHistory] = useState([]);
  const [histIdx, setHistIdx] = useState(null);
  const [histBuffer, setHistBuffer] = useState("");
  const consoleRef            = useRef(null);
  const inputRef              = useRef(null);

  useEffect(() => {
    if (consoleRef.current) {
      consoleRef.current.scrollTop = consoleRef.current.scrollHeight;
    }
  }, [output]);

  // Polling de output cuando hay programa corriendo
  useEffect(() => {
    if (!running || !pid) return;
    const interval = setInterval(async () => {
      const out = await invoke("get_program_output");
      setOutput(`PID: ${pid}\n${out}`);
    }, 300);
    return () => clearInterval(interval);
  }, [running, pid]);

  function changeLang(l) {
    setLang(l);
    const base = filename.split(".")[0];
    setFilename(base + (l === "python" ? ".py" : ".sh"));
  }

  async function handleRun() {
    if (running) return;
    setRunning(true);
    setOutput("Subiendo archivo...\n");
    const result = await invoke("run_code", { code, filename, dir, lang });
    const pidNum = parseInt(result.trim());
    if (!isNaN(pidNum)) {
      setPid(pidNum);
      setOutput(`PID: ${pidNum}\nEjecutando...\n`);
    } else {
      setOutput(result);
      setRunning(false);
    }
  }

  async function handleStop() {
    if (pid) {
      await invoke("kill_program", { pid });
      setOutput(prev => prev + "\nDetenido por el usuario\n");
    }
    setRunning(false);
    setPid(null);
  }

  async function handleSave() {
    const result = await invoke("save_file", { content: code, filename, dir });
    setOutput(result);
  }

  async function handleBashSubmit() {
    const cmd = bashInput.trim();
    if (!cmd) return;

    // Historial
    setHistory(prev => {
      const h = [...prev];
      if (h[h.length - 1] !== cmd) h.push(cmd);
      if (h.length > 100) h.shift();
      return h;
    });
    setHistIdx(null);
    setBashInput("");

    // cd especial
    if (cmd.startsWith("cd")) {
      const target = cmd.replace("cd", "").trim() || "/home/robot";
      const newDir = target.startsWith("/") ? target : `${cwd}/${target}`;
      const resolved = await invoke("run_bash", { cmd: `cd ${newDir} && pwd`, cwd });
      const clean = resolved.trim();
      if (!clean.startsWith("bash:") && !clean.includes("No such")) {
        setCwd(clean);
        setOutput(prev => prev + `\n${cwd}$ ${cmd}\n${clean}\n`);
      } else {
        setOutput(prev => prev + `\n${cwd}$ ${cmd}\n${clean}\n`);
      }
      return;
    }

    setOutput(prev => prev + `\n${cwd}$ ${cmd}\n`);
    const result = await invoke("run_bash", { cmd, cwd });
    setOutput(prev => prev + (result || "(sin salida)") + "\n");
  }

  function handleKeyDown(e) {
    if (e.key === "Enter") { handleBashSubmit(); return; }

    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (history.length === 0) return;
      if (histIdx === null) {
        setHistBuffer(bashInput);
        setHistIdx(history.length - 1);
        setBashInput(history[history.length - 1]);
      } else if (histIdx > 0) {
        setHistIdx(histIdx - 1);
        setBashInput(history[histIdx - 1]);
      }
    }

    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (histIdx === null) return;
      if (histIdx + 1 >= history.length) {
        setHistIdx(null);
        setBashInput(histBuffer);
      } else {
        setHistIdx(histIdx + 1);
        setBashInput(history[histIdx + 1]);
      }
    }
  }

  return (
    <div className="terminal-layout">

      {/* Panel izquierdo: editor */}
      <div className="editor-panel">
        <div className="editor-toolbar">
          <div className="lang-tabs">
            {["python", "bash"].map(l => (
              <button
                key={l}
                className={`lang-tab ${lang === l ? "active" : ""}`}
                onClick={() => changeLang(l)}
              >
                {l === "python" ? "Python" : "Bash"}
              </button>
            ))}
          </div>
          <input
            className="filename-input"
            value={filename}
            onChange={e => setFilename(e.target.value)}
            placeholder="nombre.py"
          />
          <input
            className="filename-input"
            value={dir}
            onChange={e => setDir(e.target.value)}
            placeholder="/home/robot"
            style={{ width: 140 }}
          />
        </div>

        <textarea
          className="code-editor"
          value={code}
          onChange={e => setCode(e.target.value)}
          spellCheck={false}
        />

        <div className="editor-actions">
          <button className="btn btn-success" onClick={handleRun} disabled={running}>
            {running ? "Ejecutando..." : "▶ Run"}
          </button>
          <button className="btn btn-danger" onClick={handleStop} disabled={!pid}>
            ■ Stop
          </button>
          <button className="btn" onClick={handleSave}>💾 Guardar</button>
          <button className="btn" onClick={() => setOutput("")}>Limpiar</button>
        </div>
      </div>

      {/* Panel derecho: consola */}
      <div className="console-panel">
        <div className="console-header">
          <span className={`console-dot ${running ? "running" : ""}`} />
          Consola {running && <span className="console-running-label">— ejecutando...</span>}
        </div>

        <div className="console-output" ref={consoleRef}>
          <pre>{output}</pre>
        </div>

        <div className="bash-input-area">
          <span className="bash-prompt">
            {cwd.replace("/home/robot", "~")}$
          </span>
          <input
            ref={inputRef}
            className="bash-input"
            value={bashInput}
            onChange={e => setBashInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="comando bash... (↑↓ historial)"
            spellCheck={false}
          />
          <button className="btn" onClick={handleBashSubmit}>↵</button>
        </div>
      </div>
    </div>
  );
}