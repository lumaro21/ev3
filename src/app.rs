use std::sync::{Arc, Mutex};
use eframe::egui::{self, Color32, RichText, Rounding, Vec2, Stroke};
use crate::state::{Ev3State, MotorCommand, SensorType};

pub struct Ev3App {
    state: Arc<Mutex<Ev3State>>,
}

impl Ev3App {
    pub fn new(state: Arc<Mutex<Ev3State>>) -> Self {
        Self { state }
    }
}

impl eframe::App for Ev3App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(400));

        let state_snapshot = self.state.lock().unwrap().clone();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(10, 10, 12)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(24.0);
                        draw_brick(ui, &state_snapshot, &self.state);
                        ui.add_space(24.0);
                    });
                });
            });
    }
}

fn draw_brick(ui: &mut egui::Ui, state: &Ev3State, shared: &Arc<Mutex<Ev3State>>) {
    egui::Frame::none()
        .fill(Color32::from_rgb(18, 18, 22))
        .rounding(Rounding::same(28.0))
        .inner_margin(egui::Margin::same(3.0))
        .show(ui, |ui| {
            egui::Frame::none()
                .fill(Color32::from_rgb(52, 54, 65))
                .rounding(Rounding::same(26.0))
                .inner_margin(egui::Margin::symmetric(18.0, 20.0))
                .show(ui, |ui| {
                    ui.set_min_width(480.0);
                    ui.set_max_width(480.0);

                    draw_top_stripe(ui);
                    ui.add_space(10.0);
                    draw_screen(ui, state);
                    ui.add_space(14.0);
                    draw_dpad(ui);
                    ui.add_space(16.0);
                    ui.add(egui::Separator::default().spacing(4.0));
                    ui.add_space(8.0);
                    draw_output_ports(ui, state, shared);
                    ui.add_space(10.0);
                    draw_input_ports(ui, state);
                    ui.add_space(6.0);

                    if !state.alerts.is_empty() {
                        ui.add_space(8.0);
                        for alert in &state.alerts {
                            ui.label(
                                RichText::new(alert)
                                    .color(Color32::from_rgb(255, 80, 80))
                                    .size(11.0),
                            );
                        }
                    }
                });
        });
}

fn draw_top_stripe(ui: &mut egui::Ui) {
    egui::Frame::none()
        .fill(Color32::from_rgb(180, 30, 30))
        .rounding(Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(12.0, 5.0))
        .show(ui, |ui| {
            ui.set_min_width(444.0);
            ui.set_max_width(444.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("MINDSTORMS")
                        .color(Color32::from_rgb(255, 200, 200))
                        .size(10.0)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("EV3").color(Color32::WHITE).size(10.0).strong());
                });
            });
        });
}

fn draw_screen(ui: &mut egui::Ui, state: &Ev3State) {
    egui::Frame::none()
        .fill(Color32::from_rgb(20, 20, 25))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::same(5.0))
        .show(ui, |ui| {
            egui::Frame::none()
                .fill(Color32::from_rgb(110, 150, 95))
                .rounding(Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(424.0, 130.0));
                    ui.set_max_width(424.0);

                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("EV3 CONTROLLER")
                                .color(Color32::from_rgb(15, 40, 15))
                                .size(13.0)
                                .strong(),
                        );
                    });

                    ui.add_space(4.0);

                    if state.connected {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("●")
                                    .color(Color32::from_rgb(0, 180, 60))
                                    .size(10.0),
                            );
                            ui.label(
                                RichText::new("Conectado")
                                    .color(Color32::from_rgb(15, 40, 15))
                                    .size(10.0),
                            );
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("●")
                                    .color(Color32::from_rgb(160, 40, 40))
                                    .size(10.0),
                            );
                            ui.label(
                                RichText::new("Sin conexion — enciende el EV3")
                                    .color(Color32::from_rgb(15, 40, 15))
                                    .size(10.0),
                            );
                        });
                    }

                    ui.add_space(5.0);
                    ui.add(egui::Separator::default().spacing(3.0));
                    ui.add_space(3.0);

                    let bat_text = if state.connected {
                        format!("Bateria   {:.2}V", state.battery_voltage)
                    } else {
                        "Bateria   —".to_string()
                    };

                    let sensor_count = state.sensors.len();
                    let motor_count = state.motors.iter().filter(|m| m.connected).count();

                    ui.label(
                        RichText::new(format!(
                            "Sensores  {}",
                            if state.connected { sensor_count.to_string() } else { "—".to_string() }
                        ))
                        .color(Color32::from_rgb(15, 40, 15))
                        .size(10.0),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Motores   {}",
                            if state.connected { motor_count.to_string() } else { "—".to_string() }
                        ))
                        .color(Color32::from_rgb(15, 40, 15))
                        .size(10.0),
                    );
                    ui.label(
                        RichText::new(bat_text)
                            .color(Color32::from_rgb(15, 40, 15))
                            .size(10.0),
                    );
                });
        });
}

