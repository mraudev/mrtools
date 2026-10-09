mod drives;
mod scan;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(scan::Scanner::default())
        .invoke_handler(tauri::generate_handler![
            drives::list_drives,
            scan::start_scan,
            scan::cancel_scan,
            scan::get_node,
            scan::file_types,
            scan::largest_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
