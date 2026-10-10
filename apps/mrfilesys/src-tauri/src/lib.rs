mod delete;
mod drives;
mod files;
mod props;
mod search;
mod shell;
mod transfer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(delete::Deletions::default())
        .manage(transfer::Transfers::default())
        .manage(search::Search::default())
        .setup(|app| {
            search::init(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            drives::list_drives,
            files::list_dir,
            files::stat,
            files::rename,
            files::create,
            files::read_text,
            props::measure,
            props::item_info,
            props::set_attributes,
            props::hashes,
            shell::open_path,
            shell::open_with,
            shell::properties,
            shell::open_terminal,
            shell::recycle,
            shell::clipboard_set,
            shell::clipboard_get,
            shell::clipboard_clear,
            shell::drag_out,
            delete::delete_permanently,
            delete::cancel_delete,
            transfer::check_conflicts,
            transfer::start_transfer,
            transfer::cancel_transfer,
            search::search,
            search::search_status,
            search::rebuild_index,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
