use tauri::State;
use std::sync::{Arc, Mutex};
use crate::state::{Ev3State, MotorCommand};
use crate::connection::SharedConn;
use serde::Serialize;

// ─── Tipos que se envían al frontend ─────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct MotorInfo {
    pub port: String,
    pub connected: bool,
    pub speed: i32,
}

#[derive(Serialize, Clone)]
pub struct SensorInfo {
    pub port: String,
    pub sensor_type: String,
    pub value: f32,
}

#[derive(Serialize, Clone)]
pub struct Ev3Status {
    pub connected: bool,
    pub ip: String,
    pub battery: f32,
    pub motors: Vec<MotorInfo>,
    pub sensors: Vec<SensorInfo>,
    pub alerts: Vec<String>,
}

// ─── Estado compartido para Tauri ────────────────────────────────────────────

pub struct AppState {
    pub ev3:  Arc<Mutex<Ev3State>>,
    pub conn: SharedConn,
}

// ─── Comandos disponibles para React ─────────────────────────────────────────

/// Devuelve el estado completo del EV3 al frontend
#[tauri::command]
pub fn get_status(state: State<AppState>) -> Ev3Status {
    let s = state.ev3.lock().unwrap();
    Ev3Status {
        connected: s.connected,
        ip:        s.ip.clone(),
        battery:   s.battery_voltage,
        motors: s.motors.iter().map(|m| MotorInfo {
            port:      m.port.clone(),
            connected: m.connected,
            speed:     m.speed,
        }).collect(),
        sensors: s.sensors.iter().map(|sen| SensorInfo {
            port:        sen.port.clone(),
            sensor_type: format!("{:?}", sen.sensor_type),
            value:       sen.value,
        }).collect(),
        alerts: s.alerts.clone(),
    }
}

/// Cambia la velocidad de un motor por HTTP
#[tauri::command]
pub fn set_motor_speed(port: String, speed: i32, state: State<AppState>) {
    let mut s = state.ev3.lock().unwrap();
    s.desired_speeds.insert(port.clone(), speed);
    s.pending_commands.push(MotorCommand::HttpSetSpeed { port, speed });
}

/// Para un motor específico por HTTP
#[tauri::command]
pub fn stop_motor(port: String, state: State<AppState>) {
    let mut s = state.ev3.lock().unwrap();
    s.pending_commands.push(MotorCommand::HttpStop { port });
}

/// Para todos los motores por HTTP
#[tauri::command]
pub fn stop_all_motors(state: State<AppState>) {
    let mut s = state.ev3.lock().unwrap();
    for port in ["outA", "outB", "outC", "outD"] {
        s.pending_commands.push(MotorCommand::HttpStop { port: port.to_string() });
    }
}

/// Cambia la IP del EV3 y fuerza reconexión
#[tauri::command]
pub fn set_ip(ip: String, state: State<AppState>) {
    {
        let mut s = state.ev3.lock().unwrap();
        s.ip = ip;
        s.reconnect_requested = true;
        s.connected = false;
    }
    *state.conn.lock().unwrap() = None;
}

/// Fuerza reconexión SSH sin cambiar IP
#[tauri::command]
pub fn reconnect(state: State<AppState>) {
    {
        let mut s = state.ev3.lock().unwrap();
        s.reconnect_requested = true;
        s.connected = false;
    }
    *state.conn.lock().unwrap() = None;
}

/// Ejecuta un comando bash en el EV3 y devuelve la salida.
/// Toma el lock, ejecuta y lo suelta inmediatamente.
#[tauri::command]
pub fn run_bash(cmd: String, cwd: String, state: State<AppState>) -> String {
    let full_cmd = format!("cd {} 2>/dev/null; {}", cwd, cmd);
    let guard = state.conn.lock().unwrap();
    match guard.as_ref() {
        Some(conn) => conn.exec(&full_cmd).unwrap_or_else(|e| format!("Error: {}", e)),
        None       => "Sin conexion SSH".to_string(),
    }
    // lock se libera aquí automáticamente
}

