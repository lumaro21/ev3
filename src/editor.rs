use std::sync::{Arc, Mutex};
use eframe::egui::{self, Color32, RichText, Rounding, Vec2, Stroke};
use crate::state::Ev3State;
use crate::connection::{SharedConn, ensure_connected};

// ─── Lenguaje ─────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub enum Language {
    Python,
    Bash,
}

impl Language {
    fn label(&self) -> &'static str {
        match self { Language::Python => "Python", Language::Bash => "Bash" }
    }
    fn extension(&self) -> &'static str {
        match self { Language::Python => "py", Language::Bash => "sh" }
    }
    fn run_cmd(&self, path: &str) -> String {
        match self {
            Language::Python => format!("python3 {}", path),
            Language::Bash   => format!("bash {}", path),
        }
    }
}

// ─── Árbol de archivos ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Vec<FileNode>,
    pub expanded: bool,
}

// ─── Autocompletado ───────────────────────────────────────────────────────────

/// Comandos bash comunes en el EV3 para autocompletar
const BASH_COMPLETIONS: &[&str] = &[
    "ls", "ls -la", "ls /sys/class/tacho-motor/",
    "ls /sys/class/lego-sensor/",
    "cat", "echo", "python3", "bash",
    "cd /home/robot", "cd /sys/class/tacho-motor/",
    "kill", "ps aux", "top",
    "echo run-forever >", "echo stop >",
    "cat /sys/class/power_supply/lego-ev3-battery/voltage_now",
];

const PYTHON_COMPLETIONS: &[&str] = &[
    "import ev3dev2",
    "from ev3dev2.motor import LargeMotor, OUTPUT_A, OUTPUT_B",
    "from ev3dev2.sensor import INPUT_1",
    "from ev3dev2.sensor.lego import TouchSensor, UltrasonicSensor, ColorSensor, GyroSensor",
    "from ev3dev2.display import Display",
    "from ev3dev2.sound import Sound",
    "motor = LargeMotor(OUTPUT_A)",
    "motor.on_for_seconds(speed=50, seconds=2)",
    "motor.on(speed=50)",
    "motor.off()",
    "sensor = TouchSensor(INPUT_1)",
    "sensor.is_pressed",
    "sensor = UltrasonicSensor(INPUT_1)",
    "sensor.distance_centimeters",
    "import time",
    "time.sleep(1)",
    "print()",
];

fn get_completions(input: &str, lang: &Language) -> Vec<String> {
    if input.is_empty() { return vec![]; }
    let input_lower = input.to_lowercase();
    let pool = match lang {
        Language::Python => PYTHON_COMPLETIONS,
        Language::Bash   => BASH_COMPLETIONS,
    };
    pool.iter()
        .filter(|c| c.to_lowercase().starts_with(&input_lower) && **c != input)
        .map(|c| c.to_string())
        .take(6)
        .collect()
}

// ─── Estado del editor ────────────────────────────────────────────────────────

pub struct EditorState {
    pub open: bool,
    pub code: String,
    pub language: Language,
    pub console_output: Arc<Mutex<String>>,
    pub running: Arc<Mutex<bool>>,
    pub file_tree: Vec<FileNode>,
    pub tree_loading: Arc<Mutex<bool>>,
    pub save_name: String,
    pub save_dir: String,
    pub last_pid: Arc<Mutex<Option<u32>>>,

    // Terminal bash inline
    pub bash_input: String,
    pub bash_running: Arc<Mutex<bool>>,

    // Historial de comandos (bash inline)
    pub history: Vec<String>,
    pub history_idx: Option<usize>, // None = input actual, Some(i) = navegando historial
    pub history_buffer: String,     // guarda el input actual mientras navegas el historial

    // Autocompletado
    pub completions: Vec<String>,
    pub completion_idx: Option<usize>,

    // Directorio de trabajo actual en el EV3
    pub cwd: Arc<Mutex<String>>,

