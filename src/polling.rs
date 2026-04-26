use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use crate::connection::Ev3Connection;
use crate::state::{Ev3State, Motor, MotorCommand, Sensor, SensorType};
 
pub fn start_polling(state: Arc<Mutex<Ev3State>>, ip: String, user: String, pass: String) {
    thread::spawn(move || {
        loop {
            match Ev3Connection::connect(&ip, &user, &pass) {
                Ok(conn) => {
                    // ── Ejecutar comandos pendientes ──────────────────────
                    let commands = {
                        let mut s = state.lock().unwrap();
                        std::mem::take(&mut s.pending_commands)
                    };
 
                    for cmd in commands {
                        apply_command(&conn, &cmd, &state);
                    }
 
                    // ── Leer motores ──────────────────────────────────────
                    let mut motors = vec![];
                    let expected = ["outA", "outB", "outC", "outD"];
                    let found_raw = conn
                        .exec("ls /sys/class/tacho-motor/ 2>/dev/null")
                        .unwrap_or_default();
                    let mut found_ports = std::collections::HashSet::new();
 
                    for motor_id in found_raw.split_whitespace() {
                        let base = format!("/sys/class/tacho-motor/{}", motor_id);
 
                        let port_raw = conn
                            .read_file(&format!("{}/address", base))
                            .unwrap_or_default();
                        // El EV3 devuelve "ev3-ports:outA" → quedamos con "outA"
                        let port = port_raw
                            .split(':')
                            .last()
                            .unwrap_or(&port_raw)
                            .trim()
                            .to_string();
 
                        let speed: i32 = conn
                            .read_file(&format!("{}/speed", base))
                            .unwrap_or_default()
                            .trim()
                            .parse()
                            .unwrap_or(0);
 
                        found_ports.insert(port.clone());
                        motors.push(Motor {
                            port,
                            motor_id: motor_id.to_string(),
                            connected: true,
                            speed,
                        });
                    }
 
                    for p in &expected {
                        if !found_ports.contains(*p) {
                            motors.push(Motor {
                                port: p.to_string(),
                                motor_id: String::new(),
                                connected: false,
                                speed: 0,
                            });
                        }
                    }
 
                    // ── Alertas motores desconectados ─────────────────────
                    let mut alerts = vec![];
                    for m in &motors {
                        if !m.connected {
                            alerts.push(format!("⚠ Motor {} desconectado", m.port));
                        }
                    }
 
                    // ── Leer sensores ─────────────────────────────────────
                    let mut sensors = vec![];
                    let sensor_raw = conn
                        .exec("ls /sys/class/lego-sensor/ 2>/dev/null")
                        .unwrap_or_default();
 
                    for sensor_id in sensor_raw.split_whitespace() {
                        let base = format!("/sys/class/lego-sensor/{}", sensor_id);
 
                        let port_raw = conn
                            .read_file(&format!("{}/address", base))
                            .unwrap_or_default();
                        let port = port_raw
                            .split(':')
                            .last()
                            .unwrap_or(&port_raw)
                            .trim()
                            .to_string();
 
                        let driver = conn
                            .read_file(&format!("{}/driver_name", base))
                            .unwrap_or_default();
                        let value: f32 = conn
                            .read_file(&format!("{}/value0", base))
                            .unwrap_or_default()
                            .trim()
                            .parse()
                            .unwrap_or(0.0);
 
                        let sensor_type = match driver.trim() {
                            "lego-ev3-touch" => SensorType::Touch,
                            "lego-ev3-color" => SensorType::Color,
                            "lego-ev3-us"    => SensorType::Ultrasonic,
                            "lego-ev3-gyro"  => SensorType::Gyro,
                            "lego-ev3-ir"    => SensorType::Infrared,
                            other            => SensorType::Unknown(other.to_string()),
                        };
 
                        sensors.push(Sensor { port, sensor_type, value });
                    }
 
                    // ── Batería ───────────────────────────────────────────
                    let battery = conn
                        .read_file("/sys/class/power_supply/lego-ev3-battery/voltage_now")
                        .unwrap_or_default()
                        .trim()
                        .parse::<f32>()
                        .unwrap_or(0.0)
                        / 1_000_000.0;
 
                    // ── Actualizar estado ─────────────────────────────────
                    if let Ok(mut s) = state.lock() {
                        s.connected = true;
                        s.motors = motors;
                        s.sensors = sensors;
                        s.battery_voltage = battery;
                        s.alerts = alerts;
                    }
                }
 
                Err(_) => {
                    if let Ok(mut s) = state.lock() {
                        s.connected = false;
                        s.motors = vec![];
                        s.sensors = vec![];
                        s.alerts = vec![];
                    }
                }
            }
 
            thread::sleep(Duration::from_millis(800));
        }
    });
}
 
/// Ejecuta un MotorCommand en el EV3 vía SSH.
fn apply_command(conn: &Ev3Connection, cmd: &MotorCommand, state: &Arc<Mutex<Ev3State>>) {
    // Resuelve el motor_id a partir del port guardado en el estado
    let motor_id_for = |port: &str| -> Option<String> {
        state
            .lock()
            .ok()
            .and_then(|s| {
                s.motors
                    .iter()
                    .find(|m| m.port == port && m.connected)
                    .map(|m| m.motor_id.clone())
            })
    };
 
    match cmd {
        MotorCommand::SetSpeed { port, speed } => {
            if let Some(mid) = motor_id_for(port) {
                let path = format!("/sys/class/tacho-motor/{}/speed_sp", mid);
                let _ = conn.exec(&format!("echo {} > {}", speed, path));
            }
        }
        MotorCommand::Run { port } => {
            if let Some(mid) = motor_id_for(port) {
                let path = format!("/sys/class/tacho-motor/{}/command", mid);
                let _ = conn.exec(&format!("echo run-forever > {}", path));
            }
        }
        MotorCommand::Stop { port } => {
            if let Some(mid) = motor_id_for(port) {
                let path = format!("/sys/class/tacho-motor/{}/command", mid);
                let _ = conn.exec(&format!("echo stop > {}", path));
            }
        }
    }
}