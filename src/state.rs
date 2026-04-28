#[derive(Clone, Debug)]
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
pub enum MotorCommand {
    SetSpeed { port: String, speed: i32 },
    Run      { port: String },
    Stop     { port: String },
}

#[derive(Clone)]
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
            ip: "192.168.10.15".to_string(),
            reconnect_requested: false,
        }
    }
}