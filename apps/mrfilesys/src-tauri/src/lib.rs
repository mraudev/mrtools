mod delete;
mod drives;
mod files;
mod shell;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .manage(delete::Deletions::default())
        .invoke_handler(tauri::generate_handler![
            drives::list_drives,
            files::list_dir,
            files::stat,
            files::rename,
            files::create,
            files::read_text,
            files::folder_size,
            shell::open_path,
            shell::open_with,
            shell::properties,
            shell::open_terminal,
            shell::file_op,
            shell::clipboard_set,
            shell::clipboard_get,
            shell::clipboard_clear,
            shell::drag_out,
            delete::delete_permanently,
            delete::cancel_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
