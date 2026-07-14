import { useRef, useMemo, useState } from "react";
import { Canvas, useFrame } from "@react-three/fiber";
import { Text, RoundedBox } from "@react-three/drei";
import * as THREE from "three";
import "./Ev3Twin.css";

// ─── Piso ─────────────────────────────────────────────────────────────────────

function Floor({ offsetX = 0, offsetZ = 0 }) {
  return (
    <group position={[offsetX, -3.2, offsetZ]}>
      <mesh rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <planeGeometry args={[80, 80]} />
        <meshStandardMaterial color="#12121f" roughness={0.9} metalness={0.1} />
      </mesh>
      <gridHelper args={[80, 80, "#2a2a5a", "#1a1a3a"]} position={[0, 0.01, 0]} />
      <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, 0.02, 0]}>
        <circleGeometry args={[5, 32]} />
        <meshStandardMaterial color="#2a2a5a" roughness={0.1} metalness={0.4} transparent opacity={0.25} />
      </mesh>
    </group>
  );
}

// ─── Rueda ────────────────────────────────────────────────────────────────────

function Wheel({ position, speed = 0, side }) {
  const spinRef = useRef();

  useFrame((_, delta) => {
    if (spinRef.current) spinRef.current.rotation.z -= (speed / 250) * delta * 6;
  });

  return (
    <group position={position} rotation={[0, Math.PI / 2, 0]}>
      <group ref={spinRef}>
        {/* Neumático */}
        <mesh castShadow>
          <torusGeometry args={[0.52, 0.22, 20, 40]} />
          <meshStandardMaterial color="#222" roughness={0.95} />
        </mesh>
        {/* Banda de rodadura */}
        {[0,1,2,3,4,5,6,7].map(i => (
          <mesh key={i} rotation={[i * Math.PI / 4, 0, 0]}>
            <torusGeometry args={[0.52, 0.04, 4, 40, Math.PI * 0.18]} />
            <meshStandardMaterial color="#333" roughness={0.9} />
          </mesh>
        ))}
        {/* Llanta */}
        <mesh castShadow>
          <cylinderGeometry args={[0.35, 0.35, 0.18, 20]} />
          <meshStandardMaterial color="#dddddd" metalness={0.7} roughness={0.2} />
        </mesh>
        {[0,1,2,3,4].map(i => (
          <mesh key={i} rotation={[0, 0, (i * Math.PI * 2) / 5]}>
            <boxGeometry args={[0.07, 0.62, 0.06]} />
            <meshStandardMaterial color="#aaaaaa" metalness={0.6} roughness={0.3} />
          </mesh>
        ))}
        <mesh>
          <cylinderGeometry args={[0.1, 0.1, 0.2, 12]} />
          <meshStandardMaterial color="#888" metalness={0.8} roughness={0.2} />
        </mesh>
      </group>
    </group>
  );
}

// ─── Rueda loca ───────────────────────────────────────────────────────────────

function CasterWheel({ position }) {
  return (
    <group position={position}>
      <mesh castShadow>
        <sphereGeometry args={[0.2, 14, 14]} />
        <meshStandardMaterial color="#333" roughness={0.8} />
      </mesh>
      <mesh position={[0, 0.25, 0]}>
        <cylinderGeometry args={[0.06, 0.06, 0.3, 8]} />
        <meshStandardMaterial color="#666" metalness={0.6} />
      </mesh>
    </group>
  );
}

// ─── Motor grande ─────────────────────────────────────────────────────────────

