import { useEffect, useRef, useState } from "react";
import { Terminal as XTerminal } from "xterm";
import { FitAddon } from "@xterm/addon-fit";
import "xterm/css/xterm.css";
import "./Terminal.css";
import { useRobot } from "../context/RobotContext";

export default function Terminal() {
  // El bridge FastAPI siempre corre en local; es él quien abre la sesión SSH
  // hacia el robot configurado (targetIp) en /api/config.
  const bridgeHost = "127.0.0.1";
  const bridgePort = 8000;
  const { targetIp, connectionMode } = useRobot();

  const [code, setCode] = useState("print('Hola desde el EV3')\n");
  const [filename, setFilename] = useState("programa.py");
  const [isConnected, setIsConnected] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const terminalRef = useRef(null);
  const socketRef = useRef(null);
  const termRef = useRef(null);
  const fitAddonRef = useRef(null);
  const fileInputRef = useRef(null);
  const targetIpRef = useRef(targetIp);
  targetIpRef.current = targetIp;

  const connectSocket = () => {
    if (socketRef.current) {
      socketRef.current.close();
    }

    const socket = new WebSocket(`ws://${bridgeHost}:${bridgePort}/ws/terminal`);
    socketRef.current = socket;

    socket.addEventListener("open", () => {
      setIsConnected(true);
      termRef.current?.write(`\r\nConectado al EV3 por SSH (${targetIpRef.current}).\r\n`);
    });

    socket.addEventListener("message", (event) => {
      const payload = typeof event.data === "string" ? event.data : new TextDecoder().decode(event.data);
      termRef.current?.write(payload);
    });

    socket.addEventListener("close", () => {
      setIsConnected(false);
      termRef.current?.write("\r\nSesión SSH cerrada.\r\n");
    });

    socket.addEventListener("error", () => {
      setIsConnected(false);
      termRef.current?.write("\r\nError de conexión WebSocket. Comprueba que FastAPI esté levantado en :8000.\r\n");
    });
  };

  useEffect(() => {
    const term = new XTerminal({
      cursorBlink: true,
      fontSize: 14,
      rows: 26,
      theme: {
        background: "#0f172a",
        foreground: "#e2e8f0",
        cursor: "#f8fafc",
      },
      scrollback: 2000,
      convertEol: true,
    });

    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(terminalRef.current);
    fit.fit();

    termRef.current = term;
    fitAddonRef.current = fit;

    term.onData((data) => {
      if (socketRef.current && socketRef.current.readyState === WebSocket.OPEN) {
        socketRef.current.send(data);
      }
    });

    connectSocket();

    const handleResize = () => fit.fit();
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      socketRef.current?.close();
      term.dispose();
    };
  }, []);

  const handleLoadFile = async (e) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;

    if (!/\.(py|sh|bash)$/i.test(file.name)) {
      termRef.current?.write(`\r\n[ERROR] Solo se admiten archivos .py, .sh o .bash\r\n`);
      return;
    }

    const text = await file.text();
    setCode(text);
    setFilename(file.name);
    termRef.current?.write(`\r\nArchivo local "${file.name}" cargado en el editor.\r\n`);
  };

  const handleRun = async () => {
    if (!socketRef.current || socketRef.current.readyState !== WebSocket.OPEN) {
      return;
    }

    setIsUploading(true);
    termRef.current?.write(`\r\nSubiendo ${filename} al EV3...\r\n`);

    try {
      const formData = new FormData();
      formData.append("file", new Blob([code], { type: "text/plain" }), filename);

      const response = await fetch(`http://${bridgeHost}:${bridgePort}/api/terminal/upload`, {
        method: "POST",
        body: formData,
      });
      const data = await response.json();

      if (!response.ok) {
        throw new Error(data.detail || "Fallo al subir el archivo");
      }

      const runner = /\.(sh|bash)$/i.test(filename) ? "bash" : "python3";
      socketRef.current.send(`${runner} ${data.path}\n`);
    } catch (err) {
      termRef.current?.write(`\r\n[ERROR] ${err.message}\r\n`);
    } finally {
      setIsUploading(false);
    }
  };

  return (
    <div className="terminal-layout">
      <div className="editor-panel">
        <div className="editor-toolbar">
          <input
            className="filename-input"
            value={filename}
            onChange={(e) => setFilename(e.target.value)}
            placeholder="nombre.py"
          />
          <input
            ref={fileInputRef}
            type="file"
            accept=".py,.sh,.bash"
            onChange={handleLoadFile}
            style={{ display: "none" }}
          />
          <button className="btn" onClick={() => fileInputRef.current?.click()}>
            📂 Cargar archivo
          </button>
          <button
            className="btn btn-success"
            onClick={handleRun}
            disabled={!isConnected || isUploading}
          >
            {!isConnected ? "Conectando..." : isUploading ? "Subiendo..." : "▶ Ejecutar en EV3"}
          </button>
          <button className="btn" onClick={connectSocket}>
            🔄 Reconectar
          </button>
        </div>

        <textarea
          className="code-editor"
          value={code}
          onChange={(e) => setCode(e.target.value)}
          spellCheck={false}
        />
      </div>

      <div className="console-panel">
        <div className="console-header">
          <span className={`console-dot ${isConnected ? "running" : ""}`} />
          Terminal SSH {isConnected ? "conectada" : "desconectada"} · {connectionMode} · {targetIp}
        </div>
        <div ref={terminalRef} className="console-output" />
      </div>
    </div>
  );
}