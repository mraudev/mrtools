// Befehle hinter window.phone (src-tauri/src/bridge.js). Entsprechen den ipcMain-Handlern in src/main.js;
// Telefonie (SIP, Gespräche), CTI, Importe und Sicherung folgen in späteren Stufen.
use crate::{
    config,
    contacts::Contacts,
    history::History,
    phone::{PhoneCmd, PhoneHandle},
    secrets::OsCrypt,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    pub dir: PathBuf,
    pub crypt: OsCrypt,
    pub cfg: Mutex<Value>,
    pub contacts: Mutex<Contacts>,
    pub history: Mutex<History>,
    pub phone: PhoneHandle,
}

const OPTION_KEYS: [&str; 6] = [
    "lockUnregister",
    "showOnCall",
    "micProcessing",
    "hdVoice",
    "ringOnHeadset",
    "headsetAnswer",
];
const RINGTONE_TYPES: [&str; 5] = ["wav", "mp3", "ogg", "m4a", "flac"];
const RINGTONE_MAX_BYTES: u64 = 10 * 1024 * 1024;

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn number(v: &Value) -> u64 {
    match v {
        Value::Number(n) => n.as_u64().unwrap_or(0),
        Value::String(s) => s.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

fn error(msg: impl Into<String>) -> Value {
    json!({ "error": msg.into() })
}

impl AppState {
    fn persist(&self, cfg: &Value) {
        if let Err(err) = config::save(&self.dir, cfg, &self.crypt) {
            crate::logger::warn(&format!(
                "config.json konnte nicht gespeichert werden: {err}"
            ));
        }
    }
}

// Konten fürs Fenster – ohne Zugangsdaten.
fn accounts_view(cfg: &Value) -> Value {
    json!(cfg["accounts"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|a| json!({
            "id": a["id"], "label": a["label"], "displayName": a["displayName"], "username": a["username"],
            "domain": a["domain"], "authUsername": a["authUsername"], "proxy": a["proxy"], "proxyPort": a["proxyPort"],
            "ctiHost": a["ctiHost"], "ctiPort": a["ctiPort"], "ctiUser": a["ctiUser"],
            "hasCredentials": !text(&a["password"]).is_empty() || !text(&a["ha1"]).is_empty(),
        }))
        .collect::<Vec<_>>())
}

fn history_view(state: &AppState) -> Value {
    let contacts = state.contacts.lock().unwrap();
    json!(state
        .history
        .lock()
        .unwrap()
        .entries
        .iter()
        .map(|e| {
            let mut e = e.clone();
            e["contactName"] = json!(contacts.lookup(&text(&e["remoteUri"])));
            e
        })
        .collect::<Vec<_>>())
}

pub fn apply_theme(app: &AppHandle, theme: &str) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_theme(match theme {
            "light" => Some(tauri::Theme::Light),
            "dark" => Some(tauri::Theme::Dark),
            _ => None,
        });
    }
}

#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> Value {
    state.phone.snapshot.lock().unwrap().clone()
}

#[tauri::command]
pub fn command(state: State<'_, AppState>, msg: Value) -> Value {
    match msg["type"].as_str() {
        // „Neu verbinden“ / „Übernehmen“: alle Konten anmelden, auch ruhende
        Some("register") => {
            state.phone.send(PhoneCmd::Register);
            Value::Null
        }
        _ => error("Telefonieren kommt in Stufe 3 der Tauri-Version."),
    }
}

#[tauri::command]
pub fn not_yet(what: String) -> Value {
    error(format!("{what} kommt in Stufe 4 der Tauri-Version."))
}

#[tauri::command]
pub fn get_audio(state: State<'_, AppState>) -> Value {
    state.cfg.lock().unwrap()["audio"].clone()
}

#[tauri::command]
pub fn set_audio(state: State<'_, AppState>, audio: Value) {
    let mut cfg = state.cfg.lock().unwrap();
    cfg["audio"] = config::merge(&cfg["audio"], &audio);
    state.persist(&cfg);
}

#[tauri::command]
pub fn get_options(state: State<'_, AppState>) -> Value {
    let cfg = state.cfg.lock().unwrap();
    let mut out = json!({});
    for k in OPTION_KEYS.iter().chain(["ringtonePreset", "theme"].iter()) {
        out[*k] = cfg[*k].clone();
    }
    out
}