/// Sube el archivo al EV3 y lo ejecuta en background.
/// Cada operación toma y suelta el lock por separado para no bloquearse.
#[tauri::command]
pub fn run_code(
    code:     String,
    filename: String,
    dir:      String,
    lang:     String,
    state:    State<AppState>,
) -> String {
    let remote_path = format!("{}/{}", dir.trim_end_matches('/'), filename);

    // 1. Subir archivo — lock se toma y suelta
    {
        let guard = state.conn.lock().unwrap();
        match guard.as_ref() {
            Some(conn) => {
                if let Err(e) = conn.write_file(&remote_path, &code) {
                    return format!("Error al subir: {}", e);
                }
            }
            None => return "Sin conexion SSH".to_string(),
        }
    }

    // 2. Limpiar log anterior — lock se toma y suelta
    {
        let guard = state.conn.lock().unwrap();
        if let Some(conn) = guard.as_ref() {
            let _ = conn.exec("echo '' > /tmp/ev3_out.log");
        }
    }

    // 3. Ejecutar en background y capturar PID — lock se toma y suelta
    let run_cmd = if lang == "python" {
        format!("nohup python3 {} > /tmp/ev3_out.log 2>&1 & echo $!", remote_path)
    } else {
        format!("nohup bash {} > /tmp/ev3_out.log 2>&1 & echo $!", remote_path)
    };

    let pid_str = {
        let guard = state.conn.lock().unwrap();
        match guard.as_ref() {
            Some(conn) => conn.exec(&run_cmd).unwrap_or_default(),
            None       => return "Sin conexion".to_string(),
        }
    };

    pid_str.trim().to_string()
}

/// Lee el log de salida del programa en ejecución.
/// Lock se toma y suelta rápidamente.
#[tauri::command]
pub fn get_program_output(state: State<AppState>) -> String {
    let guard = state.conn.lock().unwrap();
    match guard.as_ref() {
        Some(conn) => conn.exec("cat /tmp/ev3_out.log 2>/dev/null").unwrap_or_default(),
        None       => String::new(),
    }
    // lock se libera aquí automáticamente
}

/// Mata el proceso por PID usando HTTP para no bloquear el lock SSH.
#[tauri::command]
pub fn kill_program(pid: u32, state: State<AppState>) -> String {
    let ip  = state.ev3.lock().unwrap().ip.clone();
    let url = format!("http://{}:8080/?cmd=killpid&pid={}", ip, pid);

    // Fire-and-forget por HTTP — nunca bloquea
    std::thread::spawn(move || {
        let _ = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build()
            .unwrap()
            .get(&url)
            .send();
    });

    "Detenido".to_string()
}

/// Sube un archivo al EV3 sin ejecutarlo.
#[tauri::command]
pub fn save_file(
    content:  String,
    filename: String,
    dir:      String,
    state:    State<AppState>,
) -> String {
    let remote_path = format!("{}/{}", dir.trim_end_matches('/'), filename);
    let guard = state.conn.lock().unwrap();
    match guard.as_ref() {
        Some(conn) => match conn.write_file(&remote_path, &content) {
            Ok(_)  => format!("Guardado en {}", remote_path),
            Err(e) => format!("Error: {}", e),
        },
        None => "Sin conexion".to_string(),
    }
}

/// Verifica si un proceso sigue corriendo en el EV3.
#[tauri::command]
pub fn check_pid(pid: u32, state: State<AppState>) -> bool {
    let guard = state.conn.lock().unwrap();
    match guard.as_ref() {
        Some(conn) => {
            let result = conn
                .exec(&format!("kill -0 {} 2>/dev/null && echo yes || echo no", pid))
                .unwrap_or_else(|_| "no".to_string());
            result.trim() == "yes"
        }
        None => false,
    }
}