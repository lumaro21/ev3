use std::sync::{Arc, Mutex};
use eframe::egui::{self, Color32, RichText, Rounding, Vec2, Stroke};
use crate::state::Ev3State;
use crate::connection::{SharedConn, ensure_connected};

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

#[derive(Clone)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Vec<FileNode>,
    pub expanded: bool,
}

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
            tree_result: None,
            pending_load: None,
        }
    }
}

impl EditorState {
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

            // Subir el archivo — reutilizando la sesión abierta
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

            // Lanzar en background con nohup para capturar PID
            let bg_cmd = format!(
                "nohup {} > /tmp/ev3_out.log 2>&1 & echo $!",
                lang.run_cmd(&remote_path)
            );

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

            // Polling del log en tiempo real — sin reconectar, misma sesión
            loop {
                std::thread::sleep(std::time::Duration::from_millis(200)); // actualizar cada 200ms

                // Si Stop fue presionado (PID limpiado), salir
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
            *self.last_pid.lock().unwrap() = None; // el loop de run_code detecta esto y sale

            let output   = self.console_output.clone();
            let running  = self.running.clone();
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

// ─── Ventana ──────────────────────────────────────────────────────────────────

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
        .default_size([920.0, 620.0])
        .min_size([700.0, 400.0])
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(Color32::from_rgb(18, 18, 24))
                .stroke(Stroke::new(1.0, Color32::from_rgb(55, 58, 78)))
                .rounding(Rounding::same(10.0))
        )
        .show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(Color32::from_rgb(210, 215, 240));

