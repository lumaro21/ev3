use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use crate::connection::{SharedConn, ensure_connected, make_shared_conn};
use crate::state::{Ev3State, Motor, MotorCommand, Sensor, SensorType};

pub fn start_polling(
    state: Arc<Mutex<Ev3State>>,
    shared_conn: SharedConn,
    user: String,
    pass: String,
) {
    thread::spawn(move || {
        let mut current_ip = state.lock().unwrap().ip.clone();

        loop {
            // ¿Pidió reconexión con nueva IP?
            {
                let mut s = state.lock().unwrap();
                if s.reconnect_requested {
                    current_ip = s.ip.clone();
                    s.reconnect_requested = false;
                    s.connected = false;
                    // Forzar reconexión limpiando la sesión actual
                    *shared_conn.lock().unwrap() = None;
                }
            }

            // Asegurar conexión (reutiliza si ya está viva)
            if !ensure_connected(&shared_conn, &current_ip, &user, &pass) {
                if let Ok(mut s) = state.lock() {
                    s.connected = false;
                    s.motors    = vec![];
                    s.sensors   = vec![];
                    s.alerts    = vec![];
                }
                thread::sleep(Duration::from_millis(1500));
                continue;
            }

            // Obtener la conexión
            let conn_guard = shared_conn.lock().unwrap();
            let conn = match conn_guard.as_ref() {
                Some(c) => c,
                None    => { drop(conn_guard); continue; }
            };

            // Ejecutar comandos pendientes (motores) — reutilizando la sesión abierta
            let commands = {
                let mut s = state.lock().unwrap();
                std::mem::take(&mut s.pending_commands)
            };
            for cmd in &commands {
                apply_command(conn, cmd, &state);
            }

            // Leer motores
            let mut motors = vec![];
            let expected   = ["outA", "outB", "outC", "outD"];
            let found_raw  = conn.exec("ls /sys/class/tacho-motor/ 2>/dev/null").unwrap_or_default();
            let mut found_ports = std::collections::HashSet::new();

            for motor_id in found_raw.split_whitespace() {
                let base      = format!("/sys/class/tacho-motor/{}", motor_id);
                let port_raw  = conn.exec(&format!("cat {}/address", base)).unwrap_or_default();
                let port      = port_raw.split(':').last().unwrap_or(&port_raw).trim().to_string();
                let speed: i32 = conn.exec(&format!("cat {}/speed", base))
                    .unwrap_or_default().trim().parse().unwrap_or(0);
                found_ports.insert(port.clone());
                motors.push(Motor { port, motor_id: motor_id.to_string(), connected: true, speed });
            }
            for p in &expected {
                if !found_ports.contains(*p) {
                    motors.push(Motor { port: p.to_string(), motor_id: String::new(), connected: false, speed: 0 });
                }
            }

            let mut alerts = vec![];
            for m in &motors {
                if !m.connected {
                    alerts.push(format!("⚠ Motor {} desconectado", m.port));
                }
            }

            // Leer sensores
            let mut sensors  = vec![];
            let sensor_raw   = conn.exec("ls /sys/class/lego-sensor/ 2>/dev/null").unwrap_or_default();
            for sensor_id in sensor_raw.split_whitespace() {
                let base     = format!("/sys/class/lego-sensor/{}", sensor_id);
                let port_raw = conn.exec(&format!("cat {}/address", base)).unwrap_or_default();
                let port     = port_raw.split(':').last().unwrap_or(&port_raw).trim().to_string();
                let driver   = conn.exec(&format!("cat {}/driver_name", base)).unwrap_or_default();
                let value: f32 = conn.exec(&format!("cat {}/value0", base))
                    .unwrap_or_default().trim().parse().unwrap_or(0.0);
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

            // Batería
            let battery = conn
                .exec("cat /sys/class/power_supply/lego-ev3-battery/voltage_now")
                .unwrap_or_default().trim().parse::<f32>().unwrap_or(0.0) / 1_000_000.0;

            drop(conn_guard); // liberar el lock antes de actualizar el estado

            if let Ok(mut s) = state.lock() {
                s.connected       = true;
                s.motors          = motors;
                s.sensors         = sensors;
                s.battery_voltage = battery;
                s.alerts          = alerts;
            }

            thread::sleep(Duration::from_millis(300)); // polling más rápido: 300ms
        }
    });
}

fn apply_command(conn: &crate::connection::Ev3Connection, cmd: &MotorCommand, state: &Arc<Mutex<Ev3State>>) {
    let motor_id_for = |port: &str| -> Option<String> {
        state.lock().ok().and_then(|s| {
            s.motors.iter().find(|m| m.port == port && m.connected).map(|m| m.motor_id.clone())
        })
    };
    match cmd {
        MotorCommand::SetSpeed { port, speed } => {
            if let Some(mid) = motor_id_for(port) {
                let _ = conn.exec(&format!("echo {} > /sys/class/tacho-motor/{}/speed_sp", speed, mid));
            }
        }
        MotorCommand::Run { port } => {
            if let Some(mid) = motor_id_for(port) {
                let _ = conn.exec(&format!("echo run-forever > /sys/class/tacho-motor/{}/command", mid));
            }
        }
        MotorCommand::Stop { port } => {
            if let Some(mid) = motor_id_for(port) {
                let _ = conn.exec(&format!("echo stop > /sys/class/tacho-motor/{}/command", mid));
            }
        }
    }
}