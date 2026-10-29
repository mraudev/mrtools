//! "mrfilesys hier öffnen" in Explorer's context menu, for the current user only (no admin rights):
//! entries under `HKCU\Software\Classes` that start mrfilesys with the folder. Windows 11 shows them
//! under "Weitere Optionen anzeigen"; the new menu only takes packaged apps.

use std::path::Path;

/// Where Explorer reads the entries from: right click on a folder, a drive and the folder background.
const KEYS: [(&str, &str); 3] = [
    ("Software\\Classes\\Directory\\shell\\mrfilesys", "In mrfilesys öffnen"),
    ("Software\\Classes\\Drive\\shell\\mrfilesys", "In mrfilesys öffnen"),
    ("Software\\Classes\\Directory\\Background\\shell\\mrfilesys", "mrfilesys hier öffnen"),
];

/// The folder mrfilesys was started with, if any (`mrfilesys.exe "C:\Projekte"`).
pub fn folder_arg(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        // `"%V"` on a drive is `"C:\"`, read as `C:"` – the backslash escapes the closing quote.
        .map(|a| a.strip_suffix('"').map_or_else(|| a.clone(), |a| format!("{a}\\")))
        .find(|a| Path::new(a).is_dir())
}

#[tauri::command]
pub fn launch_path() -> Option<String> {
    folder_arg(&std::env::args().collect::<Vec<_>>())
}

#[tauri::command]
pub fn set_context_menu(enabled: bool) -> Result<(), String> {
    for (key, _) in KEYS {
        imp::delete(key);
    }
    if !enabled {
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy();
    for (key, label) in KEYS {
        imp::write(key, None, label)?;
        imp::write(key, Some("Icon"), &format!("\"{exe}\",0"))?;
        // %V: the folder, also for the background and drives.
        imp::write(&format!("{key}\\command"), None, &format!("\"{exe}\" \"%V\""))?;
    }
    Ok(())
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::Registry::{
        RegDeleteTreeW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    /// Sets a string value (`None`: the default value), creating the key if needed.
    pub fn write(key: &str, name: Option<&str>, value: &str) -> Result<(), String> {
        let key = wide(key);
        let name = name.map(wide);
        let data = wide(value);
        let result = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ref().map_or(std::ptr::null(), |n| n.as_ptr()),
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(format!("Registry-Eintrag konnte nicht geschrieben werden (Fehler {result})"))
        }
    }

    pub fn delete(key: &str) {
        let key = wide(key);
        unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, key.as_ptr()) };
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn write(_: &str, _: Option<&str>, _: &str) -> Result<(), String> {
        Err("Nur unter Windows".into())
    }
    pub fn delete(_: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_first_existing_folder() {
        let dir = std::env::temp_dir().to_string_lossy().into_owned();
        let args = |rest: &[&str]| -> Vec<String> {
            std::iter::once("mrfilesys.exe").chain(rest.iter().copied()).map(String::from).collect()
        };
        assert_eq!(folder_arg(&args(&[])), None);
        assert_eq!(folder_arg(&args(&["C:\\gibt\\es\\nicht"])), None);
        assert_eq!(folder_arg(&args(&["--flag", &dir])), Some(dir.clone()));
        assert_eq!(folder_arg(&args(&["C:\""])), Some("C:\\".into()));
    }
}
