use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Reads a text file as UTF-8 (without BOM); invalid bytes become U+FFFD.
#[tauri::command]
pub fn read_text(path: String) -> Result<String, String> {
    let bytes = fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
    Ok(decode(&bytes))
}

fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

#[tauri::command]
pub fn write_text(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| format!("{path}: {e}"))
}

/// The file passed on the command line (`mrtextedit.exe brief.html`), if any.
#[tauri::command]
pub fn startup_file() -> Option<String> {
    file_argument(std::env::args().skip(1))
}

fn file_argument(mut args: impl Iterator<Item = String>) -> Option<String> {
    args.find(|arg| !arg.starts_with('-'))
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_config_dir().map_err(|e| e.to_string())?.join("config.json"))
}

/// Toolbar and formats as JSON, `None` before the first save (the frontend then uses its defaults).
#[tauri::command]
pub fn load_config(app: AppHandle) -> Result<Option<String>, String> {
    match fs::read(config_path(&app)?) {
        Ok(bytes) => Ok(Some(decode(&bytes))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn save_config(app: AppHandle, json: String) -> Result<(), String> {
    serde_json::from_str::<serde_json::Value>(&json).map_err(|e| format!("Kein gültiges JSON: {e}"))?;
    let path = config_path(&app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_strips_bom_and_replaces_invalid_bytes() {
        assert_eq!(decode(b"\xEF\xBB\xBF<p>\xC3\xA4</p>"), "<p>ä</p>");
        assert_eq!(decode(b"a\xFFb"), "a\u{FFFD}b");
    }

    #[test]
    fn file_argument_skips_flags() {
        let args = ["--flag", r"C:\brief.html"].map(String::from);
        assert_eq!(file_argument(args.into_iter()).as_deref(), Some(r"C:\brief.html"));
        assert_eq!(file_argument(std::iter::empty()), None);
    }
}