#[tauri::command]
pub fn set_options(app: AppHandle, state: State<'_, AppState>, options: Value) {
    let mut cfg = state.cfg.lock().unwrap();
    for k in OPTION_KEYS {
        if let Some(b) = options[k].as_bool() {
            cfg[k] = json!(b);
        }
    }
    if let Some(theme) = options["theme"]
        .as_str()
        .filter(|t| ["light", "dark", "system"].contains(t))
    {
        cfg["theme"] = json!(theme);
        apply_theme(&app, theme);
    }
    // Welche Töne es gibt, weiß nur die Oberfläche (RINGTONES) – hier nur das Format prüfen.
    if let Some(p) = options["ringtonePreset"]
        .as_str()
        .filter(|p| (1..=20).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_lowercase()))
    {
        cfg["ringtonePreset"] = json!(p);
    }
    state.persist(&cfg);
}

#[tauri::command]
pub fn get_accounts(state: State<'_, AppState>) -> Value {
    accounts_view(&state.cfg.lock().unwrap())
}

// Legt ein Konto an (ohne id) oder ändert ein bestehendes – wie saveAccount() in src/main.js.
#[tauri::command]
pub fn save_account(state: State<'_, AppState>, data: Value) -> Value {
    let field = |name: &str| text(&data[name]).trim().to_string();
    let (username, domain) = (field("username"), field("domain"));
    if username.is_empty() || domain.is_empty() {
        return error("Benutzername und Server sind Pflichtfelder.");
    }
    let mut cfg = state.cfg.lock().unwrap();
    let id = text(&data["id"]);
    let existing = cfg["accounts"].as_array().and_then(|l| {
        l.iter()
            .position(|a| !id.is_empty() && text(&a["id"]) == id)
    });
    let auth_username = field("authUsername");
    let port = |name: &str, default: u64| match number(&data[name]) {
        0 => default,
        p => p,
    };
    let mut changes = json!({
        "label": if field("label").is_empty() { domain.clone() } else { field("label") },
        "displayName": field("displayName"),
        "username": username,
        "domain": domain,
        "authUsername": auth_username,
        "proxy": if field("proxy").is_empty() { domain.clone() } else { field("proxy") },
        "proxyPort": port("proxyPort", 5060),
        "ctiHost": field("ctiHost"),
        "ctiPort": port("ctiPort", 1337),
        "ctiUser": field("ctiUser"),
    });
    if field("ctiHost").is_empty() != field("ctiUser").is_empty() {
        return error(
            "Für den CTI-Server bitte Server und Anmeldename angeben (oder beides leer lassen).",
        );
    }
    let password = text(&data["password"]);
    if !password.is_empty() {
        changes["password"] = json!(password);
        changes["ha1"] = json!("");
        changes["realm"] = json!("");
    } else {
        // Ohne neues Passwort nur, wenn es beim selben Benutzer bleibt und schon Zugangsdaten da sind.
        let ok = existing.is_some_and(|i| {
            let a = &cfg["accounts"][i];
            let user = |auth: &str, name: &str| {
                if auth.is_empty() {
                    name.to_string()
                } else {
                    auth.to_string()
                }
            };
            let same_user = user(&text(&a["authUsername"]), &text(&a["username"]))
                == user(&auth_username, &username);
            same_user && (!text(&a["password"]).is_empty() || !text(&a["ha1"]).is_empty())
        });
        if !ok {
            return error("Bitte das Passwort eingeben.");
        }
    }
    match existing {
        Some(i) => {
            cfg["accounts"][i] = config::merge(&cfg["accounts"][i], &changes);
            state
                .phone
                .send(PhoneCmd::Update(cfg["accounts"][i].clone())); // meldet neu an
        }
        None => {
            let mut account = config::merge(&config::account_defaults(), &changes);
            account["id"] = json!(uuid::Uuid::new_v4().to_string());
            cfg["accounts"]
                .as_array_mut()
                .unwrap()
                .push(account.clone());
            state.phone.send(PhoneCmd::Add(account));
        }
    }
    state.persist(&cfg);
    json!({ "accounts": accounts_view(&cfg) })
}

#[tauri::command]
pub fn delete_account(state: State<'_, AppState>, id: String) -> Value {
    let mut cfg = state.cfg.lock().unwrap();
    cfg["accounts"]
        .as_array_mut()
        .unwrap()
        .retain(|a| text(&a["id"]) != id);
    state.persist(&cfg);
    state.phone.send(PhoneCmd::Remove(id)); // meldet vorher ab
    json!({ "accounts": accounts_view(&cfg) })
}

// --- Klingelton: eigene Datei wird in den Datenordner kopiert ---

