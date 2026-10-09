// mrphone – Tauri-Version (im Aufbau, Stufe 1: Oberfläche, Einstellungen, Konten, Kontakte, Verlauf, Kurzwahl).
// Die Oberfläche (public/) ist dieselbe wie in der Electron-Version; bridge.js stellt window.phone bereit.
mod commands;
mod config;
mod contacts;
mod history;
mod paths;
mod secrets;

use commands::AppState;
use std::sync::Mutex;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

// JSON mit 1 Leerzeichen Einrückung – so schreibt die Electron-Version contacts.json und history.json.
pub fn json_indent1(value: &serde_json::Value) -> Vec<u8> {
    use serde::Serialize;
    let mut out = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(
        &mut out,
        serde_json::ser::PrettyFormatter::with_indent(b" "),
    );
    value.serialize(&mut ser).expect("JSON");
    out
}

// Mikrofon-Freigabe selbst beantworten (sonst fragt WebView2 nach) – nur das Mikrofon, alles andere nie.
fn allow_microphone_only(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    window.with_webview(|wv| unsafe {
        use webview2_com::{Microsoft::Web::WebView2::Win32::*, PermissionRequestedEventHandler};
        let Ok(core) = wv.controller().CoreWebView2() else {
            return;
        };
        let mut token = Default::default();
        let _ = core.add_PermissionRequested(
            &PermissionRequestedEventHandler::create(Box::new(|_, args| {
                if let Some(args) = args {
                    let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
                    args.PermissionKind(&mut kind)?;
                    let allow = kind == COREWEBVIEW2_PERMISSION_KIND_MICROPHONE;
                    args.SetState(if allow {
                        COREWEBVIEW2_PERMISSION_STATE_ALLOW
                    } else {
                        COREWEBVIEW2_PERMISSION_STATE_DENY
                    })?;
                }
                Ok(())
            })),
            &mut token,
        );
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = paths::data_dir()?;
            let crypt = secrets::OsCrypt::load_or_create(&dir);
            let mut cfg = config::load(&dir);
            config::load_secrets(&mut cfg, &crypt);
            let theme = cfg["theme"].as_str().unwrap_or("system").to_string();
            app.manage(AppState {
                contacts: Mutex::new(contacts::Contacts::load(&dir)),
                history: Mutex::new(history::History::load(&dir)),
                cfg: Mutex::new(cfg),
                crypt,
                dir,
            });
            let window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("mrphone")
                    .inner_size(400.0, 800.0)
                    .min_inner_size(360.0, 740.0)
                    .initialization_script(include_str!("bridge.js"))
                    .build()?;
            allow_microphone_only(&window)?;
            commands::apply_theme(app.handle(), &theme);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::command,
            commands::not_yet,
            commands::get_audio,
            commands::set_audio,
            commands::get_options,
            commands::set_options,
            commands::get_accounts,
            commands::save_account,
            commands::delete_account,
            commands::get_ringtone,
            commands::choose_ringtone,
            commands::reset_ringtone,
            commands::get_version,
            commands::open_data_dir,
            commands::get_contacts,
            commands::save_contact,
            commands::delete_contact,
            commands::get_history,
            commands::clear_history,
            commands::get_favorites,
            commands::save_favorites,
        ])
        .run(tauri::generate_context!())
        .expect("mrphone konnte nicht starten");
}