fn draw_dpad(ui: &mut egui::Ui) {
    let dark_btn = Color32::from_rgb(28, 28, 35);
    let gray_btn = Color32::from_rgb(75, 78, 92);
    let orange = Color32::from_rgb(215, 100, 0);
    let txt = Color32::WHITE;

    ui.vertical_centered(|ui| {
        ui.horizontal(|ui| {
            ui.add_space(100.0);
            egui::Frame::none()
                .fill(dark_btn)
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::same(2.0))
                .show(ui, |ui| {
                    let _ = ui.add(
                        egui::Button::new(RichText::new("<").color(txt).size(14.0))
                            .fill(Color32::TRANSPARENT)
                            .min_size(Vec2::new(44.0, 30.0))
                            .frame(false),
                    );
                });
            ui.add_space(8.0);
            egui::Frame::none()
                .fill(gray_btn)
                .rounding(Rounding::same(8.0))
                .show(ui, |ui| {
                    let _ = ui.add(
                        egui::Button::new(RichText::new("^").color(txt).size(13.0))
                            .fill(Color32::TRANSPARENT)
                            .min_size(Vec2::new(44.0, 30.0))
                            .frame(false),
                    );
                });
            ui.add_space(8.0);
            egui::Frame::none()
                .fill(dark_btn)
                .rounding(Rounding::same(8.0))
                .show(ui, |ui| {
                    let _ = ui.add(
                        egui::Button::new(RichText::new("OK").color(txt).size(14.0))
                            .fill(Color32::TRANSPARENT)
                            .min_size(Vec2::new(44.0, 30.0))
                            .frame(false),
                    );
                });
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.add_space(114.0);
            egui::Frame::none()
                .fill(gray_btn)
                .rounding(Rounding::same(8.0))
                .show(ui, |ui| {
                    let _ = ui.add(
                        egui::Button::new(RichText::new("<").color(txt).size(13.0))
                            .fill(Color32::TRANSPARENT)
                            .min_size(Vec2::new(36.0, 30.0))
                            .frame(false),
                    );
                });
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(orange)
                .rounding(Rounding::same(22.0))
                .stroke(Stroke::new(2.0, Color32::from_rgb(255, 150, 50)))
                .show(ui, |ui| {
                    let _ = ui.add(
                        egui::Button::new(RichText::new(" ").size(18.0))
                            .fill(Color32::TRANSPARENT)
                            .min_size(Vec2::new(44.0, 44.0))
                            .frame(false),
                    );
                });
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(gray_btn)
                .rounding(Rounding::same(8.0))
                .show(ui, |ui| {
                    let _ = ui.add(
                        egui::Button::new(RichText::new(">").color(txt).size(13.0))
                            .fill(Color32::TRANSPARENT)
                            .min_size(Vec2::new(36.0, 30.0))
                            .frame(false),
                    );
                });
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.add_space(152.0);
            egui::Frame::none()
                .fill(gray_btn)
                .rounding(Rounding::same(8.0))
                .show(ui, |ui| {
                    let _ = ui.add(
                        egui::Button::new(RichText::new("v").color(txt).size(13.0))
                            .fill(Color32::TRANSPARENT)
                            .min_size(Vec2::new(44.0, 30.0))
                            .frame(false),
                    );
                });
        });
    });
}

// ─── Output ports con control de motor ──────────────────────────────────────

