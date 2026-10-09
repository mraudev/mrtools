// Zugangsdaten verschlüsseln – im selben Format wie Electrons safeStorage unter Windows, damit Electron-
// und Tauri-Version dieselbe config.json lesen können: AES-256-GCM-Schlüssel in "Local State"
// (os_crypt.encrypted_key = Base64("DPAPI" + DPAPI-geschützter Schlüssel)), Werte als
// Base64("v10" + 12 Byte Nonce + Chiffrat + Tag).
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub struct OsCrypt {
    cipher: Option<Aes256Gcm>,
}

impl OsCrypt {
    // Schlüssel aus "Local State" lesen oder neu anlegen. Ohne DPAPI (sollte unter Windows nie vorkommen)
    // bleibt die Verschlüsselung aus – wie bei Electron, wenn safeStorage nicht verfügbar ist.
    pub fn load_or_create(dir: &Path) -> Self {
        let file = dir.join("Local State");
        let mut state: Value = fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| json!({}));
        if let Some(key) = read_key(&state) {
            return Self {
                cipher: Some(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key))),
            };
        }
        let key = Aes256Gcm::generate_key(OsRng);
        let Ok(protected) = dpapi::protect(&key) else {
            return Self { cipher: None };
        };
        let mut stored = b"DPAPI".to_vec();
        stored.extend_from_slice(&protected);
        if !state.is_object() {
            state = json!({});
        }
        state["os_crypt"] = json!({ "encrypted_key": B64.encode(stored) });
        if fs::write(&file, serde_json::to_string(&state).unwrap_or_default()).is_err() {
            return Self { cipher: None };
        }
        Self {
            cipher: Some(Aes256Gcm::new(&key)),
        }
    }

    pub fn available(&self) -> bool {
        self.cipher.is_some()
    }

    pub fn encrypt(&self, plain: &str) -> Option<String> {
        let cipher = self.cipher.as_ref()?;
        let nonce = Aes256Gcm::generate_nonce(OsRng);
        let data = cipher.encrypt(&nonce, plain.as_bytes()).ok()?;
        let mut out = b"v10".to_vec();
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&data);
        Some(B64.encode(out))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String, String> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or("keine Verschlüsselung verfügbar")?;
        let blob = B64.decode(encoded).map_err(|e| e.to_string())?;
        if blob.len() < 3 + 12 + 16 || !blob.starts_with(b"v10") {
            return Err("unbekanntes Format".into());
        }
        let plain = cipher
            .decrypt(Nonce::from_slice(&blob[3..15]), &blob[15..])
            .map_err(|_| "Entschlüsselung fehlgeschlagen".to_string())?;
        String::from_utf8(plain).map_err(|e| e.to_string())
    }
}

fn read_key(state: &Value) -> Option<Vec<u8>> {
    let stored = B64
        .decode(state["os_crypt"]["encrypted_key"].as_str()?)
        .ok()?;
    let key = dpapi::unprotect(stored.strip_prefix(b"DPAPI")?).ok()?;
    (key.len() == 32).then_some(key)
}

mod dpapi {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
    };

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        }
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let data = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _)));
        data
    }

    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(
                &blob(data),
                windows::core::PCWSTR::null(),
                None,
                None,
                None,
                0,
                &mut out,
            )
            .map_err(|e| e.to_string())?;
            Ok(take(out))
        }
    }

    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptUnprotectData(&blob(data), None, None, None, None, 0, &mut out)
                .map_err(|e| e.to_string())?;
            Ok(take(out))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_key_reuse() {
        let dir = std::env::temp_dir().join(format!("mrphone-secrets-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let a = OsCrypt::load_or_create(&dir);
        assert!(a.available());
        let enc = a.encrypt("Pässwort-€-742").unwrap();
        assert!(B64.decode(&enc).unwrap().starts_with(b"v10"));
        // Neu geladen: derselbe Schlüssel aus "Local State"
        let b = OsCrypt::load_or_create(&dir);
        assert_eq!(b.decrypt(&enc).unwrap(), "Pässwort-€-742");
        assert!(b.decrypt("djEwAAAA").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