function LargeMotor({ position, rotation, speed = 0, port, connected }) {
  const shaftRef = useRef();
  const glowRef  = useRef();
  const active   = connected && speed !== 0;

  useFrame((state, delta) => {
    if (shaftRef.current && active) {
      shaftRef.current.rotation.z += (speed / 350) * delta * 10;
    }
    if (glowRef.current) {
      glowRef.current.intensity = active
        ? 0.6 + Math.sin(state.clock.elapsedTime * 4) * 0.2
        : 0;
    }
  });

  return (
    <group position={position} rotation={rotation}>
      <RoundedBox args={[0.75, 0.75, 1.2]} radius={0.07} smoothness={5} castShadow>
        <meshStandardMaterial color={connected ? "#3a4060" : "#252535"} roughness={0.3} metalness={0.5} />
      </RoundedBox>
      <mesh position={[0, 0, 0.61]}>
        <boxGeometry args={[0.73, 0.73, 0.02]} />
        <meshStandardMaterial color={connected ? "#4a5080" : "#2a2a3a"} roughness={0.3} />
      </mesh>
      <mesh position={[0, 0.385, 0]}>
        <boxGeometry args={[0.76, 0.09, 1.22]} />
        <meshStandardMaterial
          color={active ? "#4ec94e" : connected ? "#2a5a2a" : "#252535"}
          emissive={active ? "#2a8a2a" : "#000"}
          emissiveIntensity={active ? 0.8 : 0}
          roughness={0.3}
        />
      </mesh>
      <group ref={shaftRef} position={[0, 0, 0.72]}>
        <mesh castShadow rotation={[Math.PI/2,0,0]}>
          <cylinderGeometry args={[0.09, 0.09, 0.28, 10]} />
          <meshStandardMaterial color="#cccccc" metalness={0.85} roughness={0.15} />
        </mesh>
        {[0, Math.PI/2].map((r, i) => (
          <mesh key={i} rotation={[Math.PI/2, r, 0]}>
            <boxGeometry args={[0.2, 0.04, 0.3]} />
            <meshStandardMaterial color="#aaaaaa" metalness={0.7} />
          </mesh>
        ))}
      </group>
      <pointLight ref={glowRef} position={[0, 0, 0.8]} color="#4ec94e" intensity={0} distance={2} />
      <Text position={[0, 0, 0.63]} fontSize={0.18} color={connected ? "#4ec94e" : "#555"} anchorX="center" anchorY="middle" fontWeight="bold">
        {port?.replace("out","") ?? ""}
      </Text>
      {active && (
        <Text position={[0, -0.25, 0.63]} fontSize={0.1} color="#ffcc44" anchorX="center">
          {speed > 0 ? "▲" : "▼"} {Math.abs(speed)}
        </Text>
      )}
    </group>
  );
}

// ─── Brazo ────────────────────────────────────────────────────────────────────

function Arm({ position, side, motor, speed = 0 }) {
  const shoulderRef = useRef();
  const forearmRef  = useRef();
  const dir    = side === "left" ? 1 : -1;
  const active = motor?.connected && speed !== 0;

  useFrame((state, delta) => {
    if (shoulderRef.current) {
      const swing = active ? dir * 0.25 * Math.sin(state.clock.elapsedTime * 3) : 0;
      shoulderRef.current.rotation.z += (swing - shoulderRef.current.rotation.z) * delta * 3;
    }
    if (forearmRef.current) {
      const bend = active ? 0.3 + Math.sin(state.clock.elapsedTime * 3 + 0.5) * 0.2 : 0.2;
      forearmRef.current.rotation.z += (bend * dir - forearmRef.current.rotation.z) * delta * 2;
    }
  });

  return (
    <group position={position}>
      <mesh castShadow>
        <sphereGeometry args={[0.22, 14, 14]} />
        <meshStandardMaterial color="#4a4e70" metalness={0.5} roughness={0.3} />
      </mesh>
      <group ref={shoulderRef}>
        <LargeMotor position={[0,0,0]} rotation={[0,0,0]} speed={speed} port={motor?.port} connected={motor?.connected ?? false} />
        <mesh position={[0, -0.85, 0]} castShadow>
          <boxGeometry args={[0.22, 1.1, 0.22]} />
          <meshStandardMaterial color="#353850" roughness={0.5} metalness={0.3} />
        </mesh>
        <mesh position={[dir * 0.08, -0.85, 0]}>
          <boxGeometry args={[0.06, 1.0, 0.18]} />
          <meshStandardMaterial color="#404468" roughness={0.4} />
        </mesh>
        <mesh position={[0, -1.45, 0]} castShadow>
          <sphereGeometry args={[0.18, 12, 12]} />
          <meshStandardMaterial color="#555" metalness={0.5} roughness={0.3} />
        </mesh>
        <group ref={forearmRef} position={[0, -1.45, 0]}>
          <mesh position={[dir * 0.12, -0.6, 0]} castShadow>
            <boxGeometry args={[0.2, 1.0, 0.2]} />
            <meshStandardMaterial color="#2d3048" roughness={0.5} metalness={0.3} />
          </mesh>
          <mesh position={[dir * 0.22, -1.15, 0]}>
            <sphereGeometry args={[0.14, 10, 10]} />
            <meshStandardMaterial color="#666" metalness={0.5} />
          </mesh>
          <group position={[dir * 0.28, -1.38, 0]}>
            {[-1, 1].map((s, i) => (
              <group key={i} rotation={[0, 0, s * (active ? 0.3 : 0.5)]}>
                <mesh position={[dir * 0.1, s * 0.08, 0]} castShadow>
                  <boxGeometry args={[0.28, 0.08, 0.1]} />
                  <meshStandardMaterial color="#c0392b" roughness={0.4} metalness={0.2} />
                </mesh>
                <mesh position={[dir * 0.24, s * 0.08, 0]}>
                  <boxGeometry args={[0.06, 0.06, 0.08]} />
                  <meshStandardMaterial color="#e74c3c" roughness={0.3} />
                </mesh>
              </group>
            ))}
          </group>
        </group>
      </group>
    </group>
  );
}