            // Toolbar
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
                            }
                        }

                        ui.separator();

                        ui.label(RichText::new("Archivo:").color(Color32::from_rgb(120, 125, 155)).size(11.0));
                        ui.add(egui::TextEdit::singleline(&mut editor.save_name).desired_width(130.0).font(egui::TextStyle::Monospace));
                        ui.label(RichText::new("en:").color(Color32::from_rgb(120, 125, 155)).size(11.0));
                        ui.add(egui::TextEdit::singleline(&mut editor.save_dir).desired_width(130.0).font(egui::TextStyle::Monospace));

                        ui.separator();

                        let is_running = *editor.running.lock().unwrap();
                        let has_pid    = editor.last_pid.lock().unwrap().is_some();

                        // Run
                        if ui.add_enabled(
                            !is_running,
                            egui::Button::new(
                                RichText::new(if is_running { "Ejecutando..." } else { "▶ Run" })
                                    .color(Color32::from_rgb(80, 220, 120)).size(11.0)
                            )
                            .fill(Color32::from_rgb(18, 46, 24))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(35, 125, 60)))
                        ).clicked() {
                            editor.run_code(ev3_state, shared_conn);
                        }

                        // Stop
                        if ui.add_enabled(
                            has_pid,
                            egui::Button::new(
                                RichText::new("■ Stop")
                                    .color(if has_pid { Color32::from_rgb(255, 80, 80) } else { Color32::from_rgb(100, 60, 60) })
                                    .size(11.0)
                            )
                            .fill(Color32::from_rgb(50, 18, 18))
                            .stroke(Stroke::new(1.0, if has_pid { Color32::from_rgb(180, 40, 40) } else { Color32::from_rgb(80, 35, 35) }))
                        ).on_hover_text("Detener el programa en el EV3")
                         .clicked()
                        {
                            editor.stop_program(ev3_state, shared_conn);
                        }

                        // Guardar
                        if ui.add(
                            egui::Button::new(RichText::new("💾 Guardar").color(Color32::from_rgb(110, 175, 255)).size(11.0))
                                .fill(Color32::from_rgb(18, 36, 66))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(42, 90, 185)))
                        ).clicked() {
                            editor.save_to_ev3(ev3_state, shared_conn);
                        }

                        // Limpiar
                        if ui.add(
                            egui::Button::new(RichText::new("Limpiar").color(Color32::from_rgb(160, 95, 95)).size(11.0))
                                .fill(Color32::TRANSPARENT)
                        ).clicked() {
                            *editor.console_output.lock().unwrap() = String::new();
                        }
                    });
                });

            ui.add_space(4.0);
            let available = ui.available_size();

            ui.horizontal(|ui| {
                // Panel izquierdo: árbol
                egui::Frame::none()
                    .fill(Color32::from_rgb(19, 19, 27))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(36, 38, 54)))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        let panel_h = available.y - 20.0;
                        ui.set_min_size(Vec2::new(182.0, panel_h));
                        ui.set_max_size(Vec2::new(182.0, panel_h));

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Archivos EV3").color(Color32::from_rgb(120, 125, 155)).size(10.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let loading = *editor.tree_loading.lock().unwrap();
                                if ui.add_enabled(
                                    !loading,
                                    egui::Button::new(
                                        RichText::new(if loading { "..." } else { "Cargar" })
                                            .color(Color32::from_rgb(100, 165, 255)).size(10.0)
                                    ).fill(Color32::TRANSPARENT)
                                ).clicked() {
                                    editor.refresh_tree(ev3_state, shared_conn);
                                }
                            });
                        });

                        ui.add_space(3.0);
                        ui.add(egui::Separator::default().spacing(2.0));
                        ui.add_space(2.0);

                        egui::ScrollArea::vertical()
                            .id_source("tree_scroll")
                            .max_height(panel_h - 46.0)
                            .show(ui, |ui| {
                                if editor.file_tree.is_empty() {
                                    ui.label(
                                        RichText::new("Presiona \"Cargar\" para\nver los archivos del EV3")
                                            .color(Color32::from_rgb(65, 70, 95)).size(9.0)
                                    );
                                } else {
                                    let mut open_file: Option<(String, String)> = None;
                                    render_tree_flat(ui, &mut editor.file_tree, 0, &mut open_file);
                                    if let Some((path, name)) = open_file {
                                        editor.open_remote_file(&path, &name, ev3_state, shared_conn);
                                    }
                                }
                            });
                    });

                ui.add_space(4.0);

                // Panel derecho: editor + consola
                ui.vertical(|ui| {
                    let right_w   = available.x - 202.0;
                    let editor_h  = (available.y - 20.0) * 0.60;
                    let console_h = (available.y - 20.0) * 0.37;

                    // Editor
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
                                            .desired_rows(20)
                                            .code_editor()
                                            .text_color(Color32::from_rgb(200, 210, 240))
                                            .frame(false)
                                    );
                                });
                        });

                    ui.add_space(4.0);

                    // Consola
                    egui::Frame::none()
                        .fill(Color32::from_rgb(10, 12, 15))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(30, 55, 35)))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::same(6.0))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(right_w, console_h));
                            ui.set_max_size(Vec2::new(right_w, console_h));

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
                            let console_text = editor.console_output.lock().unwrap().clone();
                            egui::ScrollArea::vertical()
                                .id_source("console_scroll")
                                .max_height(console_h - 30.0)
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
                        });
                });
            });
        });

    editor.open = open;
}

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
                    egui::Button::new(
                        RichText::new(format!("{} DIR {}", arrow, node.name))
                            .color(Color32::from_rgb(180, 180, 100)).size(10.0)
                    ).fill(Color32::TRANSPARENT).frame(false)
                ).clicked() { node.expanded = !node.expanded; }
            } else {
                let ext = node.name.split('.').last().unwrap_or("");
                let tag = match ext { "py" => "[py]", "sh" => "[sh]", _ => "[  ]" };
                if ui.add(
                    egui::Button::new(
                        RichText::new(format!("   {} {}", tag, node.name))
                            .color(Color32::from_rgb(170, 185, 225)).size(10.0)
                    ).fill(Color32::TRANSPARENT).frame(false)
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