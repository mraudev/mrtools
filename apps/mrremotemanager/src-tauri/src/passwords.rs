//! Remotedesktop passwords in the Windows Credential Manager, as the credential `TERMSRV/<host>`
//! that `mstsc.exe` reads itself. A password never goes back to the frontend and never into
//! the connections file – the frontend only learns whether one is stored.

use crate::launch::{check_host, strip_brackets};
use crate::win;
use std::collections::HashMap;

fn target_host(host: &str) -> Result<&str, String> {
    Ok(strip_brackets(check_host(host)?))
}

/// Whether a password is stored, by host.
#[tauri::command]
pub fn rdp_passwords(hosts: Vec<String>) -> HashMap<String, bool> {
    hosts
        .into_iter()
        .map(|host| {
            let stored = target_host(&host).is_ok_and(win::has_rdp_password);
            (host, stored)
        })
        .collect()
}

#[tauri::command]
pub fn set_rdp_password(host: String, user: String, password: String) -> Result<(), String> {
    let user = user.trim();
    if user.is_empty() {
        return Err("Zum Speichern des Passworts wird ein Benutzername gebraucht.".into());
    }
    if password.is_empty() || password.chars().any(|c| c == '\0') {
        return Err("Ungültiges Passwort.".into());
    }
    win::set_rdp_password(target_host(&host)?, user, &password)
}

#[tauri::command]
pub fn delete_rdp_password(host: String) -> Result<(), String> {
    win::delete_rdp_password(target_host(&host)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes to the real credential store, so only on request: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn store_and_remove_password() {
        let host = "mrremotemanager-test.invalid".to_string();
        set_rdp_password(host.clone(), "tester".into(), "geheim".into()).unwrap();
        assert!(rdp_passwords(vec![host.clone()])[&host]);
        delete_rdp_password(host.clone()).unwrap();
        assert!(!rdp_passwords(vec![host.clone()])[&host]);
        // Deleting again is no error.
        delete_rdp_password(host).unwrap();
    }

    #[test]
    fn needs_a_user_and_a_valid_host() {
        assert!(set_rdp_password("srv".into(), " ".into(), "x".into()).is_err());
        assert!(set_rdp_password("srv\nx".into(), "me".into(), "x".into()).is_err());
    }
}
