mod connection;
mod app;
mod state;
mod polling;
mod editor;

use std::sync::{Arc, Mutex};
use app::Ev3App;
use state::Ev3State;
use connection::make_shared_conn;

fn main() -> eframe::Result<()> {
    let shared_state = Arc::new(Mutex::new(Ev3State::default()));
    let shared_conn  = make_shared_conn();

    polling::start_polling(
        shared_state.clone(),
        shared_conn.clone(),
        "robot".to_string(),
        "maker".to_string(),
    );

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([620.0, 820.0])
            .with_title("EV3 Controller"),
        ..Default::default()
    };

    eframe::run_native(
        "EV3 Controller",
        options,
        Box::new(move |_cc| Box::new(Ev3App::new(shared_state, shared_conn))),
    )
}