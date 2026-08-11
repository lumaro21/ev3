import React, { createContext, useState, useEffect, useCallback, useContext } from 'react';

const API_BASE = 'http://localhost:8000';
const POLL_MS = 400;

const MOTOR_KEYS = ['A', 'B', 'C', 'D'];
const SENSOR_KEYS = ['1', '2', '3', '4'];

const emptyMotors = () =>
    Object.fromEntries(MOTOR_KEYS.map(k => [k, { speed: 0, position: 0, connected: false }]));

const emptySensors = () =>
    Object.fromEntries(SENSOR_KEYS.map(k => [k, { type: 'none', value: 0 }]));

const RobotContext = createContext();

export const useRobot = () => useContext(RobotContext);

export const RobotProvider = ({ children }) => {
    // 1. NUEVOS ESTADOS DE CONFIGURACIÓN
    const [connectionMode, setConnectionMode] = useState('simulated');
    const [targetIp, setTargetIp] = useState('127.0.0.1');

    // Iniciamos con valores en 0 mientras esperamos la respuesta del servidor
    const [telemetry, setTelemetry] = useState({
        connected: false,
        ip: '—',
        battery: 0,
        alerts: [],
        motors: emptyMotors(),
        sensors: emptySensors()
    });

    // 2. FUNCIÓN PARA ACTUALIZAR LA IP Y EL MODO EN EL BACKEND
    const updateConnectionConfig = useCallback(async (newMode, newIp) => {
        try {
            const response = await fetch(`${API_BASE}/api/config`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ mode: newMode, ip: newIp })
            });
            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            
            setConnectionMode(newMode);
            setTargetIp(newIp);
            console.log(`✅ Configuración actualizada en backend: Modo ${newMode}, IP: ${newIp}`);
        } catch (error) {
            console.error('❌ Error al actualizar la configuración:', error);
        }
    }, []);

    // 3. POLLING: Leer los datos de Python cada 400ms
    useEffect(() => {
        let cancelled = false;

        const fetchStatus = async () => {
            let data;
            try {
                const response = await fetch(`${API_BASE}/api/status`);
                if (!response.ok) throw new Error(`HTTP ${response.status}`);
                data = await response.json();
            } catch (error) {
                console.error('❌ No se pudo conectar con el Backend en Python:', error);
                // Sin backend marcamos desconectado, pero conservamos la última telemetría
                if (!cancelled) {
                    setTelemetry(prev => ({ ...prev, connected: false, alerts: ['Sin conexión con el backend'] }));
                }
                return;
            }
            if (cancelled) return;

            setTelemetry(prev => {
                // Adaptamos el formato de Python al que espera el Dashboard y el Gemelo 3D.
                const motors = emptyMotors();
                MOTOR_KEYS.forEach(k => { motors[k].position = prev.motors[k]?.position ?? 0; });

                (Array.isArray(data.motors) ? data.motors : []).forEach(m => {
                    const key = String(m.port ?? '').replace('out', '').toUpperCase();
                    if (!motors[key]) return;
                    motors[key] = {
                        speed: m.speed ?? 0,
                        position: prev.motors[key]?.position ?? 0,
                        connected: m.connected ?? false
                    };
                });

                const sensors = emptySensors();
                (Array.isArray(data.sensors) ? data.sensors : []).forEach(s => {
                    const key = String(s.port ?? '').replace('in', '');
                    if (!sensors[key]) return;
                    sensors[key] = { type: s.sensor_type ?? 'none', value: s.value ?? 0 };
                });

                return {
                    connected: !!data.connected,
                    ip: data.ip ?? prev.ip,
                    battery: data.battery ?? 0,
                    alerts: Array.isArray(data.alerts) ? data.alerts : [],
                    motors,
                    sensors
                };
            });
        };

        fetchStatus();
        const interval = setInterval(fetchStatus, POLL_MS);
        return () => { cancelled = true; clearInterval(interval); };
    }, []);

    // Integración de la posición angular (física simulada)
    useEffect(() => {
        const physicsInterval = setInterval(() => {
            setTelemetry(prev => {
                const motors = {};
                Object.entries(prev.motors).forEach(([port, m]) => {
                    motors[port] = { ...m, position: m.position + m.speed * 0.5 };
                });
                return { ...prev, motors };
            });
        }, POLL_MS);
        return () => clearInterval(physicsInterval);
    }, []);

    // 4. ENVIAR COMANDOS: Mandar las órdenes a Python
    const sendMotorCommand = useCallback(async (port, speed) => {
        const apiPort = `out${String(port).replace('out', '').toUpperCase()}`;

        try {
            const response = await fetch(`${API_BASE}/api/motor`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ port: apiPort, speed: Number(speed) || 0 })
            });
            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            console.log(`[HTTP POST] Comando enviado a Python -> ${apiPort} a velocidad ${speed}`);
        } catch (error) {
            console.error('❌ Error enviando comando a Python:', error);
        }
    }, []);

    const stopAllMotors = useCallback(async () => {
        try {
            await fetch(`${API_BASE}/api/stop_all`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({})
            });
        } catch (error) {
            console.error('❌ Error deteniendo los motores:', error);
        }
    }, []);

    return (
        <RobotContext.Provider value={{ 
            telemetry, 
            sendMotorCommand, 
            stopAllMotors,
            // Exportamos las nuevas herramientas para que Dashboard las use
            connectionMode,
            targetIp,
            updateConnectionConfig
        }}>
            {children}
        </RobotContext.Provider>
    );
};