fn ringtone_path(state: &AppState, cfg: &Value) -> Option<PathBuf> {
    cfg["ringtone"]["file"].as_str().map(|f| state.dir.join(f))
}

#[tauri::command]
pub fn get_ringtone(state: State<'_, AppState>) -> Value {
    let cfg = state.cfg.lock().unwrap();
    match ringtone_path(&state, &cfg).and_then(|p| std::fs::read(p).ok()) {
        Some(data) => json!({ "name": cfg["ringtone"]["name"], "data": B64.encode(data) }),
        None => Value::Null,
    }
}

#[tauri::command]
pub async fn choose_ringtone(app: AppHandle, state: State<'_, AppState>) -> Result<Value, ()> {
    let picked = app
        .dialog()
        .file()
        .set_title("Klingelton wählen")
        .add_filter("Audiodateien", &RINGTONE_TYPES)
        .blocking_pick_file();
    let Some(src) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(Value::Null);
    };
    if std::fs::metadata(&src).map(|m| m.len()).unwrap_or(u64::MAX) > RINGTONE_MAX_BYTES {
        return Ok(error("Die Datei ist zu groß (max. 10 MB)."));
    }
    let ext = src
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let file = format!("ringtone.{ext}");
    let mut cfg = state.cfg.lock().unwrap();
    if let Some(old) = ringtone_path(&state, &cfg) {
        let _ = std::fs::remove_file(old);
    }
    if let Err(err) = std::fs::copy(&src, state.dir.join(&file)) {
        return Ok(error(err.to_string()));
    }
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    cfg["ringtone"] = json!({ "file": file, "name": name });
    state.persist(&cfg);
    Ok(json!({ "name": name }))
}

#[tauri::command]
pub fn reset_ringtone(state: State<'_, AppState>) {
    let mut cfg = state.cfg.lock().unwrap();
    if let Some(old) = ringtone_path(&state, &cfg) {
        let _ = std::fs::remove_file(old);
    }
    cfg["ringtone"] = Value::Null;
    state.persist(&cfg);
}

#[tauri::command]
pub fn get_version() -> String {
    format!("{} (Tauri-Test)", env!("CARGO_PKG_VERSION"))
}

// Bis es ein eigenes Protokoll gibt (Stufe 5): den Datenordner im Explorer öffnen.
#[tauri::command]
pub fn open_data_dir(state: State<'_, AppState>) {
    let _ = std::process::Command::new("explorer")
        .arg(&state.dir)
        .spawn();
}

// --- Telefonbuch, Verlauf, Kurzwahl ---

#[tauri::command]
pub fn get_contacts(state: State<'_, AppState>) -> Value {
    state.contacts.lock().unwrap().view()
}

fn contacts_changed(app: &AppHandle, state: &AppState) {
    let _ = app.emit(
        "phone:contactsChanged",
        state.contacts.lock().unwrap().view(),
    );
    let _ = app.emit("phone:historyChanged", history_view(state));
}

#[tauri::command]
pub fn save_contact(app: AppHandle, state: State<'_, AppState>, data: Value) -> Value {
    let result = state.contacts.lock().unwrap().upsert(&data);
    match result {
        Ok(_) => {
            contacts_changed(&app, &state);
            Value::Null
        }
        Err(msg) => error(msg),
    }
}

#[tauri::command]
pub fn delete_contact(app: AppHandle, state: State<'_, AppState>, id: String) {
    let _ = state.contacts.lock().unwrap().remove(&id);
    contacts_changed(&app, &state);
}

#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Value {
    history_view(&state)
}

#[tauri::command]
pub fn clear_history(app: AppHandle, state: State<'_, AppState>) {
    let _ = state.history.lock().unwrap().clear();
    let _ = app.emit("phone:historyChanged", history_view(&state));
}

#[tauri::command]
pub fn get_favorites(state: State<'_, AppState>) -> Value {
    let presence = state.phone.presence.lock().unwrap().clone();
    json!({ "list": state.cfg.lock().unwrap()["favorites"], "presence": presence })
}

#[tauri::command]
pub fn save_favorites(state: State<'_, AppState>, list: Value) -> Value {
    let mut cfg = state.cfg.lock().unwrap();
    let favorites = config::normalize_favorites(&list);
    let numbers = favorites.iter().map(|f| text(&f["number"])).collect();
    cfg["favorites"] = json!(favorites);
    state.persist(&cfg);
    state.phone.send(PhoneCmd::SetFavorites(numbers));
    json!({ "list": cfg["favorites"] })
}
