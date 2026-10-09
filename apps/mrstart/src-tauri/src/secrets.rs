//! API tokens live in the Windows Credential Manager (DPAPI-encrypted, bound
//! to this machine) – never in the config file. The frontend can only store,
//! delete or ask whether a token exists; it can never read one back.

use keyring_core::{Entry, Error};
use serde::Serialize;
use std::collections::HashMap;

const SERVICE: &str = "de.mraudev.mrstart";

/// Names of the tokens the app knows about.
#[derive(Clone, Copy)]
pub enum Secret {
    GitHub,
    Gitea,
}

impl Secret {
    fn parse(name: &str) -> Result<Self, String> {
        match name {
            "github" => Ok(Self::GitHub),
            "gitea" => Ok(Self::Gitea),
            _ => Err(format!("Unbekanntes Token: {name}")),
        }
    }

    fn entry(self) -> Result<Entry, String> {
        let user = match self {
            Self::GitHub => "github-token",
            Self::Gitea => "gitea-token",
        };
        // "Local": stays on this machine instead of roaming with the profile.
        let modifiers = HashMap::from([("persistence", "Local")]);
        Entry::new_with_modifiers(SERVICE, user, &modifiers).map_err(|e| e.to_string())
    }
}

/// Registers the Windows credential store; must run before any other call.
pub fn init() -> Result<(), String> {
    let store = windows_native_keyring_store::Store::new().map_err(|e| e.to_string())?;
    keyring_core::set_default_store(store);
    Ok(())
}

/// Reads a token for use inside the backend. Not exposed as a command.
pub fn get(secret: Secret) -> Option<String> {
    secret.entry().ok()?.get_password().ok()
}

#[derive(Serialize)]
pub struct SecretStatus {
    github: bool,
    gitea: bool,
}

#[tauri::command]
pub fn secret_status() -> SecretStatus {
    SecretStatus {
        github: get(Secret::GitHub).is_some(),
        gitea: get(Secret::Gitea).is_some(),
    }
}

#[tauri::command]
pub fn set_secret(name: String, value: String) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        return Err("Ungültiges Token.".into());
    }
    Secret::parse(&name)?
        .entry()?
        .set_password(value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_secret(name: String) -> Result<(), String> {
    match Secret::parse(&name)?.entry()?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
