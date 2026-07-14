
import React, { createContext, useState, useEffect, useContext } from 'react';

const RobotContext = createContext();

export const useRobot = () => useContext(RobotContext);

export const RobotProvider = ({ children }) => {
    // Iniciamos con valores en 0 mientras esperamos la respuesta del servidor
    const [telemetry, setTelemetry] = useState({
        battery: 0,
        motors: {
            A: { speed: 0, position: 0 },
            B: { speed: 0, position: 0 },
            C: { speed: 0, position: 0 },
            D: { speed: 0, position: 0 }
        },
        sensors: {
            1: { type: 'none', value: 0 },
            2: { type: 'none', value: 0 },
            3: { type: 'none', value: 0 },
            4: { type: 'none', value: 0 }
        }
    });

    // 1. POLLING: Leer los datos de Python cada 400ms
    useEffect(() => {
        const fetchStatus = async () => {
            try {
                const response = await fetch('http://localhost:8000/api/status');
                if (response.ok) {
                    const data = await response.json();
                    
                    // Adaptamos el formato de Python al que espera tu Dashboard y Gemelo 3D
                    const newMotors = { A: { speed: 0, position: 0 }, B: { speed: 0, position: 0 }, C: { speed: 0, position: 0 }, D: { speed: 0, position: 0 } };
                    data.motors.forEach(m => {
                        const portLetter = m.port.replace('out', '');
                        // Mantenemos la posición anterior por ahora para que el Gemelo 3D no colapse
                        newMotors[portLetter] = { speed: m.speed, position: telemetry.motors[portLetter]?.position || 0 };
                    });

                    const newSensors = { 1: { type: 'none', value: 0 }, 2: { type: 'none', value: 0 }, 3: { type: 'none', value: 0 }, 4: { type: 'none', value: 0 } };
                    data.sensors.forEach(s => {
                        const portNum = s.port.replace('in', '');
                        newSensors[portNum] = { type: s.sensor_type.toLowerCase(), value: s.value };
                    });

                    setTelemetry(prev => ({
                        ...prev,
                        battery: data.battery,
                        motors: newMotors,
                        sensors: newSensors
                    }));
                }
            } catch (error) {
                console.error("❌ No se pudo conectar con el Backend en Python:", error);
            }
        };

        const interval = setInterval(fetchStatus, 400);
        return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // Simulación de física temporal para que el Gemelo 3D siga girando
    useEffect(() => {
        const physicsInterval = setInterval(() => {
            setTelemetry(prev => {
                const updatedMotors = { ...prev.motors };
                Object.keys(updatedMotors).forEach(port => {
                    updatedMotors[port].position += (updatedMotors[port].speed * 0.5);
                });
                return { ...prev, motors: updatedMotors };
            });
        }, 400);
        return () => clearInterval(physicsInterval);
    }, []);

    // 2. ENVIAR COMANDOS: Mandar las órdenes a Python
    const sendMotorCommand = async (port, speed) => {
        const cleanPort = port.replace("out", ""); 
        const apiPort = `out${cleanPort}`; // Aseguramos el formato "outA" para Python

        try {
            await fetch('http://localhost:8000/api/motor', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ port: apiPort, speed: speed })
            });
            console.log(`[HTTP POST] Comando enviado a Python -> ${apiPort} a velocidad ${speed}`);
        } catch (error) {
            console.error("❌ Error enviando comando a Python:", error);
        }
    };

    return (
        <RobotContext.Provider value={{ telemetry, sendMotorCommand }}>
            {children}
        </RobotContext.Provider>
    );
};