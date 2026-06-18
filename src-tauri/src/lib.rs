mod connection;
mod state;
mod polling;
mod commands;

use std::sync::{Arc, Mutex};
use state::Ev3State;
use connection::make_shared_conn;
use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared_state = Arc::new(Mutex::new(Ev3State::default()));
    let shared_conn  = make_shared_conn();

    polling::start_polling(
        shared_state.clone(),
        shared_conn.clone(),
        "robot".to_string(),
        "maker".to_string(),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            ev3: shared_state,
            conn: shared_conn,
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::set_motor_speed,
            commands::stop_motor,
            commands::stop_all_motors,
            commands::set_ip,
            commands::reconnect,
            commands::run_bash,
            commands::run_code,
            commands::get_program_output,
            commands::kill_program,
            commands::save_file,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri app");
}