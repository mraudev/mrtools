mod config;
mod git;
mod launch;
mod projects;
mod pulls;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            config::load_config,
            config::save_config,
            config::config_path,
            projects::existing_files,
            projects::watched_projects,
            launch::run_command,
            launch::open_path,
            launch::open_git_tool,
            git::git_branch,
            git::git_run,
            pulls::pull_requests,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
