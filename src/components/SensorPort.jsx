import "./Port.css";

const SENSOR_LABELS = {
  Touch: "Touch",
  Ultrasonic: "Ultrasónico",
  Color: "Color",
  Gyro: "Gyro",
  Infrared: "IR",
};

export default function SensorPort({ port, sensor }) {
  const connected = !!sensor;
  const label = port.replace("in", "");

  return (
    <div className={`port-card ${connected ? "connected sensor-connected" : ""}`}>
      <div className="port-label sensor-label">{label}</div>
      {connected ? (
        <>
          <div className="sensor-type">{SENSOR_LABELS[sensor.sensor_type] ?? sensor.sensor_type}</div>
          <SensorValue sensor={sensor} />
        </>
      ) : (
        <div className="port-empty">vacío</div>
      )}
    </div>
  );
}

function SensorValue({ sensor }) {
  const { sensor_type, value } = sensor;

  if (sensor_type === "Touch") {
    const pressed = value !== 0;
    return (
      <div className="sensor-value">
        <span className="touch-dot" style={{ background: pressed ? "#e05252" : "#444" }} />
        <span style={{ color: pressed ? "#e05252" : "#888" }}>
          {pressed ? "PRESIONADO" : "suelto"}
        </span>
      </div>
    );
  }

  if (sensor_type === "Ultrasonic") {
    const pct = Math.min(value / 255, 1);
    return (
      <div className="sensor-value">
        <div className="sensor-number">{value.toFixed(0)} cm</div>
        <div className="sensor-bar">
          <div className="sensor-bar-fill" style={{ width: `${pct * 100}%`, background: `hsl(${120 - pct * 120}, 70%, 45%)` }} />
        </div>
      </div>
    );
  }

  if (sensor_type === "Gyro") {
    return (
      <div className="sensor-value">
        <div className="sensor-number">{value.toFixed(0)}°</div>
      </div>
    );
  }

  if (sensor_type === "Infrared") {
    const pct = Math.min(value / 100, 1);
    return (
      <div className="sensor-value">
        <div className="sensor-number">{value.toFixed(0)}%</div>
        <div className="sensor-bar">
          <div className="sensor-bar-fill" style={{ width: `${pct * 100}%`, background: "#a855f7" }} />
        </div>
      </div>
    );
  }

  return <div className="sensor-value"><div className="sensor-number">{value.toFixed(1)}</div></div>;
}