    // Async internos
    tree_result: Option<Arc<Mutex<Vec<FileNode>>>>,
    pending_load: Option<Arc<Mutex<String>>>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            open: false,
            code: "# Escribe tu programa aqui\nprint('Hola desde el EV3!')\n".to_string(),
            language: Language::Python,
            console_output: Arc::new(Mutex::new(String::new())),
            running: Arc::new(Mutex::new(false)),
            file_tree: vec![],
            tree_loading: Arc::new(Mutex::new(false)),
            save_name: "programa.py".to_string(),
            save_dir: "/home/robot".to_string(),
            last_pid: Arc::new(Mutex::new(None)),
            bash_input: String::new(),
            bash_running: Arc::new(Mutex::new(false)),
            history: vec![],
            history_idx: None,
            history_buffer: String::new(),
            completions: vec![],
            completion_idx: None,
            cwd: Arc::new(Mutex::new("/home/robot".to_string())),
            tree_result: None,
            pending_load: None,
        }
    }
}

impl EditorState {
    // ── Árbol ─────────────────────────────────────────────────────────────────

    pub fn refresh_tree(&mut self, ev3_state: &Arc<Mutex<Ev3State>>, shared_conn: &SharedConn) {
        let loading    = self.tree_loading.clone();
        let tree_out: Arc<Mutex<Vec<FileNode>>> = Arc::new(Mutex::new(vec![]));
        let tree_clone = tree_out.clone();
        let state_ref  = ev3_state.clone();
        let conn_ref   = shared_conn.clone();

        *loading.lock().unwrap() = true;
        self.tree_result = Some(tree_out);

        std::thread::spawn(move || {
            let (ip, user, pass) = get_creds(&state_ref);
            if ensure_connected(&conn_ref, &ip, &user, &pass) {
                let guard = conn_ref.lock().unwrap();
                if let Some(conn) = guard.as_ref() {
                    let nodes = load_dir(conn, "/home/robot", 0);
                    *tree_clone.lock().unwrap() = nodes;
                }
            }
            *loading.lock().unwrap() = false;
        });
    }

    pub fn poll_tree_result(&mut self) {
        let new_nodes = if let Some(ref arc) = self.tree_result {
            if !*self.tree_loading.lock().unwrap() {
                arc.try_lock().ok().and_then(|nodes| {
                    if !nodes.is_empty() { Some(nodes.clone()) } else { None }
                })
            } else { None }
        } else { None };

        if let Some(nodes) = new_nodes {
            self.file_tree = nodes;
            self.tree_result = None;
        }
    }

    fn poll_pending_load(&mut self) {
        let mut done = false;
        if let Some(ref arc) = self.pending_load {
            if let Ok(content) = arc.try_lock() {
                if !content.is_empty() {
                    self.code = content.clone();
                    done = true;
                }
            }
        }
        if done { self.pending_load = None; }
    }

    // ── Historial ─────────────────────────────────────────────────────────────

    fn history_up(&mut self) {
        if self.history.is_empty() { return; }
        match self.history_idx {
            None => {
                // Guardar el input actual y ir al último comando
                self.history_buffer = self.bash_input.clone();
                self.history_idx    = Some(self.history.len() - 1);
                self.bash_input     = self.history[self.history.len() - 1].clone();
            }
            Some(0) => {} // ya estamos en el más antiguo
            Some(i) => {
                self.history_idx = Some(i - 1);
                self.bash_input  = self.history[i - 1].clone();
            }
        }
        self.completions = vec![];
    }

    fn history_down(&mut self) {
        match self.history_idx {
            None => {}
            Some(i) if i + 1 >= self.history.len() => {
                // Volver al input que tenía el usuario
                self.history_idx = None;
                self.bash_input  = self.history_buffer.clone();
            }
            Some(i) => {
                self.history_idx = Some(i + 1);
                self.bash_input  = self.history[i + 1].clone();
            }
        }
        self.completions = vec![];
    }

