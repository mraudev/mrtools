//! Persistence of the saved connections.
//!
//! The schema is owned by the frontend (`src/lib/types.ts`); the backend only
//! stores the JSON document in the app config directory. It never contains passwords.

use serde_json::Value;
use std::{fs, io::ErrorKind, path::PathBuf};
use tauri::{AppHandle, Manager};

fn connections_file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("connections.json"))
}

/// Returns the stored connections, or `null` if none have been saved yet.
#[tauri::command]
pub fn load_connections(app: AppHandle) -> Result<Value, String> {
    let file = connections_file(&app)?;
    match fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", file.display())),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Value::Null),
        Err(e) => Err(format!("{}: {e}", file.display())),
    }
}

#[tauri::command]
pub fn save_connections(app: AppHandle, connections: Value) -> Result<(), String> {
    let file = connections_file(&app)?;
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&connections).map_err(|e| e.to_string())?;
    // Write to a temp file first so a crash never leaves a truncated file behind.
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &file).map_err(|e| e.to_string())
}

fn require_json(path: &str) -> Result<(), String> {
    if path.to_ascii_lowercase().ends_with(".json") {
        Ok(())
    } else {
        Err("Bitte eine .json-Datei wählen.".into())
    }
}

/// Writes the connections to a file chosen by the user (a backup or for another PC).
#[tauri::command]
pub fn export_connections(path: String, connections: Value) -> Result<(), String> {
    require_json(&path)?;
    let text = serde_json::to_string_pretty(&connections).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())
}

/// Reads a file chosen by the user; the frontend validates and merges it.
#[tauri::command]
pub fn import_connections(path: String) -> Result<Value, String> {
    require_json(&path)?;
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("Keine gültige Datei: {e}"))
}
