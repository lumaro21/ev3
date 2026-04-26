use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use crate::connection::Ev3Connection;
use crate::state::{Ev3State, Motor, Sensor, SensorType};

pub fn start_polling(state: Arc<Mutex<Ev3State>>, ip: String, user: String, pass: String) {
    thread::spawn(move || {
        loop {
            match Ev3Connection::connect(&ip, &user, &pass) {
                Ok(conn) => {
                    // Motores
                    let mut motors = vec![];
                    let expected = ["outA", "outB", "outC", "outD"];
                    let found_raw = conn.exec("ls /sys/class/tacho-motor/ 2>/dev/null").unwrap_or_default();
                    let mut found_ports = std::collections::HashSet::new();

                    for motor_id in found_raw.split_whitespace() {
                        let base = format!("/sys/class/tacho-motor/{}", motor_id);
                        let port_raw = conn.read_file(&format!("{}/address", base)).unwrap_or_default();
// El EV3 devuelve "ev3-ports:outA" — nos quedamos solo con la parte después de ":"
                        let port = port_raw.split(':').last().unwrap_or(&port_raw).to_string();
                        let speed = conn.read_file(&format!("{}/speed", base)).unwrap_or("0".into()).parse().unwrap_or(0);
                        found_ports.insert(port.clone());
                        motors.push(Motor { port, connected: true, speed });
                    }

                    for p in &expected {
                        if !found_ports.contains(*p) {
                            motors.push(Motor { port: p.to_string(), connected: false, speed: 0 });
                        }
                    }

                    // Alertas motores desconectados
                    let mut alerts = vec![];
                    for m in &motors {
                        if !m.connected {
                            alerts.push(format!("⚠ Motor {} desconectado", m.port));
                        }
                    }

                    // Sensores
                    let mut sensors = vec![];
                    let sensor_raw = conn.exec("ls /sys/class/lego-sensor/ 2>/dev/null").unwrap_or_default();

                    for sensor_id in sensor_raw.split_whitespace() {
                        let base = format!("/sys/class/lego-sensor/{}", sensor_id);
                        let port_raw = conn.read_file(&format!("{}/address", base)).unwrap_or_default();
                        let port = port_raw.split(':').last().unwrap_or(&port_raw).to_string();
                        let driver = conn.read_file(&format!("{}/driver_name", base)).unwrap_or_default();
                        let value  = conn.read_file(&format!("{}/value0", base)).unwrap_or("0".into()).parse().unwrap_or(0.0);

                        let sensor_type = match driver.as_str() {
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
                        .read_file("/sys/class/power_supply/lego-ev3-battery/voltage_now")
                        .unwrap_or("0".into())
                        .parse::<f32>()
                        .unwrap_or(0.0) / 1_000_000.0;

                    // Actualizar estado
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