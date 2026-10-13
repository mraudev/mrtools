mod ports;
mod win;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(ports::Monitor::default())
        .invoke_handler(tauri::generate_handler![
            ports::snapshot,
            ports::kill,
            ports::icons,
            ports::is_elevated,
            ports::restart_as_admin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
