//! Installed apps, read from the uninstall entries in the registry – the
//! NSIS installers of Tauri and electron-builder both write one.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    #[serde(skip)]
    pub name: String,
    pub version: Option<String>,
    /// Path of the program, if it exists.
    pub exe: Option<String>,
}

/// Strips quotes and the icon index (`"C:\app.exe",0`).
fn clean_path(raw: &str) -> &str {
    let raw = raw.trim();
    let raw = match raw.rsplit_once(',') {
        Some((path, index)) if index.trim().parse::<i32>().is_ok() => path,
        _ => raw,
    };
    raw.trim().trim_matches('"')
}

/// The program: `DisplayIcon` if it is an `.exe`, otherwise `<InstallLocation>\<name>.exe`.
fn find_exe(name: &str, icon: Option<&str>, location: Option<&str>) -> Option<PathBuf> {
    let from_icon = icon
        .map(clean_path)
        .filter(|p| p.to_ascii_lowercase().ends_with(".exe"))
        .map(PathBuf::from);
    let from_location = location
        .map(clean_path)
        .filter(|p| !p.is_empty())
        .map(|dir| Path::new(dir).join(format!("{name}.exe")));
    [from_icon, from_location].into_iter().flatten().find(|p| p.is_file())
}

#[cfg(windows)]
pub fn all() -> Vec<Installed> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
    use winreg::RegKey;

    const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
    let roots = [
        (HKEY_CURRENT_USER, KEY_READ),
        (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_64KEY),
        (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY),
    ];

    let mut result = Vec::new();
    for (hive, flags) in roots {
        let Ok(uninstall) = RegKey::predef(hive).open_subkey_with_flags(UNINSTALL, flags) else {
            continue;
        };
        for key in uninstall.enum_keys().flatten() {
            let Ok(entry) = uninstall.open_subkey_with_flags(&key, flags) else {
                continue;
            };
            let Ok(name) = entry.get_value::<String, _>("DisplayName") else {
                continue;
            };
            let icon = entry.get_value::<String, _>("DisplayIcon").ok();
            let location = entry.get_value::<String, _>("InstallLocation").ok();
            let exe = find_exe(&name, icon.as_deref(), location.as_deref());
            result.push(Installed {
                version: entry.get_value::<String, _>("DisplayVersion").ok(),
                exe: exe.map(|p| p.to_string_lossy().into_owned()),
                name,
            });
        }
    }
    result
}

#[cfg(not(windows))]
pub fn all() -> Vec<Installed> {
    Vec::new()
}

/// Starts the installed program of the app `name`. The path is looked up
/// again here instead of being taken from the frontend.
#[tauri::command]
pub fn launch(name: String) -> Result<(), String> {
    let exe = all()
        .into_iter()
        .find(|i| i.name.eq_ignore_ascii_case(&name))
        .and_then(|i| i.exe)
        .ok_or_else(|| format!("{name} ist nicht installiert."))?;
    let exe = PathBuf::from(exe);
    let mut command = std::process::Command::new(&exe);
    if let Some(dir) = exe.parent() {
        command.current_dir(dir);
    }
    command.spawn().map(drop).map_err(|e| format!("{}: {e}", exe.display()))
}

/// Opens an app folder in the Explorer.
#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    if !Path::new(&path).is_dir() {
        return Err(format!("{path} ist kein Ordner."));
    }
    std::process::Command::new("explorer.exe")
        .arg(&path)
        .spawn()
        .map(drop)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::clean_path;

    #[test]
    fn cleans_display_icon() {
        assert_eq!(clean_path(r#""C:\a\mrstart.exe""#), r"C:\a\mrstart.exe");
        assert_eq!(clean_path(r"C:\a\SIP Phone.exe,0"), r"C:\a\SIP Phone.exe");
        assert_eq!(clean_path(r#""C:\a,b\x.exe",-101"#), r"C:\a,b\x.exe");
    }
}
