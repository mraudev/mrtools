mod active;
mod connections;
mod launch;
mod win;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(active::Monitor::default())
        .invoke_handler(tauri::generate_handler![
            connections::load_connections,
            connections::save_connections,
            launch::connect,
            active::active_connections,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
