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
| Servidor del robot | [`api_simulador/main.py`](api_simulador/main.py) | 8080 | Corre **dentro** del ladrillo. Mueve los motores con `ev3dev2` y lee sensores y batería del sysfs de ev3dev. |
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

1. Copia [`api_simulador/main.py`](api_simulador/main.py) al ladrillo (que debe
   tener [ev3dev](https://www.ev3dev.org/) instalado) y ejecútalo allí:

   ```bash
   python3 main.py     # escucha en el puerto 8080
   ```

2. En el PC, arranca el puente apuntando a la IP del robot:

   ```bash
   EV3_IP=192.168.1.50 python ev3_backend/ev3_bridge.py
   ```

   Sin `EV3_IP`, el puente usa el valor por defecto definido en
   [`ev3_bridge.py`](ev3_backend/ev3_bridge.py). También acepta `EV3_PORT`.

3. Arranca el frontend con `npm run dev`.

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