// ─── Antena ───────────────────────────────────────────────────────────────────

function Antenna({ position, phase = 0, connected = false }) {
  const tipRef = useRef();
  const ref    = useRef();

  useFrame(({ clock }) => {
    if (ref.current) {
      ref.current.rotation.z = Math.sin(clock.elapsedTime * 1.8 + phase) * 0.06;
    }
    if (tipRef.current) {
      tipRef.current.material.emissiveIntensity = connected
        ? 0.6 + Math.sin(clock.elapsedTime * 3 + phase) * 0.4
        : 0.1;
    }
  });

  return (
    <group position={position} ref={ref}>
      <mesh>
        <cylinderGeometry args={[0.05, 0.07, 0.18, 8]} />
        <meshStandardMaterial color="#777" metalness={0.7} roughness={0.3} />
      </mesh>
      <mesh position={[0, 0.38, 0]}>
        <cylinderGeometry args={[0.03, 0.05, 0.6, 8]} />
        <meshStandardMaterial color="#999" metalness={0.6} roughness={0.3} />
      </mesh>
      <mesh ref={tipRef} position={[0, 0.75, 0]}>
        <sphereGeometry args={[0.08, 10, 10]} />
        <meshStandardMaterial
          color={connected ? "#e74c3c" : "#666"}
          emissive={connected ? "#e74c3c" : "#220000"}
          emissiveIntensity={0.4}
        />
      </mesh>
    </group>
  );
}

// ─── Sensor ───────────────────────────────────────────────────────────────────

function SensorBlock({ position, sensor, port }) {
  const connected = !!sensor;
  const eyeRef    = useRef();

  const color = useMemo(() => {
    if (!connected) return "#2a2a3a";
    switch (sensor.sensor_type) {
      case "Touch":      return sensor.value ? "#c0392b" : "#2980b9";
      case "Ultrasonic": return `hsl(${120 - Math.min(sensor.value/255,1)*120},65%,35%)`;
      case "Gyro":       return "#d4930a";
      case "Infrared":   return "#7d3c98";
      case "Color":      return "#1a8a1a";
      default:           return "#2980b9";
    }
  }, [connected, sensor]);

  useFrame(({ clock }) => {
    if (eyeRef.current && connected) {
      eyeRef.current.material.emissiveIntensity =
        0.6 + Math.sin(clock.elapsedTime * 2) * 0.3;
    }
  });

  return (
    <group position={position}>
      <RoundedBox args={[0.42, 0.38, 0.32]} radius={0.05} smoothness={4} castShadow>
        <meshStandardMaterial color={color} roughness={0.4} metalness={0.2}
          emissive={connected ? color : "#000"} emissiveIntensity={connected ? 0.2 : 0} />
      </RoundedBox>
      <mesh ref={eyeRef} position={[0, 0, 0.175]}>
        <circleGeometry args={[0.1, 16]} />
        <meshStandardMaterial color={connected ? "#fff" : "#222"}
          emissive={connected ? color : "#000"} emissiveIntensity={0.8} />
      </mesh>
      {connected && (
        <mesh position={[0.1, 0.1, 0.175]}>
          <circleGeometry args={[0.04, 12]} />
          <meshStandardMaterial color="#fff" emissive={color} emissiveIntensity={1.0} />
        </mesh>
      )}
      <Text position={[0, -0.26, 0.17]} fontSize={0.09} color={connected ? "#ddd" : "#444"} anchorX="center">
        {port?.replace("in","S") ?? ""}
      </Text>
      {connected && (
        <Text position={[0, 0.26, 0.17]} fontSize={0.09} color="#fff" anchorX="center">
          {sensor.sensor_type === "Touch"      ? (sensor.value ? "ON" : "—") :
           sensor.sensor_type === "Ultrasonic" ? `${sensor.value.toFixed(0)}cm` :
           sensor.sensor_type === "Gyro"       ? `${sensor.value.toFixed(0)}°` :
           sensor.sensor_type === "Infrared"   ? `${sensor.value.toFixed(0)}%` : ""}
        </Text>
      )}
    </group>
  );
}

