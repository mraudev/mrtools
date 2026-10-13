mod govee;
mod hue;
mod secrets;
mod tado;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    secrets::init().expect("Windows-Anmeldeinformationsverwaltung nicht verfügbar");
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(tado::Tado::default())
        .manage(hue::Hue::default())
        .manage(govee::Govee::default())
        .setup(|app| {
            hue::init(app.handle());
            govee::init(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tado::auth_status,
            tado::login_start,
            tado::login_finish,
            tado::logout,
            tado::homes,
            tado::rooms,
            tado::room,
            tado::set_room,
            tado::resume_room,
            tado::end_open_window,
            tado::quick_action,
            tado::home_state,
            tado::set_presence,
            hue::hue_status,
            hue::hue_discover,
            hue::hue_pair,
            hue::hue_unpair,
            hue::hue_state,
            hue::hue_set_light,
            hue::hue_set_group,
            hue::hue_scene,
            govee::govee_scan,
            govee::govee_set,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
