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
    pub connected: bool,
    pub speed: i32,
}

#[derive(Clone, Default)]
pub struct Ev3State {
    pub connected: bool,
    pub motors: Vec<Motor>,
    pub sensors: Vec<Sensor>,
    pub battery_voltage: f32,
    pub alerts: Vec<String>,
}