// ─── Cuerpo EV3 ───────────────────────────────────────────────────────────────

function Ev3Body({ status }) {
  const connected = status?.connected ?? false;
  const battery   = status?.battery ?? 0;
  const batPct    = Math.min(Math.max((battery - 6) / (8.4 - 6), 0), 1);
  const screenRef = useRef();

  useFrame(({ clock }) => {
    if (screenRef.current && connected) {
      screenRef.current.material.emissiveIntensity =
        0.4 + Math.sin(clock.elapsedTime * 0.5) * 0.08;
    }
  });

  return (
    <group>
      {/* Cuerpo principal — color más claro para contrastar */}
      <RoundedBox args={[2.5, 3.4, 1.1]} radius={0.12} smoothness={6} castShadow receiveShadow>
        <meshStandardMaterial color="#2a3050" roughness={0.3} metalness={0.35} />
      </RoundedBox>
      {/* Bordes metálicos */}
      <mesh position={[-1.26, 0, 0]}>
        <boxGeometry args={[0.04, 3.4, 1.1]} />
        <meshStandardMaterial color="#4a5080" roughness={0.2} metalness={0.6} />
      </mesh>
      <mesh position={[1.26, 0, 0]}>
        <boxGeometry args={[0.04, 3.4, 1.1]} />
        <meshStandardMaterial color="#4a5080" roughness={0.2} metalness={0.6} />
      </mesh>
      {/* Franja roja */}
      <mesh position={[0, 1.35, 0.56]}>
        <boxGeometry args={[2.5, 0.52, 0.02]} />
        <meshStandardMaterial color="#c0392b" roughness={0.25} metalness={0.1}
          emissive="#8a1a0a" emissiveIntensity={0.3} />
      </mesh>
      <mesh position={[0, 1.55, 0.57]}>
        <boxGeometry args={[2.4, 0.06, 0.01]} />
        <meshStandardMaterial color="#e05040" emissive="#e05040" emissiveIntensity={0.5} />
      </mesh>
      {/* Texto */}
      <Text position={[0, 1.36, 0.58]} fontSize={0.115} color="#ffdddd" anchorX="center" letterSpacing={0.12} fontWeight="bold">
        MINDSTORMS
      </Text>
      <Text position={[0.92, 1.36, 0.58]} fontSize={0.11} color="#ffffff" anchorX="center">EV3</Text>
      {/* Pantalla */}
      <mesh ref={screenRef} position={[0, 0.42, 0.56]}>
        <boxGeometry args={[1.7, 1.08, 0.01]} />
        <meshStandardMaterial
          color={connected ? "#2d6e34" : "#0a0f0a"}
          roughness={0.1}
          emissive={connected ? "#1a4d22" : "#000"}
          emissiveIntensity={0.4}
        />
      </mesh>
      <mesh position={[0, 0.42, 0.555]}>
        <boxGeometry args={[1.78, 1.16, 0.01]} />
        <meshStandardMaterial color="#0d0d0d" roughness={0.5} />
      </mesh>
      {/* Contenido pantalla */}
      <Text position={[0, 0.72, 0.575]} fontSize={0.15} color={connected ? "#7dff7d" : "#1a2a1a"} anchorX="center" fontWeight="bold">EV3</Text>
      <Text position={[0, 0.50, 0.575]} fontSize={0.095} color={connected ? "#7dff7d" : "#1a2a1a"} anchorX="center">
        {connected ? "CONECTADO" : "SIN CONEXION"}
      </Text>
      <Text position={[0, 0.30, 0.575]} fontSize={0.09} color="#ffcc44" anchorX="center">
        {connected ? `BAT ${battery.toFixed(1)}V` : ""}
      </Text>
      {/* Barra batería */}
      {connected && (
        <group position={[0, 0.12, 0.575]}>
          <mesh>
            <boxGeometry args={[1.1, 0.08, 0.01]} />
            <meshStandardMaterial color="#0a1a0a" />
          </mesh>
          <mesh position={[-0.55 + batPct * 0.55, 0, 0.005]}>
            <boxGeometry args={[batPct * 1.1, 0.06, 0.01]} />
            <meshStandardMaterial
              color={batPct > 0.5 ? "#4ec94e" : batPct > 0.25 ? "#f0a500" : "#e05252"}
              emissive={batPct > 0.5 ? "#1a5a1a" : "#000"} emissiveIntensity={0.5}
            />
          </mesh>
        </group>
      )}
      {/* Botón naranja */}
      <mesh position={[0, -0.5, 0.57]} castShadow>
        <cylinderGeometry args={[0.24, 0.22, 0.09, 32]} />
        <meshStandardMaterial color="#e67e22" roughness={0.25} metalness={0.25}
          emissive="#aa5500" emissiveIntensity={0.4} />
      </mesh>
      {/* D-pad */}
      {[[-0.45,-0.5,"<"],[0.45,-0.5,">"],[0,-0.22,"^"],[0,-0.78,"v"]].map(([x,y,l],i) => (
        <group key={i} position={[x,y,0.57]}>
          <mesh><boxGeometry args={[0.2,0.15,0.05]} /><meshStandardMaterial color="#1a1d2e" roughness={0.6} /></mesh>
          <Text position={[0,0,0.03]} fontSize={0.08} color="#555" anchorX="center">{l}</Text>
        </group>
      ))}
      {/* Botones laterales */}
      {[[-0.7,-0.28],[0.7,-0.28],[-0.7,-0.7],[0.7,-0.7]].map(([x,y],i) => (
        <mesh key={i} position={[x,y,0.57]}>
          <cylinderGeometry args={[0.09,0.09,0.05,16]} />
          <meshStandardMaterial color="#1e2035" roughness={0.5} />
        </mesh>
      ))}
      {/* Puerto USB */}
      <mesh position={[0,-1.1,0.57]}>
        <boxGeometry args={[0.3,0.14,0.04]} />
        <meshStandardMaterial color="#111" roughness={0.5} />
      </mesh>
      {/* Ventilación */}
      {[-0.4,-0.2,0,0.2,0.4].map((x,i) => (
        <mesh key={i} position={[x,-1.45,0.57]}>
          <boxGeometry args={[0.06,0.18,0.02]} />
          <meshStandardMaterial color="#111" />
        </mesh>
      ))}
    </group>
  );
}

