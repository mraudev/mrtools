// config.json lesen/schreiben – gleiches Format wie src/config.js der Electron-Version (normalizeConfig).
// Bewusst als JSON-Wert statt fester Struktur: unbekannte Felder (z. B. von neueren Versionen) bleiben erhalten.
use crate::secrets::OsCrypt;
use serde_json::{json, Map, Value};
use std::{fs, path::Path};

pub fn account_defaults() -> Value {
    json!({
        "id": "", "label": "", "displayName": "", "username": "", "domain": "", "authUsername": "",
        "realm": "", "ha1": "", "password": "", "proxy": "", "proxyPort": 5060, "expires": 600,
        "ctiHost": "", "ctiPort": 1337, "ctiUser": ""
    })
}

fn defaults() -> Value {
    json!({
        "accounts": [],
        "audio": { "microphone": "", "speaker": "", "ringer": "", "spkMicrophone": "", "spkSpeaker": "", "volume": 1 },
        "ringtone": null,
        "lockUnregister": true,
        "showOnCall": true,
        "micProcessing": true,
        "hdVoice": true,
        "theme": "system",
        "ringOnHeadset": false,
        "headsetAnswer": false,
        "ringtonePreset": "standard",
        "favorites": []
    })
}

const SECRET_FIELDS: [&str; 2] = ["password", "ha1"];

fn account_keys() -> Vec<String> {
    let mut keys: Vec<String> = account_defaults()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    keys.extend(["passwordEnc", "ha1Enc", "sipPort"].map(String::from));
    keys
}

// Flaches Zusammenführen wie { ...base, ...over } in JavaScript.
pub fn merge(base: &Value, over: &Value) -> Value {
    let mut out = base.as_object().cloned().unwrap_or_default();
    if let Some(over) = over.as_object() {
        for (k, v) in over {
            out.insert(k.clone(), v.clone());
        }
    }
    Value::Object(out)
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub fn normalize(raw: &Value) -> Value {
    let raw = if raw.is_object() {
        raw.clone()
    } else {
        json!({})
    };
    let mut cfg = merge(&defaults(), &raw);
    cfg["audio"] = merge(&defaults()["audio"], &raw["audio"]);
    // Bis Version 1.1.3 stand genau ein Konto direkt in der Konfiguration -> wird das erste Konto.
    let keys = account_keys();
    if !raw["accounts"].is_array() {
        let mut legacy = Map::new();
        for k in &keys {
            if let Some(v) = raw.get(k) {
                legacy.insert(k.clone(), v.clone());
            }
        }
        cfg["accounts"] = if raw["username"].as_str().is_some_and(|s| !s.is_empty()) {
            json!([legacy])
        } else {
            json!([])
        };
    }
    if let Some(obj) = cfg.as_object_mut() {
        for k in &keys {
            obj.remove(k);
        }
    }
    let accounts: Vec<Value> = cfg["accounts"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let mut acc = merge(&account_defaults(), a);
            if text(&acc["id"]).is_empty() {
                acc["id"] = json!(format!("konto-{}", i + 1));
            }
            if text(&acc["label"]).is_empty() {
                let domain = text(&acc["domain"]);
                acc["label"] = json!(if domain.is_empty() {
                    format!("Konto {}", i + 1)
                } else {
                    domain
                });
            }
            acc
        })
        .collect();
    cfg["accounts"] = json!(accounts);
    cfg["favorites"] = json!(normalize_favorites(&raw["favorites"]));
    cfg
}

pub fn normalize_favorites(list: &Value) -> Vec<Value> {
    list.as_array()
        .map(|l| {
            l.iter()
                .map(|f| json!({ "name": text(&f["name"]).trim(), "number": text(&f["number"]).trim() }))
                .filter(|f| !f["number"].as_str().unwrap_or_default().is_empty())
                .collect()
        })
        .unwrap_or_default()
}

// INI wie parseIni in src/config.js: Abschnitte -> (Schlüssel, Wert), Reihenfolge wie in der Datei.
type Ini = Vec<(String, Vec<(String, String)>)>;

fn parse_ini(text: &str) -> Ini {
    let mut sections: Ini = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.len() > 2 && line.starts_with('[') && line.ends_with(']') {
            sections.push((line[1..line.len() - 1].to_string(), Vec::new()));
            continue;
        }
        if let (Some(i), Some(current)) = (line.find('=').filter(|&i| i > 0), sections.last_mut()) {
            current
                .1
                .push((line[..i].to_string(), line[i + 1..].to_string()));
        }
    }
    sections
}

