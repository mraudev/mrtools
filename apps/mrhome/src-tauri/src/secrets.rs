//! Secrets (the tado refresh token, the Hue app key) live in the Windows Credential Manager
//! (DPAPI-encrypted, bound to this machine) – never in a file and never in the frontend.

use keyring_core::{Entry, Error};
use std::collections::HashMap;

const SERVICE: &str = "de.mraudev.mrhome";

#[derive(Clone, Copy)]
pub enum Secret {
    TadoRefreshToken,
    HueKey,
}

impl Secret {
    fn entry(self) -> Result<Entry, String> {
        let user = match self {
            Self::TadoRefreshToken => "tado-refresh-token",
            Self::HueKey => "hue-app-key",
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

pub fn get(secret: Secret) -> Option<String> {
    secret.entry().ok()?.get_password().ok()
}

pub fn set(secret: Secret, value: &str) -> Result<(), String> {
    secret.entry()?.set_password(value).map_err(|e| e.to_string())
}

pub fn delete(secret: Secret) -> Result<(), String> {
    match secret.entry()?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