// ─── Robot completo ───────────────────────────────────────────────────────────

function Ev3Robot({ status }) {
  const robotRef  = useRef();
  const motors    = status?.motors  ?? [];
  const sensors   = status?.sensors ?? [];

  const motorB = motors.find(m => m.port === "outB");
  const motorD = motors.find(m => m.port === "outD");
  const motorA = motors.find(m => m.port === "outA");
  const motorC = motors.find(m => m.port === "outC");

  const speedB    = motorB?.connected ? motorB.speed : 0;
  const speedD    = motorD?.connected ? motorD.speed : 0;
  const avgSpeed  = (speedB + speedD) / 2;
  const diffSpeed = speedB - speedD;

  useFrame((state, delta) => {
    if (!robotRef.current) return;
    const moving = Math.abs(avgSpeed) > 5;
    const tilt   = moving ? Math.sin(state.clock.elapsedTime * 9) * 0.025 : 0;
    const lean   = moving ? (avgSpeed > 0 ? -0.06 : 0.06) : 0;
    const turn   = Math.abs(diffSpeed) > 10 ? diffSpeed * 0.0003 : 0;
    robotRef.current.rotation.z += (tilt + turn - robotRef.current.rotation.z) * delta * 4;
    robotRef.current.rotation.x += (lean - robotRef.current.rotation.x) * delta * 3;
  });

  return (
    <group>
      <group ref={robotRef}>
        {/* Chasis */}
        <group position={[0, -2.2, 0]}>
          <RoundedBox args={[2.8, 0.35, 1.5]} radius={0.08} smoothness={4} castShadow receiveShadow>
            <meshStandardMaterial color="#1e2235" roughness={0.6} metalness={0.35} />
          </RoundedBox>
          <mesh position={[0, 0.15, 0]}>
            <boxGeometry args={[2.6, 0.06, 1.3]} />
            <meshStandardMaterial color="#2a2e48" roughness={0.5} />
          </mesh>
        </group>
        {/* Ruedas */}
        <Wheel position={[-1.55, -2.15, 0.1]} speed={speedB} side="left"  />
        <Wheel position={[ 1.55, -2.15, 0.1]} speed={speedD} side="right" />
        <CasterWheel position={[0, -2.4, -0.75]} />
        {/* Soportes */}
        {[-0.65, 0.65].map((x, i) => (
          <group key={i} position={[x, -1.45, 0]}>
            <RoundedBox args={[0.28, 1.2, 1.0]} radius={0.06} smoothness={4} castShadow>
              <meshStandardMaterial color="#252840" roughness={0.5} metalness={0.25} />
            </RoundedBox>
            <mesh position={[0, 0, 0.52]}>
              <boxGeometry args={[0.26, 1.1, 0.04]} />
              <meshStandardMaterial color="#303560" roughness={0.4} />
            </mesh>
          </group>
        ))}
        {/* Cuerpo EV3 */}
        <Ev3Body status={status} />
        {/* Brazos */}
        <Arm position={[-1.52, 0.65, 0]} side="left"  motor={motorA} speed={motorA?.connected ? motorA.speed : 0} />
        <Arm position={[ 1.52, 0.65, 0]} side="right" motor={motorC} speed={motorC?.connected ? motorC.speed : 0} />
        {/* Sensores */}
        {["in1","in2","in3","in4"].map((port, i) => {
          const sensor = sensors.find(s => s.port === port);
          return <SensorBlock key={port} position={[(i - 1.5) * 0.58, -0.95, 0.58]} sensor={sensor} port={port} />;
        })}
        {/* Cuello */}
        <mesh position={[0, 1.82, 0]} castShadow>
          <boxGeometry args={[1.3, 0.28, 0.88]} />
          <meshStandardMaterial color="#252840" roughness={0.45} metalness={0.25} />
        </mesh>
        {/* Antenas */}
        <Antenna position={[-0.42, 2.08, 0.12]} phase={0}            connected={status?.connected} />
        <Antenna position={[ 0.42, 2.18, 0.12]} phase={Math.PI / 2} connected={status?.connected} />
        <Antenna position={[ 0.0,  2.28, 0.12]} phase={Math.PI}     connected={status?.connected} />
      </group>
    </group>
  );
}