    fn push_history(&mut self, cmd: &str) {
        let cmd = cmd.trim().to_string();
        if cmd.is_empty() { return; }
        // No duplicar el último comando
        if self.history.last().map(|s| s.as_str()) != Some(&cmd) {
            self.history.push(cmd);
            // Limitar a 100 entradas
            if self.history.len() > 100 {
                self.history.remove(0);
            }
        }
        self.history_idx    = None;
        self.history_buffer = String::new();
    }

    // ── Autocompletado ────────────────────────────────────────────────────────

    fn update_completions(&mut self) {
        self.completions    = get_completions(&self.bash_input, &self.language);
        self.completion_idx = None;
    }

    fn apply_completion(&mut self, idx: usize) {
        if let Some(c) = self.completions.get(idx) {
            self.bash_input  = c.clone();
            self.completions = vec![];
            self.completion_idx = None;
        }
    }

    fn tab_complete(&mut self) {
        if self.completions.is_empty() {
            self.update_completions();
            if self.completions.len() == 1 {
                // Solo hay una opción — aplicar directamente
                let c = self.completions[0].clone();
                self.bash_input  = c;
                self.completions = vec![];
            }
        } else {
            // Ciclar entre opciones
            let next = self.completion_idx.map(|i| (i + 1) % self.completions.len()).unwrap_or(0);
            self.completion_idx = Some(next);
            let c = self.completions[next].clone();
            self.bash_input = c;
        }
    }

    // ── Ejecutar comando bash inline ──────────────────────────────────────────

    pub fn run_bash_cmd(&mut self, ev3_state: &Arc<Mutex<Ev3State>>, shared_conn: &SharedConn) {
        let cmd = self.bash_input.trim().to_string();
        if cmd.is_empty() || *self.bash_running.lock().unwrap() { return; }

        self.push_history(&cmd);
        self.bash_input  = String::new();
        self.completions = vec![];

        let output    = self.console_output.clone();
        let running   = self.bash_running.clone();
        let cwd       = self.cwd.clone();
        let state_ref = ev3_state.clone();
        let conn_ref  = shared_conn.clone();

        let current_dir = cwd.lock().unwrap().clone();

        // Mostrar prompt con directorio actual
        {
            let mut out = output.lock().unwrap();
            out.push_str(&format!("\n{}$ {}\n", current_dir, cmd));
        }
        *running.lock().unwrap() = true;

        std::thread::spawn(move || {
            let (ip, user, pass) = get_creds(&state_ref);
            if !ensure_connected(&conn_ref, &ip, &user, &pass) {
                output.lock().unwrap().push_str("Sin conexion\n");
                *running.lock().unwrap() = false;
                return;
            }

            let guard = conn_ref.lock().unwrap();
            let conn  = match guard.as_ref() {
                Some(c) => c,
                None    => { *running.lock().unwrap() = false; return; }
            };

            // Si es un cd, resolverlo y actualizar cwd
            if cmd.starts_with("cd") {
                let target = cmd.trim_start_matches("cd").trim();
                let new_dir = if target.is_empty() {
                    "/home/robot".to_string()
                } else if target.starts_with('/') {
                    target.to_string()
                } else {
                    format!("{}/{}", current_dir.trim_end_matches('/'), target)
                };

                // Verificar que el directorio existe
                match conn.exec(&format!("cd {} 2>&1 && pwd", new_dir)) {
                    Ok(resolved) if !resolved.starts_with("bash:") && !resolved.contains("No such") => {
                        let resolved = resolved.trim().to_string();
                        *cwd.lock().unwrap() = resolved.clone();
                        output.lock().unwrap().push_str(&format!("{}\n", resolved));
                    }
                    Ok(err) => { output.lock().unwrap().push_str(&format!("{}\n", err)); }
                    Err(e)  => { output.lock().unwrap().push_str(&format!("Error: {}\n", e)); }
                }
            } else {
                // Ejecutar en el directorio actual
                let full_cmd = format!("cd {} 2>/dev/null; {}", current_dir, cmd);
                match conn.exec(&full_cmd) {
                    Ok(out) => {
                        let mut o = output.lock().unwrap();
                        if out.is_empty() {
                            o.push_str("(sin salida)\n");
                        } else {
                            o.push_str(&format!("{}\n", out));
                        }
                    }
                    Err(e) => { output.lock().unwrap().push_str(&format!("Error: {}\n", e)); }
                }
            }

            *running.lock().unwrap() = false;
        });
    }

