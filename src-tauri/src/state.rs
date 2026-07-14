#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum SensorType {
    Touch,
    Color,
    Ultrasonic,
    Gyro,
    Infrared,
    Unknown(String),
}

#[derive(Clone, Debug)]
pub struct Sensor {
    pub port: String,
    pub sensor_type: SensorType,
    pub value: f32,
}

#[derive(Clone, Debug)]
pub struct Motor {
    pub port: String,
    pub motor_id: String,
    pub connected: bool,
    pub speed: i32,
}


#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum MotorCommand {
    // Comandos SSH legacy (fallback)
    SetSpeed { port: String, speed: i32 },
    Run      { port: String },
    Stop     { port: String },

    // Comandos HTTP — van directo al servidor en el EV3
    // speed: -100..=100 (negativo = marcha atrás)
    HttpSetSpeed { port: String, speed: i32 },
    HttpStop     { port: String },
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct Ev3State {
    pub connected: bool,
    pub motors: Vec<Motor>,
    pub sensors: Vec<Sensor>,
    pub battery_voltage: f32,
    pub alerts: Vec<String>,
    pub desired_speeds: std::collections::HashMap<String, i32>,
    pub pending_commands: Vec<MotorCommand>,
    pub ip: String,
    pub reconnect_requested: bool,
}

impl Default for Ev3State {
    fn default() -> Self {
        Self {
            connected: false,
            motors: vec![],
            sensors: vec![],
            battery_voltage: 0.0,
            alerts: vec![],
            desired_speeds: std::collections::HashMap::new(),
            pending_commands: vec![],
            ip: "192.168.20.232".to_string(),
            reconnect_requested: false,
        }
    }
}