// ─── Escena ───────────────────────────────────────────────────────────────────

function Scene({ status }) {
  // 1. Usamos refs puras para la matemática (esto no causa re-renders de React)
  const posRef = useRef(new THREE.Vector3(0, 0, 0));
  const rotRef = useRef(0);
  
  // 2. Ref para controlar la malla del robot directamente
  const groupRef = useRef();

  const motors = status?.motors ?? [];
  const motorB = motors.find(m => m.port === "outB");
  const motorD = motors.find(m => m.port === "outD");
  const speedB = (motorB?.connected ? motorB.speed : 0) / 1000;
  const speedD = (motorD?.connected ? motorD.speed : 0) / 1000;

  useFrame((state, delta) => {
    // 3. Calculamos la física
    const linear  = (speedB + speedD) / 2;
    const angular = (speedD - speedB) * 1.5;

    rotRef.current += angular * delta;
    posRef.current.x += Math.sin(rotRef.current) * linear * delta * 8;
    posRef.current.z += Math.cos(rotRef.current) * linear * delta * 8;

    // 4. Aplicamos la matemática directamente al objeto 3D saltándonos el ciclo de React
    if (groupRef.current) {
        groupRef.current.position.copy(posRef.current);
        groupRef.current.rotation.y = rotRef.current;
    }

    // 5. La cámara sigue las coordenadas matemáticas fluidamente
    const behind = new THREE.Vector3(
      posRef.current.x + Math.sin(rotRef.current) * 8,
      5,
      posRef.current.z + Math.cos(rotRef.current) * 8
    );
    state.camera.position.lerp(behind, delta * 1.8);
    state.camera.lookAt(posRef.current.x, 0, posRef.current.z);
  });

  return (
    <>
      <ambientLight intensity={0.9} />
      <directionalLight position={[6, 12, 6]} intensity={2.5} castShadow
        shadow-mapSize={[2048, 2048]}
        shadow-camera-far={40}
        shadow-camera-left={-10}
        shadow-camera-right={10}
        shadow-camera-top={10}
        shadow-camera-bottom={-10}
      />
      <directionalLight position={[-6, 8, -4]} intensity={1.2} color="#aabbff" />
      <directionalLight position={[0, -4, 6]} intensity={0.8} color="#ffffff" />
      <pointLight position={[0, 6, 6]}  intensity={1.5} color="#ffffff" />
      <pointLight position={[-4, 4, 4]} intensity={0.8} color="#88aaff" />
      <pointLight position={[4, 4, 4]}  intensity={0.8} color="#ffddaa" />

      {/* El piso estático. Ahora el robot es el que se moverá a través del mundo */}
      <Floor offsetX={0} offsetZ={0} />
      
      {/* Envolvemos el robot en el grupo que controlamos matemáticamente */}
      <group ref={groupRef}>
         <Ev3Robot status={status} />
      </group>
    </>
  );
}

