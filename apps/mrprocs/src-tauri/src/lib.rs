mod actions;
mod procs;
mod win;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(procs::Monitor::default())
        .invoke_handler(tauri::generate_handler![
            procs::snapshot,
            actions::kill,
            actions::kill_tree,
            actions::suspend,
            actions::set_priority,
            actions::details,
            actions::modules,
            actions::connections,
            actions::icons,
            actions::show_properties,
            actions::is_elevated,
            actions::restart_as_admin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