    // ── Ejecutar archivo completo ─────────────────────────────────────────────

    pub fn run_code(&mut self, ev3_state: &Arc<Mutex<Ev3State>>, shared_conn: &SharedConn) {
        if *self.running.lock().unwrap() { return; }

        let output    = self.console_output.clone();
        let running   = self.running.clone();
        let last_pid  = self.last_pid.clone();
        let code      = self.code.clone();
        let lang      = self.language.clone();
        let dir       = self.save_dir.clone();
        let name      = self.save_name.clone();
        let state_ref = ev3_state.clone();
        let conn_ref  = shared_conn.clone();

        *output.lock().unwrap()   = "Conectando...\n".to_string();
        *running.lock().unwrap()  = true;
        *last_pid.lock().unwrap() = None;

        std::thread::spawn(move || {
            let (ip, user, pass) = get_creds(&state_ref);

            if !ensure_connected(&conn_ref, &ip, &user, &pass) {
                *output.lock().unwrap() = "Sin conexion al EV3\n".to_string();
                *running.lock().unwrap() = false;
                return;
            }

            let remote_path = format!("{}/{}", dir.trim_end_matches('/'), name);

            {
                let guard = conn_ref.lock().unwrap();
                if let Some(conn) = guard.as_ref() {
                    match conn.write_file(&remote_path, &code) {
                        Ok(_)  => *output.lock().unwrap() = format!("Guardado en {}\nEjecutando...\n", remote_path),
                        Err(e) => {
                            *output.lock().unwrap() = format!("Error al subir: {}\n", e);
                            *running.lock().unwrap() = false;
                            return;
                        }
                    }
                }
            }

            let bg_cmd  = format!("nohup {} > /tmp/ev3_out.log 2>&1 & echo $!", lang.run_cmd(&remote_path));
            let pid_str = {
                let guard = conn_ref.lock().unwrap();
                guard.as_ref().and_then(|conn| conn.exec(&bg_cmd).ok()).unwrap_or_default()
            };
            let pid_str = pid_str.trim().to_string();

            if let Ok(pid) = pid_str.parse::<u32>() {
                *last_pid.lock().unwrap() = Some(pid);
                *output.lock().unwrap() += &format!("PID: {}\n", pid);
            } else {
                *output.lock().unwrap() += "No se pudo obtener el PID\n";
                *running.lock().unwrap() = false;
                return;
            }

            loop {
                std::thread::sleep(std::time::Duration::from_millis(200));

                if last_pid.lock().unwrap().is_none() {
                    *output.lock().unwrap() += "\nDetenido por el usuario\n";
                    break;
                }

                let guard = conn_ref.lock().unwrap();
                let conn  = match guard.as_ref() { Some(c) => c, None => break };

                let still_running = conn
                    .exec(&format!("kill -0 {} 2>/dev/null && echo yes || echo no", pid_str))
                    .unwrap_or_else(|_| "no".to_string());
                let log = conn.exec("cat /tmp/ev3_out.log").unwrap_or_default();
                drop(guard);

                *output.lock().unwrap() = format!("PID: {}\n{}", pid_str, log);

                if still_running.trim() == "no" {
                    *output.lock().unwrap() += "\n✓ Finalizado\n";
                    *last_pid.lock().unwrap() = None;
                    break;
                }
            }

            *running.lock().unwrap() = false;
        });
    }