fn draw_output_ports(ui: &mut egui::Ui, state: &Ev3State, shared: &Arc<Mutex<Ev3State>>) {
    ui.label(
        RichText::new("OUTPUT PORTS — MOTORS")
            .color(Color32::from_rgb(140, 140, 160))
            .size(9.0),
    );
    ui.add_space(4.0);

    // Necesitamos ancho suficiente para los controles — usamos layout vertical con columnas manuales
    ui.horizontal(|ui| {
        for port in ["outA", "outB", "outC", "outD"] {
            let motor = state.motors.iter().find(|m| m.port == port);
            let connected = motor.map(|m| m.connected).unwrap_or(false);

            let (bg, border, label_color) = if connected {
                (
                    Color32::from_rgb(30, 60, 35),
                    Color32::from_rgb(0, 140, 60),
                    Color32::from_rgb(80, 220, 120),
                )
            } else {
                (
                    Color32::from_rgb(35, 36, 45),
                    Color32::from_rgb(65, 68, 85),
                    Color32::from_rgb(200, 200, 220),
                )
            };

            egui::Frame::none()
                .fill(bg)
                .rounding(Rounding::same(8.0))
                .stroke(Stroke::new(1.0, border))
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(104.0, 52.0));
                    ui.set_max_width(104.0);
                    ui.vertical_centered(|ui| {
                        // Letra del puerto
                        ui.label(
                            RichText::new(port.replace("out", ""))
                                .color(label_color)
                                .size(18.0)
                                .strong(),
                        );

                        if connected {
                            let actual_speed = motor.map(|m| m.speed).unwrap_or(0);
                            ui.label(
                                RichText::new(format!("{}rpm", actual_speed))
                                    .color(Color32::from_rgb(80, 200, 100))
                                    .size(9.0),
                            );

                            // Leer velocidad deseada actual
                            let desired = {
                                shared
                                    .lock()
                                    .unwrap()
                                    .desired_speeds
                                    .get(port)
                                    .copied()
                                    .unwrap_or(0)
                            };
                            let mut desired_mut = desired;

                            // Slider de velocidad: -1050 a 1050 (tacho EV3)
                            let slider = egui::Slider::new(&mut desired_mut, -1050..=1050)
                                .show_value(false)
                                .text("");
                            if ui.add(slider).changed() {
                                let mut s = shared.lock().unwrap();
                                s.desired_speeds.insert(port.to_string(), desired_mut);
                                s.pending_commands.push(MotorCommand::SetSpeed {
                                    port: port.to_string(),
                                    speed: desired_mut,
                                });
                            }

                            // Botones ▶ / ■
                            ui.horizontal(|ui| {
                                let run_btn = ui.add(
                                    egui::Button::new(
                                        RichText::new("▶").color(Color32::from_rgb(80, 220, 120)).size(11.0),
                                    )
                                    .fill(Color32::from_rgb(20, 50, 25))
                                    .min_size(Vec2::new(36.0, 18.0)),
                                );
                                if run_btn.clicked() {
                                    let mut s = shared.lock().unwrap();
                                    s.pending_commands.push(MotorCommand::Run {
                                        port: port.to_string(),
                                    });
                                }

                                let stop_btn = ui.add(
                                    egui::Button::new(
                                        RichText::new("■").color(Color32::from_rgb(220, 80, 80)).size(11.0),
                                    )
                                    .fill(Color32::from_rgb(50, 20, 20))
                                    .min_size(Vec2::new(36.0, 18.0)),
                                );
                                if stop_btn.clicked() {
                                    let mut s = shared.lock().unwrap();
                                    s.pending_commands.push(MotorCommand::Stop {
                                        port: port.to_string(),
                                    });
                                }
                            });
                        } else {
                            ui.label(
                                RichText::new("vacio")
                                    .color(Color32::from_rgb(90, 90, 110))
                                    .size(9.0),
                            );
                        }
                    });
                });

            ui.add_space(8.0);
        }
    });
}

// ─── Input ports ────────────────────────────────────────────────────────────

fn draw_input_ports(ui: &mut egui::Ui, state: &Ev3State) {
    ui.label(
        RichText::new("INPUT PORTS — SENSORS")
            .color(Color32::from_rgb(140, 140, 160))
            .size(9.0),
    );
    ui.add_space(4.0);

    egui::Grid::new("input_ports")
        .num_columns(4)
        .spacing([8.0, 4.0])
        .min_col_width(104.0)
        .max_col_width(104.0)
        .show(ui, |ui| {
            for port in ["in1", "in2", "in3", "in4"] {
                let sensor = state.sensors.iter().find(|s| s.port == port);

                let (bg, border) = if sensor.is_some() {
                    (Color32::from_rgb(25, 45, 70), Color32::from_rgb(60, 120, 200))
                } else {
                    (Color32::from_rgb(28, 38, 55), Color32::from_rgb(45, 60, 90))
                };

                egui::Frame::none()
                    .fill(bg)
                    .rounding(Rounding::same(8.0))
                    .stroke(Stroke::new(1.0, border))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(104.0, 72.0));
                        ui.set_max_width(104.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(port.replace("in", ""))
                                    .color(Color32::from_rgb(100, 150, 220))
                                    .size(18.0)
                                    .strong(),
                            );
                            if let Some(s) = sensor {
                                draw_sensor_widget(ui, s);
                            } else {
                                ui.label(
                                    RichText::new("vacio")
                                        .color(Color32::from_rgb(60, 80, 110))
                                        .size(9.0),
                                );
                            }
                        });
                    });
            }
            ui.end_row();
        });
}

