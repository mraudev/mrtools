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

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    secrets::init().expect("Windows-Anmeldeinformationsverwaltung nicht verfügbar");

    tauri::Builder::default()
        // Must be registered first: a second start only brings the running
        // window to the front.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            config::load_config,
            config::save_config,
            config::config_path,
            config::export_config,
            config::import_config,
            config::release_notes,
            projects::existing_files,
            projects::watched_projects,
            launch::run_command,
            launch::open_path,
            launch::open_git_tool,
            git::git_run,
            git::git_fetch_all,
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