    pub fn stop_program(&mut self, ev3_state: &Arc<Mutex<Ev3State>>, shared_conn: &SharedConn) {
        let pid_opt = *self.last_pid.lock().unwrap();
        if let Some(pid) = pid_opt {
            *self.last_pid.lock().unwrap() = None;

            let output    = self.console_output.clone();
            let running   = self.running.clone();
            let state_ref = ev3_state.clone();
            let conn_ref  = shared_conn.clone();

            std::thread::spawn(move || {
                let (ip, user, pass) = get_creds(&state_ref);
                if ensure_connected(&conn_ref, &ip, &user, &pass) {
                    let guard = conn_ref.lock().unwrap();
                    if let Some(conn) = guard.as_ref() {
                        let _ = conn.exec(&format!("kill -- -{} 2>/dev/null || kill {} 2>/dev/null", pid, pid));
                        *output.lock().unwrap() += &format!("\nDetenido (PID {})\n", pid);
                    }
                }
                *running.lock().unwrap() = false;
            });
        }
    }

    pub fn save_to_ev3(&mut self, ev3_state: &Arc<Mutex<Ev3State>>, shared_conn: &SharedConn) {
        let output    = self.console_output.clone();
        let code      = self.code.clone();
        let dir       = self.save_dir.clone();
        let name      = self.save_name.clone();
        let state_ref = ev3_state.clone();
        let conn_ref  = shared_conn.clone();

        *output.lock().unwrap() = "Guardando...\n".to_string();

        std::thread::spawn(move || {
            let (ip, user, pass) = get_creds(&state_ref);
            if ensure_connected(&conn_ref, &ip, &user, &pass) {
                let guard = conn_ref.lock().unwrap();
                if let Some(conn) = guard.as_ref() {
                    let remote_path = format!("{}/{}", dir.trim_end_matches('/'), name);
                    match conn.write_file(&remote_path, &code) {
                        Ok(_)  => *output.lock().unwrap() = format!("✓ Guardado en {}\n", remote_path),
                        Err(e) => *output.lock().unwrap() = format!("Error: {}\n", e),
                    }
                }
            } else {
                *output.lock().unwrap() = "Sin conexion\n".to_string();
            }
        });
    }

