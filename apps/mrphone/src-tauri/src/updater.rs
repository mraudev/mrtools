// Automatische Updates wie in der Electron-Version: beim Start und alle 4 Stunden prüfen, ein gefundenes
// Update still herunterladen, dann „Neu starten“ anbieten – oder beim Beenden einspielen. Die Pakete sind
// signiert (öffentlicher Schlüssel in tauri.conf.json), Quelle ist das feste Release mrphone-latest.
use crate::{commands::AppState, phone::PhoneCmd};
use serde_json::{json, Value};
use std::{sync::Mutex, time::Duration};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

const INTERVAL: Duration = Duration::from_secs(4 * 60 * 60);

#[derive(Default)]
pub struct Pending(Mutex<Option<(Update, Vec<u8>)>>);

// Nur die installierte App aktualisiert sich – nicht die Exe aus dem Build-Ordner und keine Selbsttests.
// Der Installer trägt den Programmordner unter Uninstall\mrphone ein.
fn installed() -> bool {
    let Some(location) = crate::migration::uninstall_value("mrphone", "InstallLocation") else {
        return false;
    };
    let location = std::path::PathBuf::from(location.trim_matches('"'));
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    match (exe_dir, location.canonicalize()) {
        (Some(dir), Ok(loc)) => dir.canonicalize().is_ok_and(|d| d == loc),
        _ => false,
    }
}

pub fn start(app: &AppHandle) {
    app.manage(Pending::default());
    if crate::desktop::quiet() || !installed() {
        crate::logger::info("Automatische Updates nur in der installierten App");
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            check(&app).await;
            tokio::time::sleep(INTERVAL).await;
        }
    });
}

async fn check(app: &AppHandle) {
    if app.state::<Pending>().0.lock().unwrap().is_some() {
        return;
    }
    let found = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(err) => Err(err),
    };
    let update = match found {
        Ok(Some(update)) => update,
        Ok(None) => return crate::logger::info("Kein Update verfügbar"),
        Err(err) => return crate::logger::warn(&format!("Update-Fehler: {err}")),
    };
    crate::logger::info(&format!(
        "Update {} verfügbar, lade herunter …",
        update.version
    ));
    match update.download(|_, _| {}, || {}).await {
        Ok(bytes) => {
            let info = json!({ "version": update.version, "notes": update.body.clone().unwrap_or_default().trim() });
            crate::logger::info(&format!("Update {} bereit", update.version));
            *app.state::<Pending>().0.lock().unwrap() = Some((update, bytes));
            let _ = app.emit("phone:update", info);
        }
        Err(err) => crate::logger::warn(&format!("Update-Fehler: {err}")),
    }
}

#[tauri::command]
pub fn get_update(pending: State<'_, Pending>) -> Value {
    match pending.0.lock().unwrap().as_ref() {
        Some((update, _)) => {
            json!({ "version": update.version, "notes": update.body.clone().unwrap_or_default().trim() })
        }
        None => Value::Null,
    }
}

// „Neu starten“: sauber abmelden, dann installiert der Installer still und startet mrphone wieder.
#[tauri::command]
pub async fn install_update(
    state: State<'_, AppState>,
    pending: State<'_, Pending>,
) -> Result<Value, ()> {
    if pending.0.lock().unwrap().is_none() {
        return Ok(Value::Null);
    }
    if !state.phone.snapshot.lock().unwrap()["call"].is_null() {
        return Ok(json!({ "error": "Bitte erst das Gespräch beenden." }));
    }
    crate::commands::stop_cti(&state);
    state.phone.stop().await;
    let Some((update, bytes)) = pending.0.lock().unwrap().take() else {
        return Ok(Value::Null);
    };
    // Unter Windows beendet install() die App, sobald der Installer läuft.
    if let Err(err) = update.install(bytes) {
        crate::logger::warn(&format!("Update-Fehler: {err}"));
        state.phone.send(PhoneCmd::Register);
        return Ok(json!({ "error": format!("Update fehlgeschlagen: {err}") }));
    }
    Ok(Value::Null)
}

// Beim Beenden (nach dem Abmelden): ein bereits geladenes Update einspielen, ohne mrphone neu zu starten.
pub fn install_on_quit(app: &AppHandle) {
    let Some((update, bytes)) = app.state::<Pending>().0.lock().unwrap().take() else {
        return;
    };
    crate::logger::info(&format!(
        "Update {} wird beim Beenden installiert",
        update.version
    ));
    if let Err(err) = update.restart_after_install(false).install(bytes) {
        crate::logger::warn(&format!("Update-Fehler: {err}"));
    }
}
