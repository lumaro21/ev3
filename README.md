# EV3 Controller

Aplicación web para controlar y monitorizar un robot **LEGO Mindstorms EV3**:
dashboard de motores y sensores en tiempo real, control por teclado (W/A/S/D),
gemelo digital 3D y una terminal remota.

## Arquitectura

Tres piezas, cada una en su propio puerto:

```
  React + Vite            Puente FastAPI            Servidor del robot
    (:1420)         →         (:8000)         →          (:8080)
  navegador                    PC                     ladrillo EV3
                    ←──────── telemetría ────────
```

| Componente | Ruta | Puerto | Función |
|---|---|---|---|
| Frontend | [`src/`](src/) | 1420 | Interfaz React. Hace polling cada 400 ms a `/api/status` y envía comandos a `/api/motor`. |
| Puente HTTP | [`ev3_backend/ev3_bridge.py`](ev3_backend/ev3_bridge.py) | 8000 | Proxy con CORS. Normaliza la telemetría del robot al formato que consume el frontend y degrada con elegancia si el robot no responde. |
| Servidor del robot | [`api_simulador/main.py`](api_simulador/main.py) | 8080 | Corre **dentro** del ladrillo. Mueve los motores con `ev3dev2` y lee sensores y batería del sysfs de ev3dev. Solo biblioteca estándar: no necesita instalar nada en el EV3. |
| Simulador | [`tools/fake_ev3.py`](tools/fake_ev3.py) | 8080 | Sustituye al ladrillo durante el desarrollo. Ver más abajo. |

`src-tauri/` contiene la implementación original de escritorio en Rust + Tauri.
Es código heredado: el frontend ya no la usa.

## Requisitos

- **Node.js** 18 o superior
- **Python** 3.9 o superior

```bash
npm install
pip install fastapi uvicorn requests
```

En el **ladrillo EV3 no hay que instalar nada**: su servidor usa solo la
biblioteca estándar de Python. FastAPI no es una opción ahí, porque exige
Python 3.7+ (ev3dev-stretch trae 3.5) y pydantic no publica wheels para el ARM
del EV3.

## Entorno de Desarrollo Local (Simulado)

Permite trabajar en la interfaz **sin el robot conectado**. Abre tres terminales
y lanza un servicio en cada una, en este orden.

### Paso 1 — Simulador del EV3 (puerto 8080)

Sustituye al ladrillo físico.

```bash
python tools/fake_ev3.py
```

Debe imprimir `EV3 simulado escuchando en http://127.0.0.1:8080`.

### Paso 2 — Puente HTTP (puerto 8000)

Apúntalo al simulador con la variable `EV3_IP`:

```bash
EV3_IP=127.0.0.1 python ev3_backend/ev3_bridge.py
```

En PowerShell:

```powershell
$env:EV3_IP = "127.0.0.1"; python ev3_backend/ev3_bridge.py
```

Comprobación rápida: `curl http://localhost:8000/api/status` debe devolver
motores, sensores y batería.

### Paso 3 — Frontend (puerto 1420)

```bash
npm run dev
```

Abre **http://localhost:1420**.

### Nota sobre el simulador

El simulador **devuelve telemetría falsa, pero respeta los contratos de la API
real**: monta un sysfs ficticio y ejecuta el mismo
[`api_simulador/main.py`](api_simulador/main.py) que corre en el ladrillo, con
la librería `ev3dev2` sustituida por un stub. Las rutas, los nombres de los
campos y las unidades son idénticos a los del hardware, así que se pueden
probar el polling, el dashboard y el gemelo 3D sin conectar el robot.

El lazo se cierra de verdad: mover un slider escribe el `speed` en el sysfs
ficticio y ese valor vuelve por la telemetría en el siguiente ciclo, igual que
haría el motor real.

Hardware que simula, elegido para cubrir ambos estados de cada tarjeta:

| | Presente | Ausente |
|---|---|---|
| Motores | `outA`, `outB`, `outD` | `outC` → "vacío" + alerta |
| Sensores | `in1` ultrasónico (42 cm), `in4` táctil | `in2`, `in3` → "vacío" |
| Batería | 7.85 V | |

