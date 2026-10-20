//! Tauri-Befehle für den Import aus GHFakt.

use super::Bericht;
use crate::db::Res;
use crate::state::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
struct Fortschritt {
    text: String,
    anteil: f64,
}

/// Standardordner von GHFakt, falls vorhanden.
#[tauri::command]
pub fn altdaten_vorschlag() -> Option<String> {
    [r"C:\Entwicklung\GHFakt", r"C:\GHFakt", r"C:\GH", r"D:\GHFakt"]
        .iter()
        .find(|p| super::pruefen(std::path::Path::new(p)).is_ok())
        .map(|p| p.to_string())
}

#[tauri::command]
pub async fn altdaten_importieren(app: AppHandle, ordner: String) -> Res<Bericht> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let ordner = PathBuf::from(ordner);
        super::pruefen(&ordner)?;
        // Vorher sichern, falls schon Daten da sind.
        let leer = state.mit_db(|db| Ok(db.query_row("SELECT COUNT(*) FROM kunde", [], |r| r.get::<_, i64>(0)).unwrap_or(0) == 0))?;
        if !leer {
            state.sichern(1000)?;
        }
        let arbeit = std::env::temp_dir().join(format!("mrfakt-import-{}", std::process::id()));
        let melden = |text: &str, anteil: f64| {
            let _ = app.emit("import-fortschritt", Fortschritt { text: text.into(), anteil });
        };
        let ergebnis = state.mit_db(|db| super::importieren(db, &ordner, &arbeit, &melden));
        let _ = std::fs::remove_dir_all(&arbeit);
        ergebnis
    })
    .await
    .map_err(|e| e.to_string())?
}