fn draw_sensor_widget(ui: &mut egui::Ui, sensor: &crate::state::Sensor) {
    match &sensor.sensor_type {
        SensorType::Touch => {
            let pressed = sensor.value != 0.0;
            let (dot_color, label) = if pressed {
                (Color32::from_rgb(255, 80, 80), "PRESIONADO")
            } else {
                (Color32::from_rgb(60, 70, 90), "suelto")
            };
            ui.label(RichText::new("Touch").color(Color32::from_rgb(180, 190, 220)).size(8.0));
            ui.label(RichText::new("●").color(dot_color).size(16.0));
            ui.label(RichText::new(label).color(dot_color).size(7.5));
        }
        SensorType::Ultrasonic => {
            let dist = sensor.value.clamp(0.0, 255.0);
            let frac = dist / 255.0;
            let bar_color = lerp_color(
                Color32::from_rgb(0, 200, 120),
                Color32::from_rgb(220, 60, 60),
                frac,
            );
            ui.label(RichText::new("Sonic").color(Color32::from_rgb(180, 190, 220)).size(8.0));
            ui.label(
                RichText::new(format!("{:.0} cm", dist))
                    .color(Color32::from_rgb(120, 200, 255))
                    .size(10.0)
                    .strong(),
            );
            draw_bar(ui, frac, bar_color, 88.0, 5.0);
        }
        SensorType::Color => {
            let code = sensor.value as u8;
            let (color, name) = color_code_to_color(code);
            ui.label(RichText::new("Color").color(Color32::from_rgb(180, 190, 220)).size(8.0));
            let (rect, _) = ui.allocate_exact_size(Vec2::new(52.0, 10.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, Rounding::same(3.0), color);
            ui.label(RichText::new(name).color(Color32::from_rgb(200, 210, 240)).size(8.0));
        }
        SensorType::Gyro => {
            let angle_deg = sensor.value;
            let frac = ((angle_deg + 180.0) / 360.0).clamp(0.0, 1.0);
            ui.label(RichText::new("Gyro").color(Color32::from_rgb(180, 190, 220)).size(8.0));
            ui.label(
                RichText::new(format!("{:.0}°", angle_deg))
                    .color(Color32::from_rgb(255, 200, 80))
                    .size(10.0)
                    .strong(),
            );
            draw_bar(ui, frac, Color32::from_rgb(200, 160, 40), 88.0, 5.0);
        }
        SensorType::Infrared => {
            let prox = sensor.value.clamp(0.0, 100.0);
            let frac = prox / 100.0;
            let bar_color = lerp_color(
                Color32::from_rgb(180, 80, 200),
                Color32::from_rgb(255, 40, 120),
                frac,
            );
            ui.label(RichText::new("IR").color(Color32::from_rgb(180, 190, 220)).size(8.0));
            ui.label(
                RichText::new(format!("{:.0}%", prox))
                    .color(Color32::from_rgb(220, 140, 255))
                    .size(10.0)
                    .strong(),
            );
            draw_bar(ui, frac, bar_color, 88.0, 5.0);
        }
        SensorType::Unknown(driver) => {
            let short = if driver.len() > 10 { &driver[..10] } else { driver.as_str() };
            ui.label(RichText::new(short).color(Color32::from_rgb(140, 140, 160)).size(7.5));
            ui.label(
                RichText::new(format!("{:.1}", sensor.value))
                    .color(Color32::from_rgb(160, 160, 180))
                    .size(9.0),
            );
        }
    }
}

fn draw_bar(ui: &mut egui::Ui, fraction: f32, color: Color32, width: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(3.0), Color32::from_rgb(30, 32, 45));
    if fraction > 0.0 {
        let filled = egui::Rect::from_min_size(
            rect.min,
            Vec2::new((width * fraction).max(4.0), height),
        );
        ui.painter().rect_filled(filled, Rounding::same(3.0), color);
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

fn color_code_to_color(code: u8) -> (Color32, &'static str) {
    match code {
        0 => (Color32::from_rgb(30, 30, 35),   "ninguno"),
        1 => (Color32::from_rgb(20, 20, 20),   "negro"),
        2 => (Color32::from_rgb(40, 60, 160),  "azul"),
        3 => (Color32::from_rgb(30, 140, 40),  "verde"),
        4 => (Color32::from_rgb(200, 30, 30),  "rojo"),
        5 => (Color32::from_rgb(220, 220, 40), "amarillo"),
        6 => (Color32::from_rgb(220, 120, 30), "blanco"),
        7 => (Color32::from_rgb(230, 230, 230),"blanco"),
        _ => (Color32::from_rgb(90, 90, 110),  "?"),
    }
}