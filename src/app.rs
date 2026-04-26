use std::sync::{Arc, Mutex};
use eframe::egui::{self, Color32, RichText, Rounding, Vec2, Stroke};
use crate::state::{Ev3State, SensorType};

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

        let state = self.state.lock().unwrap().clone();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(10, 10, 12)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(24.0);
                        draw_brick(ui, &state);
                        ui.add_space(24.0);
                    });
                });
            });
    }
}

fn draw_brick(ui: &mut egui::Ui, state: &Ev3State) {
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
                    draw_output_ports(ui, state);
                    ui.add_space(10.0);
                    draw_input_ports(ui, state);
                    ui.add_space(6.0);

                    // Alertas
                    if !state.alerts.is_empty() {
                        ui.add_space(8.0);
                        for alert in &state.alerts {
                            ui.label(RichText::new(alert).color(Color32::from_rgb(255, 80, 80)).size(11.0));
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
                ui.label(RichText::new("MINDSTORMS").color(Color32::from_rgb(255, 200, 200)).size(10.0).strong());
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
                        ui.label(RichText::new("EV3 CONTROLLER").color(Color32::from_rgb(15, 40, 15)).size(13.0).strong());
                    });

                    ui.add_space(4.0);

                    // Estado conexión
                    if state.connected {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("●").color(Color32::from_rgb(0, 180, 60)).size(10.0));
                            ui.label(RichText::new("Conectado").color(Color32::from_rgb(15, 40, 15)).size(10.0));
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("●").color(Color32::from_rgb(160, 40, 40)).size(10.0));
                            ui.label(RichText::new("Sin conexion — enciende el EV3").color(Color32::from_rgb(15, 40, 15)).size(10.0));
                        });
                    }

                    ui.add_space(5.0);
                    ui.add(egui::Separator::default().spacing(3.0));
                    ui.add_space(3.0);

                    // Batería
                    let bat_text = if state.connected {
                        format!("Bateria   {:.2}V", state.battery_voltage)
                    } else {
                        "Bateria   —".to_string()
                    };

                    let sensor_count = state.sensors.len();
                    let motor_count  = state.motors.iter().filter(|m| m.connected).count();

                    ui.label(RichText::new(format!("Sensores  {}", if state.connected { sensor_count.to_string() } else { "—".to_string() })).color(Color32::from_rgb(15, 40, 15)).size(10.0));
                    ui.label(RichText::new(format!("Motores   {}", if state.connected { motor_count.to_string() } else { "—".to_string() })).color(Color32::from_rgb(15, 40, 15)).size(10.0));
                    ui.label(RichText::new(bat_text).color(Color32::from_rgb(15, 40, 15)).size(10.0));
                });
        });
}

fn draw_dpad(ui: &mut egui::Ui) {
    let dark_btn = Color32::from_rgb(28, 28, 35);
    let gray_btn = Color32::from_rgb(75, 78, 92);
    let orange   = Color32::from_rgb(215, 100, 0);
    let txt      = Color32::WHITE;

    ui.vertical_centered(|ui| {
        ui.horizontal(|ui| {
            ui.add_space(100.0);
            egui::Frame::none().fill(dark_btn).rounding(Rounding::same(8.0)).inner_margin(egui::Margin::same(2.0)).show(ui, |ui| {
                let _ = ui.add(egui::Button::new(RichText::new("<").color(txt).size(14.0)).fill(Color32::TRANSPARENT).min_size(Vec2::new(44.0, 30.0)).frame(false));
            });
            ui.add_space(8.0);
            egui::Frame::none().fill(gray_btn).rounding(Rounding::same(8.0)).show(ui, |ui| {
                let _ = ui.add(egui::Button::new(RichText::new("^").color(txt).size(13.0)).fill(Color32::TRANSPARENT).min_size(Vec2::new(44.0, 30.0)).frame(false));
            });
            ui.add_space(8.0);
            egui::Frame::none().fill(dark_btn).rounding(Rounding::same(8.0)).show(ui, |ui| {
                let _ = ui.add(egui::Button::new(RichText::new("OK").color(txt).size(14.0)).fill(Color32::TRANSPARENT).min_size(Vec2::new(44.0, 30.0)).frame(false));
            });
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.add_space(114.0);
            egui::Frame::none().fill(gray_btn).rounding(Rounding::same(8.0)).show(ui, |ui| {
                let _ = ui.add(egui::Button::new(RichText::new("<").color(txt).size(13.0)).fill(Color32::TRANSPARENT).min_size(Vec2::new(36.0, 30.0)).frame(false));
            });
            ui.add_space(4.0);
            egui::Frame::none().fill(orange).rounding(Rounding::same(22.0)).stroke(Stroke::new(2.0, Color32::from_rgb(255, 150, 50))).show(ui, |ui| {
                let _ = ui.add(egui::Button::new(RichText::new(" ").size(18.0)).fill(Color32::TRANSPARENT).min_size(Vec2::new(44.0, 44.0)).frame(false));
            });
            ui.add_space(4.0);
            egui::Frame::none().fill(gray_btn).rounding(Rounding::same(8.0)).show(ui, |ui| {
                let _ = ui.add(egui::Button::new(RichText::new(">").color(txt).size(13.0)).fill(Color32::TRANSPARENT).min_size(Vec2::new(36.0, 30.0)).frame(false));
            });
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.add_space(152.0);
            egui::Frame::none().fill(gray_btn).rounding(Rounding::same(8.0)).show(ui, |ui| {
                let _ = ui.add(egui::Button::new(RichText::new("v").color(txt).size(13.0)).fill(Color32::TRANSPARENT).min_size(Vec2::new(44.0, 30.0)).frame(false));
            });
        });
    });
}