Para verificar la degradación, detén el simulador con la interfaz abierta: el
dashboard debe pasar a "Sin conexión" sin errores en consola.

## Uso con el robot físico

El ladrillo debe tener [ev3dev](https://www.ev3dev.org/) y estar en **la misma
subred** que el PC. Comprueba la IP en el EV3 (*Wireless and Networks* → tu red)
y con `ipconfig` en el PC: los tres primeros grupos deben coincidir.

1. Copia el servidor al ladrillo y arráncalo (usuario y contraseña por defecto
   de ev3dev: `robot` / `maker`):

   ```bash
   scp api_simulador/main.py robot@<IP-DEL-EV3>:/home/robot/
   ssh robot@<IP-DEL-EV3> "python3 /home/robot/main.py"
   ```

   Debe imprimir `EV3 escuchando en http://0.0.0.0:8080`. Déjalo abierto.

2. En el PC, arranca el puente y el frontend:

   ```bash
   python ev3_backend/ev3_bridge.py
   npm run dev
   ```

3. En el dashboard, cambia el modo a **Hardware** e introduce la IP del robot.
   El frontend se lo comunica al puente por `POST /api/config`, así que no hace
   falta reiniciar nada.

   Alternativa por variable de entorno al arrancar el puente:
   `EV3_PORT=8080 python ev3_backend/ev3_bridge.py`.

### Si el robot no conecta

Recorre la cadena de fuera hacia dentro; el primer paso que falle es la causa:

| Comprobación | Comando | Qué significa si falla |
|---|---|---|
| ¿Hay ruta hasta el robot? | `ping <IP-DEL-EV3>` | Subredes distintas o el EV3 no está asociado a la WiFi. |
| ¿Está vivo ev3dev? | `ssh robot@<IP-DEL-EV3>` | El ladrillo está arrancado pero sin red, o la IP cambió. |
| ¿Corre el servidor? | `curl http://<IP-DEL-EV3>:8080/status` | El paso 1 no está en marcha: es el fallo más habitual. |
| ¿Apunta bien el puente? | `curl http://localhost:8000/api/status` | Sigue en modo simulado; cambia a Hardware en el dashboard. |

El puente **nunca se queda colgado** si el robot desaparece: responde al
instante con `connected: false` y reintenta cada segundo y medio.

## API

### Puente (`:8000`) — lo que consume el frontend

| Método | Ruta | Descripción |
|---|---|---|
| `GET` | `/api/status` | `{connected, ip, battery, motors[], sensors[], alerts[]}`. Si el robot no responde devuelve la misma estructura con `connected: false`. |
| `POST` | `/api/motor` | `{port, speed}` — `port` acepta `"A"` o `"outA"`; `speed` en % (−100…100). |
| `POST` | `/api/stop_all` | `{port}` opcional; sin él detiene los cuatro motores. |

### Robot (`:8080`)

| Método | Ruta | Descripción |
|---|---|---|
| `GET` | `/status` | Telemetría leída del sysfs de ev3dev. |
| `POST` | `/move` | `{motor, speed}`. |
| `POST` | `/stop` | `{motor}` opcional. |
| `GET` | `/?cmd=...` | Interfaz heredada por query params que usa el backend en Rust. |

Nota sobre unidades: los comandos van en **porcentaje** (−100…100), mientras
que la telemetría devuelve la velocidad medida en **rpm**. En el dashboard, el
slider es la consigna en % y la cifra sobre él es la medida en rpm.

## Compilación

```bash
npm run build      # genera dist/
```

## Estado del proyecto

En migración desde la arquitectura de escritorio (Tauri + Rust) hacia web
(FastAPI + React). Todavía no tienen equivalente en FastAPI las funciones que
[`src-tauri/src/commands.rs`](src-tauri/src/commands.rs) resolvía por SSH: la
terminal remota, la subida y ejecución de programas y el cambio de IP desde la
interfaz. Esos controles siguen presentes en la UI pero son maquetas.
