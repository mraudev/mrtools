//! Persistence of the user configuration (projects + settings).
//!
//! The schema is owned by the frontend (`src/lib/types.ts`); the backend only
//! stores the JSON document in the app config directory.

use serde_json::Value;
use std::{fs, io::ErrorKind, path::PathBuf};
use tauri::{AppHandle, Manager};

fn config_file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

/// Returns the stored configuration, or `null` if none has been saved yet.
#[tauri::command]
pub fn load_config(app: AppHandle) -> Result<Value, String> {
    let file = config_file(&app)?;
    match fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", file.display())),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Value::Null),
        Err(e) => Err(format!("{}: {e}", file.display())),
    }
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: Value) -> Result<(), String> {
    let file = config_file(&app)?;
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    // Write to a temp file first so a crash never leaves a truncated config behind.
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

/// Writes the configuration to a file chosen by the user. It never contains
/// tokens – those live in the credential store.
#[tauri::command]
pub fn export_config(path: String, config: Value) -> Result<(), String> {
    require_json(&path)?;
    let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())
}

/// Reads a configuration file chosen by the user; the frontend validates it.
#[tauri::command]
pub fn import_config(path: String) -> Result<Value, String> {
    require_json(&path)?;
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("Keine gültige Konfiguration: {e}"))
}

/// Release notes of a mrstart version from GitHub (public, no token).
#[tauri::command]
pub async fn release_notes(version: String) -> Result<String, String> {
    if version.is_empty()
        || !version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err("Ungültige Version.".into());
    }
    let url = format!("https://api.github.com/repos/mraudev/mrstart/releases/tags/v{version}");
    let response = crate::pulls::http_client()?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    let body: Value = response.json().await.map_err(|e| e.to_string())?;
    Ok(body["body"].as_str().unwrap_or("").to_string())
}

#[tauri::command]
pub fn config_path(app: AppHandle) -> Result<String, String> {
    Ok(config_file(&app)?.to_string_lossy().into_owned())
}
