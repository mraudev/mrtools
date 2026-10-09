// Verschlüsselte Sicherung aller Daten – gleiches Format wie src/backup.js der Electron-Version, damit
// Sicherungen in beide Richtungen eingespielt werden können. Schlüssel per scrypt aus einem frei gewählten
// Passwort, Inhalt mit AES-256-GCM; die Kopfdaten gehen als AAD ein (verändern macht die Datei unlesbar).
use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng, Payload},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::Value;

const FORMAT: &str = "sipphone-backup"; // Kennung bleibt trotz Umbenennung in mrphone (alte Sicherungen)
const VERSION: u64 = 1;
const LOG_N: u8 = 16; // N = 2^16, r = 8, p = 1 wie die Electron-Version
pub const MIN_PASSWORD: usize = 8;
pub const MAX_FILE: u64 = 64 * 1024 * 1024;

fn derive_key(password: &str, salt: &[u8], log_n: u8) -> Result<[u8; 32], String> {
    let params = scrypt::Params::new(log_n, 8, 1, 32).map_err(|e| e.to_string())?;
    let mut key = [0u8; 32];
    scrypt::scrypt(password.as_bytes(), salt, &params, &mut key).map_err(|e| e.to_string())?;
    Ok(key)
}

// Genau wie JSON.stringify(header(...)) in backup.js – die Bytes müssen übereinstimmen (AAD).
fn header(n: u64, salt: &str, iv: &str) -> String {
    format!(
        r#"{{"format":"{FORMAT}","version":{VERSION},"kdf":{{"name":"scrypt","N":{n},"r":8,"p":1,"salt":"{salt}"}},"iv":"{iv}"}}"#
    )
}

pub fn encrypt(payload: &Value, password: &str) -> Result<Vec<u8>, String> {
    encrypt_with(payload, password, LOG_N)
}

fn encrypt_with(payload: &Value, password: &str, log_n: u8) -> Result<Vec<u8>, String> {
    if password.chars().count() < MIN_PASSWORD {
        return Err(format!(
            "Das Passwort braucht mindestens {MIN_PASSWORD} Zeichen."
        ));
    }
    let (mut salt, mut iv) = ([0u8; 16], [0u8; 12]);
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut iv);
    let head = header(1 << log_n, &B64.encode(salt), &B64.encode(iv));
    let key = derive_key(password, &salt, log_n)?;
    let plain = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let mut sealed = Aes256Gcm::new(&key.into())
        .encrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: &plain,
                aad: head.as_bytes(),
            },
        )
        .map_err(|_| "Verschlüsselung fehlgeschlagen".to_string())?;
    let tag = sealed.split_off(sealed.len() - 16);
    // {...head, tag, data} wie in backup.js
    Ok(format!(
        r#"{},"tag":"{}","data":"{}"}}"#,
        &head[..head.len() - 1],
        B64.encode(tag),
        B64.encode(sealed)
    )
    .into_bytes())
}

pub fn decrypt(bytes: &[u8], password: &str) -> Result<Value, String> {
    if bytes.len() as u64 > MAX_FILE {
        return Err("Die Datei ist zu groß für eine mrphone-Sicherung.".into());
    }
    let file: Value = serde_json::from_slice(bytes).unwrap_or(Value::Null);
    if file["format"] != FORMAT {
        return Err("Das ist keine mrphone-Sicherung.".into());
    }
    if file["version"].as_u64() != Some(VERSION) {
        return Err(
            "Diese Sicherung stammt von einer neueren mrphone-Version – bitte erst aktualisieren."
                .into(),
        );
    }
    let kdf = &file["kdf"];
    // Nur plausible scrypt-Werte zulassen, sonst könnte eine präparierte Datei Speicher/Zeit ausreizen.
    let log_n = match kdf["N"].as_u64() {
        Some(16384) => 14,
        Some(32768) => 15,
        Some(65536) => 16,
        Some(131072) => 17,
        _ => 0,
    };
    if kdf["name"] != "scrypt"
        || log_n == 0
        || kdf["r"].as_u64() != Some(8)
        || kdf["p"].as_u64() != Some(1)
    {
        return Err("Die Sicherung ist beschädigt (unbekannte Verschlüsselungsparameter).".into());
    }
    let field = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let (salt_b64, iv_b64) = (field(&kdf["salt"]), field(&file["iv"]));
    let decode = |s: &str| B64.decode(s).unwrap_or_default();
    let (salt, iv, tag) = (
        decode(&salt_b64),
        decode(&iv_b64),
        decode(&field(&file["tag"])),
    );
    if salt.len() != 16 || iv.len() != 12 || tag.len() != 16 {
        return Err("Die Sicherung ist beschädigt.".into());
    }
    let key = derive_key(password, &salt, log_n)?;
    let mut sealed = decode(&field(&file["data"]));
    sealed.extend_from_slice(&tag);
    let head = header(1 << log_n, &salt_b64, &iv_b64);
    let plain = Aes256Gcm::new(&key.into())
        .decrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: &sealed,
                aad: head.as_bytes(),
            },
        )
        .map_err(|_| "Falsches Passwort oder die Datei wurde verändert.".to_string())?;
    serde_json::from_slice(&plain).map_err(|e| e.to_string())
}

// Zeitpunkt wie new Date().toISOString() (UTC).
pub fn iso_now() -> String {
    let t = unsafe { windows::Win32::System::SystemInformation::GetSystemTime() };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn roundtrip_and_errors() {
        let payload =
            json!({ "app": "mrphone", "config": { "accounts": [{ "password": "geheim ä" }] } });
        let file = encrypt_with(&payload, "passwort1", 14).unwrap();
        assert_eq!(decrypt(&file, "passwort1").unwrap(), payload);
        assert_eq!(
            decrypt(&file, "falsch123").unwrap_err(),
            "Falsches Passwort oder die Datei wurde verändert."
        );
        // Kopfdaten verändert (AAD) -> unlesbar
        let text = String::from_utf8(file).unwrap();
        let mut v: Value = serde_json::from_str(&text).unwrap();
        v["kdf"]["N"] = json!(32768);
        assert!(decrypt(v.to_string().as_bytes(), "passwort1").is_err());
        assert!(encrypt(&payload, "kurz").is_err());
        assert_eq!(
            decrypt(b"{}", "x").unwrap_err(),
            "Das ist keine mrphone-Sicherung."
        );
        assert!(decrypt(br#"{"format":"sipphone-backup","version":1,"kdf":{"name":"scrypt","N":1048576,"r":8,"p":1}}"#, "x")
            .unwrap_err()
            .contains("unbekannte"));
    }

    // Mit der Electron-Version (src/backup.js, encryptBackup) erstellt: Passwort "passwort1",
    // Inhalt {"app":"mrphone","n":"Jörg"}.
    #[test]
    fn reads_electron_backup() {
        let file = include_bytes!("../testdata/electron-backup.mrphone");
        assert_eq!(
            decrypt(file, "passwort1").unwrap(),
            json!({ "app": "mrphone", "n": "Jörg" })
        );
    }
}
