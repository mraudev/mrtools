mod active;
mod connections;
mod import;
mod launch;
mod passwords;
mod probe;
mod win;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(active::Monitor::default())
        .invoke_handler(tauri::generate_handler![
            connections::load_connections,
            connections::save_connections,
            connections::export_connections,
            connections::import_connections,
            launch::connect,
            launch::open_sftp,
            launch::tools,
            active::active_connections,
            active::focus_session,
            active::end_session,
            passwords::rdp_passwords,
            passwords::set_rdp_password,
            passwords::delete_rdp_password,
            probe::probe,
            import::import_sources,
            import::import_rdp_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