// PhonerLite-Konto aus sipper.ini (ohne Passwort – das ist an die AppGUID gebunden verschlüsselt).
// Aktives Konto steht in [Profile] Profile=<Name>, die Daten im gleichnamigen Abschnitt.
pub fn parse_phonerlite(text: &str) -> Option<Value> {
    let ini = parse_ini(text);
    let get = |section: &[(String, String)], key: &str| {
        section
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
    };
    let profile = ini
        .iter()
        .rev()
        .find(|(name, _)| name == "Profile")
        .and_then(|(_, s)| get(s, "profile"));
    let section = profile
        .as_ref()
        .and_then(|p| ini.iter().rev().find(|(name, _)| name == p))
        .or_else(|| {
            ini.iter()
                .find(|(name, _)| !name.eq_ignore_ascii_case("profile"))
        })?;
    let user_field = get(&section.1, "username").filter(|u| !u.is_empty())?;
    let mut parts = user_field.split('|');
    let username = parts.next().unwrap_or_default().trim().to_string();
    let auth = parts.next().map(str::trim).filter(|a| !a.is_empty());
    let display = get(&section.1, "displayname").unwrap_or_default();
    let display = display
        .trim_start()
        .strip_prefix('"')
        .and_then(|rest| rest.split_once('"'))
        .map(|(name, _)| name.to_string())
        .unwrap_or_default();
    let gateway = get(&section.1, "gateway")
        .unwrap_or_default()
        .trim()
        .to_string();
    let label = if !gateway.is_empty() {
        gateway.clone()
    } else if !section.0.is_empty() {
        section.0.clone()
    } else {
        "PhonerLite".into()
    };
    Some(json!({
        "label": label,
        "displayName": display,
        "authUsername": auth.map(String::from).unwrap_or_else(|| username.clone()),
        "username": username,
        "domain": gateway,
    }))
}

pub fn load(dir: &Path) -> Value {
    let raw = fs::read_to_string(dir.join("config.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({}));
    normalize(&raw)
}

// Verschlüsselte Zugangsdaten (*Enc) im Speicher entschlüsseln.
pub fn load_secrets(cfg: &mut Value, crypt: &OsCrypt) {
    if let Some(accounts) = cfg["accounts"].as_array_mut() {
        for acc in accounts {
            for field in SECRET_FIELDS {
                let enc = text(&acc[format!("{field}Enc")]);
                if enc.is_empty() {
                    continue;
                }
                match crypt.decrypt(&enc) {
                    Ok(plain) => acc[field] = json!(plain),
                    Err(err) => eprintln!(
                        "{}: {field} konnte nicht entschlüsselt werden: {err}",
                        text(&acc["label"])
                    ),
                }
            }
        }
    }
}

// Schreiben: Zugangsdaten nur verschlüsselt, nie im Klartext (wie persist() in src/main.js).
pub fn save(dir: &Path, cfg: &Value, crypt: &OsCrypt) -> std::io::Result<()> {
    let mut stored = cfg.clone();
    if crypt.available() {
        if let Some(accounts) = stored["accounts"].as_array_mut() {
            for acc in accounts {
                for field in SECRET_FIELDS {
                    let obj = acc.as_object_mut().unwrap();
                    obj.remove(&format!("{field}Enc"));
                    let plain = text(obj.get(field).unwrap_or(&Value::Null));
                    if plain.is_empty() {
                        continue;
                    }
                    if let Some(enc) = crypt.encrypt(&plain) {
                        obj.insert(format!("{field}Enc"), json!(enc));
                        obj.insert(field.to_string(), json!(""));
                    }
                }
            }
        }
    }
    let body = serde_json::to_string_pretty(&stored).unwrap_or_default() + "\n";
    fs::write(dir.join("config.json"), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_single_account_becomes_first_account() {
        let cfg = normalize(
            &json!({ "username": "742", "domain": "sip.test", "ha1": "x", "proxyPort": 5060 }),
        );
        assert_eq!(cfg["accounts"][0]["id"], "konto-1");
        assert_eq!(cfg["accounts"][0]["label"], "sip.test");
        assert_eq!(cfg["accounts"][0]["ctiPort"], 1337);
        assert!(cfg.get("username").is_none());
        assert_eq!(cfg["hdVoice"], true);
    }

    #[test]
    fn keeps_unknown_fields_and_explicit_false() {
        let cfg = normalize(
            &json!({ "accounts": [], "hdVoice": false, "zukunft": 1, "audio": { "volume": 0.5 } }),
        );
        assert_eq!(cfg["hdVoice"], false);
        assert_eq!(cfg["zukunft"], 1);
        assert_eq!(cfg["audio"]["volume"], 0.5);
        assert_eq!(cfg["audio"]["ringer"], "");
    }

    #[test]
    fn phonerlite_active_profile() {
        let ini = "[Profile]\r\nProfile=Büro\r\n[Privat]\r\nUsername=1\r\n[Büro]\r\nUSERNAME=742|auth742\r\nDisplayName=\"Michael R\" <sip:742@x>\r\nGateway= sip.test \r\n";
        assert_eq!(
            parse_phonerlite(ini).unwrap(),
            json!({ "label": "sip.test", "displayName": "Michael R", "username": "742", "authUsername": "auth742", "domain": "sip.test" })
        );
        let plain = parse_phonerlite("[Konto]\nUsername=100\n").unwrap();
        assert_eq!(plain["label"], "Konto");
        assert_eq!(plain["authUsername"], "100");
        assert!(parse_phonerlite("[Profile]\nProfile=x\n").is_none());
    }

    #[test]
    fn favorites_trimmed_and_empty_dropped() {
        let cfg = normalize(
            &json!({ "favorites": [{ "name": " A ", "number": " 100 " }, { "name": "B", "number": "" }] }),
        );
        assert_eq!(cfg["favorites"], json!([{ "name": "A", "number": "100" }]));
    }
}
