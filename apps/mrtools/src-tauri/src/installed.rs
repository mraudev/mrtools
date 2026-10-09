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
    /// The uninstaller from `UninstallString`, if it exists.
    #[serde(skip)]
    pub uninstaller: Option<PathBuf>,
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

/// The program of an `UninstallString` such as `"C:\app\uninstall.exe" /currentuser`.
fn uninstaller_path(raw: &str) -> Option<PathBuf> {
    let raw = raw.trim();
    let path = match raw.strip_prefix('"') {
        Some(rest) => rest.split('"').next()?,
        None => &raw[..raw.to_ascii_lowercase().find(".exe")? + 4],
    };
    Some(PathBuf::from(path)).filter(|p| p.to_string_lossy().to_ascii_lowercase().ends_with(".exe"))
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
            let uninstaller = entry
                .get_value::<String, _>("UninstallString")
                .ok()
                .and_then(|raw| uninstaller_path(&raw))
                .filter(|p| p.is_file());
            result.push(Installed {
                version: entry.get_value::<String, _>("DisplayVersion").ok(),
                exe: exe.map(|p| p.to_string_lossy().into_owned()),
                uninstaller,
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

/// Opens the installation folder of the app `name` in the Explorer with its
/// program selected. Like `launch`, the path is looked up here.
#[tauri::command]
pub fn reveal(name: String) -> Result<(), String> {
    let exe = find(&name)
        .and_then(|i| i.exe)
        .ok_or_else(|| format!("{name} ist nicht installiert."))?;
    tauri_plugin_opener::reveal_item_in_dir(exe).map_err(|e| e.to_string())
}

fn find(name: &str) -> Option<Installed> {
    all().into_iter().find(|i| i.name.eq_ignore_ascii_case(name))
}

/// Ends all processes running the program file `exe` and waits until they are gone.
fn close_running(exe: &Path) -> Result<(), String> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

    let refresh = ProcessRefreshKind::nothing().with_exe(UpdateKind::Always);
    let mut system = System::new();
    for attempt in 0..20 {
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh);
        let running: Vec<_> = system
            .processes()
            .values()
            .filter(|p| p.exe().is_some_and(|path| path == exe))
            .collect();
        if running.is_empty() {
            return Ok(());
        }
        if attempt == 0 {
            for process in &running {
                process.kill();
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    Err("Die laufende App ließ sich nicht beenden.".into())
}

/// Uninstalls the app `name` silently with its registered uninstaller; its
/// settings stay. The silent uninstaller does not reliably close a running
/// instance (and then leaves the locked files behind while still removing the
/// uninstall entry), so running instances are ended first. NSIS uninstallers
/// copy themselves to the temp folder and return at once, so this waits until
/// both the uninstall entry and the program file are gone.
#[tauri::command]
pub async fn uninstall(name: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let installed = find(&name).ok_or_else(|| format!("{name} ist nicht installiert."))?;
        let uninstaller = installed.uninstaller.ok_or("Kein Deinstallationsprogramm gefunden.")?;
        let exe = installed.exe.map(PathBuf::from);
        if let Some(exe) = &exe {
            close_running(exe)?;
        }
        let status = std::process::Command::new(&uninstaller)
            .arg("/S")
            .status()
            .map_err(|e| format!("{}: {e}", uninstaller.display()))?;
        if !status.success() {
            return Err(format!("Deinstallation beendet mit Code {}.", status.code().unwrap_or(-1)));
        }
        for _ in 0..120 {
            if find(&name).is_none() && !exe.as_deref().is_some_and(Path::exists) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        Err("Die Deinstallation wurde nicht innerhalb einer Minute abgeschlossen.".into())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{clean_path, uninstaller_path};
    use std::path::PathBuf;

    #[cfg(windows)]
    #[test]
    fn closes_running_instances_of_a_program_file() {
        // A private copy, so no other process runs the same file.
        let dir = std::env::temp_dir().join(format!("mrtools-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("ping-copy.exe");
        std::fs::copy(r"C:\Windows\System32\PING.EXE", &exe).unwrap();
        let mut child = std::process::Command::new(&exe)
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();

        super::close_running(&exe).unwrap();
        assert!(child.wait().is_ok());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn parses_uninstall_string() {
        let path = |s: &str| uninstaller_path(s).map(|p| p.to_string_lossy().into_owned());
        assert_eq!(path(r#""C:\a\uninstall.exe""#).as_deref(), Some(r"C:\a\uninstall.exe"));
        assert_eq!(path(r#""C:\a b\Uninstall X.exe" /currentuser"#).as_deref(), Some(r"C:\a b\Uninstall X.exe"));
        assert_eq!(path(r"C:\a\uninstall.exe /S").as_deref(), Some(r"C:\a\uninstall.exe"));
        assert_eq!(uninstaller_path(r#""C:\a\setup.msi""#), None::<PathBuf>);
    }

    #[test]
    fn cleans_display_icon() {
        assert_eq!(clean_path(r#""C:\a\mrstart.exe""#), r"C:\a\mrstart.exe");
        assert_eq!(clean_path(r"C:\a\SIP Phone.exe,0"), r"C:\a\SIP Phone.exe");
        assert_eq!(clean_path(r#""C:\a,b\x.exe",-101"#), r"C:\a,b\x.exe");
    }
}
