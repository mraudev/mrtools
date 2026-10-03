mod config;
mod dashboard;
mod git;
mod gitea;
mod github;
mod launch;
mod projects;
mod pulls;
mod review;
mod secrets;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    secrets::init().expect("Windows-Anmeldeinformationsverwaltung nicht verfügbar");

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
            git::git_run,
            pulls::branch_info,
            pulls::pull_requests,
            dashboard::dashboard,
            dashboard::update_pull_branch,
            review::review_with_claude,
            secrets::secret_status,
            secrets::set_secret,
            secrets::delete_secret,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