fn draw_output_ports(ui: &mut egui::Ui, state: &Ev3State) {
    ui.label(RichText::new("OUTPUT PORTS — MOTORS").color(Color32::from_rgb(140, 140, 160)).size(9.0));
    ui.add_space(4.0);

    egui::Grid::new("output_ports")
        .num_columns(4)
        .spacing([8.0, 0.0])
        .min_col_width(104.0)
        .max_col_width(104.0)
        .show(ui, |ui| {
            for port in ["outA", "outB", "outC", "outD"] {
                let motor = state.motors.iter().find(|m| m.port == port);
                let connected = motor.map(|m| m.connected).unwrap_or(false);

                let (bg, border, label_color) = if connected {
                    (Color32::from_rgb(30, 60, 35), Color32::from_rgb(0, 140, 60), Color32::from_rgb(80, 220, 120))
                } else {
                    (Color32::from_rgb(35, 36, 45), Color32::from_rgb(65, 68, 85), Color32::from_rgb(200, 200, 220))
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
                            ui.label(RichText::new(port.replace("out", "")).color(label_color).size(18.0).strong());
                            if connected {
                                let speed = motor.map(|m| m.speed).unwrap_or(0);
                                ui.label(RichText::new(format!("{}rpm", speed)).color(Color32::from_rgb(80, 200, 100)).size(9.0));
                            } else {
                                ui.label(RichText::new("vacio").color(Color32::from_rgb(90, 90, 110)).size(9.0));
                            }
                        });
                    });
            }
            ui.end_row();
        });
}

fn draw_input_ports(ui: &mut egui::Ui, state: &Ev3State) {
    ui.label(RichText::new("INPUT PORTS — SENSORS").color(Color32::from_rgb(140, 140, 160)).size(9.0));
    ui.add_space(4.0);

    egui::Grid::new("input_ports")
        .num_columns(4)
        .spacing([8.0, 0.0])
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
                        ui.set_min_size(Vec2::new(104.0, 52.0));
                        ui.set_max_width(104.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new(port.replace("in", "")).color(Color32::from_rgb(100, 150, 220)).size(18.0).strong());
                            if let Some(s) = sensor {
                                let tipo = match &s.sensor_type {
                                    SensorType::Touch       => "Touch",
                                    SensorType::Color       => "Color",
                                    SensorType::Ultrasonic  => "Sonic",
                                    SensorType::Gyro        => "Gyro",
                                    SensorType::Infrared    => "IR",
                                    SensorType::Unknown(_)  => "?",
                                };
                                ui.label(RichText::new(format!("{} {:.0}", tipo, s.value)).color(Color32::from_rgb(120, 170, 240)).size(9.0));
                            } else {
                                ui.label(RichText::new("vacio").color(Color32::from_rgb(60, 80, 110)).size(9.0));
                            }
                        });
                    });
            }
            ui.end_row();
        });
}