// ─── Componente principal ─────────────────────────────────────────────────────

export default function Ev3Twin({ status }) {
  const motors  = status?.motors  ?? [];
  const sensors = status?.sensors ?? [];

  return (
    <div className="twin-container">
      <div className="twin-hint">🤖 El robot se mueve en tiempo real con los motores</div>

      <Canvas
        camera={{ position: [0, 5, 11], fov: 46 }}
        shadows
        style={{ background: "linear-gradient(180deg, #1a1a2e 0%, #0d0d1a 100%)", flex: 1 }}
        gl={{ antialias: true, toneMapping: THREE.ACESFilmicToneMapping, toneMappingExposure: 1.2 }}
      >
        <Scene status={status} />
      </Canvas>

      <div className="twin-info">
        <div className="twin-info-title">Estado en vivo</div>

        <div className="twin-section">
          <div className="twin-section-label">Motores</div>
          {["outA","outB","outC","outD"].map(port => {
            const m = motors.find(m => m.port === port);
            return (
              <div key={port} className="twin-row">
                <span className={`twin-dot ${m?.connected ? "on" : ""}`} />
                <span className="twin-port">{port.replace("out","")}</span>
                <span className="twin-val">{m?.connected ? `${m.speed} rpm` : "vacío"}</span>
              </div>
            );
          })}
        </div>

        <div className="twin-section">
          <div className="twin-section-label">Sensores</div>
          {["in1","in2","in3","in4"].map(port => {
            const s = sensors.find(s => s.port === port);
            return (
              <div key={port} className="twin-row">
                <span className={`twin-dot sensor ${s ? "on" : ""}`} />
                <span className="twin-port">{port.replace("in","S")}</span>
                <span className="twin-val">{s ? `${s.sensor_type} · ${s.value.toFixed(1)}` : "vacío"}</span>
              </div>
            );
          })}
        </div>

        <div className="twin-section">
          <div className="twin-section-label">Sistema</div>
          <div className="twin-row">
            <span className={`twin-dot ${status?.connected ? "on" : ""}`} />
            <span className="twin-port">SSH</span>
            <span className="twin-val">{status?.connected ? "OK" : "Sin conexión"}</span>
          </div>
          <div className="twin-row">
            <span className="twin-dot on" style={{ background: "#f0a500" }} />
            <span className="twin-port">BAT</span>
            <span className="twin-val">{status?.battery?.toFixed(2) ?? "—"}V</span>
          </div>
        </div>
      </div>
    </div>
  );
}