    fn open_remote_file(&mut self, path: &str, name: &str, ev3_state: &Arc<Mutex<Ev3State>>, shared_conn: &SharedConn) {
        let result    = Arc::new(Mutex::new(String::new()));
        let result_cl = result.clone();
        let path_cl   = path.to_string();
        let output    = self.console_output.clone();
        let state_ref = ev3_state.clone();
        let conn_ref  = shared_conn.clone();

        self.pending_load = Some(result);
        self.save_name    = name.to_string();

        std::thread::spawn(move || {
            let (ip, user, pass) = get_creds(&state_ref);
            if ensure_connected(&conn_ref, &ip, &user, &pass) {
                let guard = conn_ref.lock().unwrap();
                if let Some(conn) = guard.as_ref() {
                    match conn.read_file(&path_cl) {
                        Ok(content) => *result_cl.lock().unwrap() = content,
                        Err(e)      => *output.lock().unwrap() = format!("No se pudo abrir: {}\n", e),
                    }
                }
            }
        });
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn get_creds(state: &Arc<Mutex<Ev3State>>) -> (String, String, String) {
    let s = state.lock().unwrap();
    (s.ip.clone(), "robot".to_string(), "maker".to_string())
}

fn load_dir(conn: &crate::connection::Ev3Connection, path: &str, depth: usize) -> Vec<FileNode> {
    if depth > 3 { return vec![]; }
    let output = conn.exec(&format!("ls -1p {} 2>/dev/null", path)).unwrap_or_default();
    let mut nodes = vec![];
    for entry in output.lines() {
        let entry  = entry.trim();
        if entry.is_empty() { continue; }
        let is_dir = entry.ends_with('/');
        let name   = entry.trim_end_matches('/').to_string();
        let full   = format!("{}/{}", path.trim_end_matches('/'), name);
        let children = if is_dir && depth < 2 { load_dir(conn, &full, depth + 1) } else { vec![] };
        nodes.push(FileNode { name, path: full, is_dir, children, expanded: false });
    }
    nodes
}

// ─── Ventana principal ────────────────────────────────────────────────────────

pub fn show_editor_window(
    ctx: &egui::Context,
    editor: &mut EditorState,
    ev3_state: &Arc<Mutex<Ev3State>>,
    shared_conn: &SharedConn,
) {
    if !editor.open { return; }

    editor.poll_tree_result();
    editor.poll_pending_load();

    let mut open = editor.open;

    egui::Window::new("  >_  Terminal EV3")
        .id(egui::Id::new("ev3_editor"))
        .open(&mut open)
        .resizable(true)
        .default_size([960.0, 660.0])
        .min_size([700.0, 440.0])
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(Color32::from_rgb(18, 18, 24))
                .stroke(Stroke::new(1.0, Color32::from_rgb(55, 58, 78)))
                .rounding(Rounding::same(10.0))
        )
        .show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(Color32::from_rgb(210, 215, 240));

            // ── Toolbar ───────────────────────────────────────────────────
            egui::Frame::none()
                .fill(Color32::from_rgb(22, 22, 30))
                .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Lenguaje:").color(Color32::from_rgb(120, 125, 155)).size(11.0));
                        for lang in [Language::Python, Language::Bash] {
                            let selected = editor.language == lang;
                            let btn = egui::Button::new(
                                RichText::new(lang.label())
                                    .color(if selected { Color32::from_rgb(80, 160, 255) } else { Color32::from_rgb(120, 125, 155) })
                                    .size(11.0)
                            )
                            .fill(if selected { Color32::from_rgb(20, 45, 88) } else { Color32::TRANSPARENT })
                            .stroke(Stroke::new(if selected { 1.0 } else { 0.0 }, Color32::from_rgb(50, 100, 190)));

                            if ui.add(btn).clicked() && editor.language != lang {
                                let ext = lang.extension();
                                editor.language = lang;
                                if let Some(base) = editor.save_name.split('.').next().map(|s| s.to_string()) {
                                    editor.save_name = format!("{}.{}", base, ext);
                                }
                                editor.completions = vec![];
                            }
                        }

                        ui.separator();

                        ui.label(RichText::new("Archivo:").color(Color32::from_rgb(120, 125, 155)).size(11.0));
                        ui.add(egui::TextEdit::singleline(&mut editor.save_name).desired_width(130.0).font(egui::TextStyle::Monospace));
                        ui.label(RichText::new("en:").color(Color32::from_rgb(120, 125, 155)).size(11.0));
                        ui.add(egui::TextEdit::singleline(&mut editor.save_dir).desired_width(130.0).font(egui::TextStyle::Monospace));

                        ui.separator();

                        let is_running  = *editor.running.lock().unwrap();
                        let has_pid     = editor.last_pid.lock().unwrap().is_some();

                        if ui.add_enabled(!is_running,
                            egui::Button::new(RichText::new(if is_running { "Ejecutando..." } else { "▶ Run" }).color(Color32::from_rgb(80, 220, 120)).size(11.0))
                                .fill(Color32::from_rgb(18, 46, 24)).stroke(Stroke::new(1.0, Color32::from_rgb(35, 125, 60)))
                        ).clicked() { editor.run_code(ev3_state, shared_conn); }

                        if ui.add_enabled(has_pid,
                            egui::Button::new(RichText::new("■ Stop")
                                .color(if has_pid { Color32::from_rgb(255, 80, 80) } else { Color32::from_rgb(100, 60, 60) }).size(11.0))
                                .fill(Color32::from_rgb(50, 18, 18)).stroke(Stroke::new(1.0, if has_pid { Color32::from_rgb(180, 40, 40) } else { Color32::from_rgb(80, 35, 35) }))
                        ).on_hover_text("Detener programa").clicked() { editor.stop_program(ev3_state, shared_conn); }

                        if ui.add(egui::Button::new(RichText::new("💾 Guardar").color(Color32::from_rgb(110, 175, 255)).size(11.0))
                            .fill(Color32::from_rgb(18, 36, 66)).stroke(Stroke::new(1.0, Color32::from_rgb(42, 90, 185)))
                        ).clicked() { editor.save_to_ev3(ev3_state, shared_conn); }

                        if ui.add(egui::Button::new(RichText::new("Limpiar").color(Color32::from_rgb(160, 95, 95)).size(11.0))
                            .fill(Color32::TRANSPARENT)
                        ).clicked() { *editor.console_output.lock().unwrap() = String::new(); }
                    });
                });

            ui.add_space(4.0);
            let available = ui.available_size();

            ui.vertical(|ui| {
                    let right_w   = available.x;
                    let editor_h  = (available.y - 20.0) * 0.52;
                    let console_h = (available.y - 20.0) * 0.44;

                    // Editor de código
                    egui::Frame::none()
                        .fill(Color32::from_rgb(13, 13, 19))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(36, 38, 54)))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::same(6.0))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(right_w, editor_h));
                            ui.set_max_size(Vec2::new(right_w, editor_h));
                            ui.label(RichText::new(format!("Editor — {}", editor.language.label())).color(Color32::from_rgb(100, 105, 135)).size(10.0));
                            ui.add_space(2.0);
                            egui::ScrollArea::both()
                                .id_source("code_scroll")
                                .max_height(editor_h - 26.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut editor.code)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(right_w - 14.0)
                                            .desired_rows(18)
                                            .code_editor()
                                            .text_color(Color32::from_rgb(200, 210, 240))
                                            .frame(false)
                                    );
                                });
                        });

                    ui.add_space(4.0);

                    // Consola + input bash
                    egui::Frame::none()
                        .fill(Color32::from_rgb(10, 12, 15))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(30, 55, 35)))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::same(6.0))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(right_w, console_h));
                            ui.set_max_size(Vec2::new(right_w, console_h));

                            // Título
                            ui.horizontal(|ui| {
                                let is_running = *editor.running.lock().unwrap();
                                ui.label(RichText::new("●")
                                    .color(if is_running { Color32::from_rgb(80, 220, 80) } else { Color32::from_rgb(60, 60, 60) })
                                    .size(10.0));
                                ui.label(RichText::new("Consola").color(Color32::from_rgb(65, 165, 65)).size(10.0));
                                if is_running {
                                    ui.label(RichText::new("— ejecutando...").color(Color32::from_rgb(80, 180, 80)).size(10.0));
                                }
                            });

                            ui.add_space(2.0);

                            // Salida de consola
                            let console_text = editor.console_output.lock().unwrap().clone();
                            egui::ScrollArea::vertical()
                                .id_source("console_scroll")
                                .max_height(console_h - 62.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut console_text.clone())
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(right_w - 14.0)
                                            .text_color(Color32::from_rgb(75, 210, 90))
                                            .frame(false)
                                            .interactive(false)
                                    );
                                });

                            ui.add_space(4.0);
                            ui.add(egui::Separator::default().spacing(2.0));
                            ui.add_space(2.0);

                            // ── Input bash con historial y autocompletado ──
                            draw_bash_input(ui, editor, ev3_state, shared_conn, right_w);
                        });
                });
            });

    editor.open = open;
}

