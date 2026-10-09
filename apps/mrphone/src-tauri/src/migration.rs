// Umstieg aus der Electron-Version: Sie ist per Benutzer installiert und unter einem festen Schlüssel
// eingetragen (aus ihrer App-ID de.rau.sipphone abgeleitet). Der Installer dieser Version entfernt sie
// (installer-hooks.nsh), die Daten in %APPDATA%\SIP Phone bleiben und werden direkt weiterbenutzt.
// Solange sie noch installiert ist (z. B. Exe aus dem Build-Ordner), arbeitet diese Version auf einer Kopie.
use windows::{
    core::HSTRING,
    Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ},
};

pub const ELECTRON_UNINSTALL_KEY: &str = "bbbb2b48-a2e1-5e29-8a75-0784954697a7";

// Wert aus HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\<key>.
pub fn uninstall_value(key: &str, name: &str) -> Option<String> {
    let subkey = HSTRING::from(format!(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{key}"
    ));
    let name = HSTRING::from(name);
    let mut size = 0u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &subkey,
            &name,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
        .ok()
        .ok()?;
        let mut buf = vec![0u16; (size as usize).div_ceil(2)];
        RegGetValueW(
            HKEY_CURRENT_USER,
            &subkey,
            &name,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
        .ok()
        .ok()?;
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    }
}

pub fn electron_installed() -> bool {
    uninstall_value(ELECTRON_UNINSTALL_KEY, "UninstallString").is_some()
}
