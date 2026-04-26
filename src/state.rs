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
    pub port: String,       // "outA", "outB", ...
    pub motor_id: String,   // "motor0", "motor1", ... (ruta en sysfs)
    pub connected: bool,
    pub speed: i32,
}
 
/// Comandos que la GUI puede encolar para que el polling los ejecute.
#[derive(Clone, Debug)]
pub enum MotorCommand {
    SetSpeed { port: String, speed: i32 },
    Run      { port: String },
    Stop     { port: String },
}
 
#[derive(Clone, Default)]
pub struct Ev3State {
    pub connected: bool,
    pub motors: Vec<Motor>,
    pub sensors: Vec<Sensor>,
    pub battery_voltage: f32,
    pub alerts: Vec<String>,
 
    /// Velocidades deseadas que el usuario mueve con el slider (port → speed).
    /// La GUI escribe aquí; el polling las lee y las aplica.
    pub desired_speeds: std::collections::HashMap<String, i32>,
 
    /// Cola de comandos pendientes (Run / Stop).
    pub pending_commands: Vec<MotorCommand>,
}
 