// ─── Input bash inline ────────────────────────────────────────────────────────

fn draw_bash_input(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    ev3_state: &Arc<Mutex<Ev3State>>,
    shared_conn: &SharedConn,
    width: f32,
) {
    // Sugerencias de autocompletado (encima del input)
    if !editor.completions.is_empty() {
        egui::Frame::none()
            .fill(Color32::from_rgb(22, 26, 32))
            .stroke(Stroke::new(1.0, Color32::from_rgb(55, 60, 80)))
            .rounding(Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(6.0, 3.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let completions = editor.completions.clone();
                    for (i, c) in completions.iter().enumerate() {
                        let selected = editor.completion_idx == Some(i);
                        let btn = egui::Button::new(
                            RichText::new(c)
                                .color(if selected { Color32::from_rgb(80, 200, 255) } else { Color32::from_rgb(140, 160, 200) })
                                .size(10.0)
                                .monospace()
                        )
                        .fill(if selected { Color32::from_rgb(20, 45, 70) } else { Color32::TRANSPARENT })
                        .frame(false);

                        if ui.add(btn).clicked() {
                            editor.apply_completion(i);
                        }
                    }
                    ui.label(RichText::new("  Tab↹ para ciclar").color(Color32::from_rgb(70, 75, 100)).size(9.0));
                });
            });
        ui.add_space(2.0);
    }

    // Historial hint
    if let Some(idx) = editor.history_idx {
        ui.label(
            RichText::new(format!("historial [{}/{}] — ↑↓ para navegar", idx + 1, editor.history.len()))
                .color(Color32::from_rgb(100, 105, 130))
                .size(9.0)
        );
    }

    ui.horizontal(|ui| {
        // Prompt con directorio actual
        let cwd_display = {
            let cwd = editor.cwd.lock().unwrap();
            // Acortar /home/robot → ~ para que no sea tan largo
            cwd.replace("/home/robot", "~")
        };
        ui.label(
            RichText::new(format!("{}$ ", cwd_display))
                .color(Color32::from_rgb(80, 200, 120))
                .size(11.0)
                .monospace()
        );

        // Campo de input
        let bash_running = *editor.bash_running.lock().unwrap();
        let input_resp = ui.add_enabled(
            !bash_running,
            egui::TextEdit::singleline(&mut editor.bash_input)
                .font(egui::TextStyle::Monospace)
                .desired_width(width - 80.0)
                .text_color(Color32::from_rgb(200, 215, 240))
                .frame(false)
                .hint_text("escribe un comando bash... (Tab=completar, ↑↓=historial)")
        );

        // Botón enviar
        let send_btn = egui::Button::new(
            RichText::new(if bash_running { "…" } else { "↵" })
                .color(Color32::from_rgb(80, 200, 120))
                .size(12.0)
        )
        .fill(Color32::TRANSPARENT);

        if ui.add_enabled(!bash_running, send_btn).clicked() {
            editor.run_bash_cmd(ev3_state, shared_conn);
        }

        // Procesar teclas especiales cuando el input tiene foco
        if input_resp.has_focus() {
            let (enter, up, down, tab) = ui.input(|i| (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Tab),
            ));

            if enter { editor.run_bash_cmd(ev3_state, shared_conn); }
            if up    { editor.history_up(); }
            if down  { editor.history_down(); }
            if tab   { editor.tab_complete(); }

            // Actualizar sugerencias mientras escribe (si no está navegando historial)
            if input_resp.changed() && editor.history_idx.is_none() {
                editor.update_completions();
            }
        }

        
    });
}

// ─── Árbol de archivos ────────────────────────────────────────────────────────

fn render_tree_flat(
    ui: &mut egui::Ui,
    nodes: &mut Vec<FileNode>,
    depth: usize,
    open_file: &mut Option<(String, String)>,
) {
    for node in nodes.iter_mut() {
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 10.0);
            if node.is_dir {
                let arrow = if node.expanded { "v" } else { ">" };
                if ui.add(
                    egui::Button::new(RichText::new(format!("{} DIR {}", arrow, node.name)).color(Color32::from_rgb(180, 180, 100)).size(10.0))
                        .fill(Color32::TRANSPARENT).frame(false)
                ).clicked() { node.expanded = !node.expanded; }
            } else {
                let ext = node.name.split('.').last().unwrap_or("");
                let tag = match ext { "py" => "[py]", "sh" => "[sh]", _ => "[  ]" };
                if ui.add(
                    egui::Button::new(RichText::new(format!("   {} {}", tag, node.name)).color(Color32::from_rgb(170, 185, 225)).size(10.0))
                        .fill(Color32::TRANSPARENT).frame(false)
                ).on_hover_text(&node.path).clicked() {
                    *open_file = Some((node.path.clone(), node.name.clone()));
                }
            }
        });
        if node.is_dir && node.expanded {
            render_tree_flat(ui, &mut node.children, depth + 1, open_file);
        }
    }
}