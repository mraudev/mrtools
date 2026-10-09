// Befehle hinter window.phone (src-tauri/src/bridge.js). Entsprechen den ipcMain-Handlern in src/main.js.
use crate::{
    backup, config,
    contacts::Contacts,
    cti,
    history::History,
    imports,
    phone::{PhoneCmd, PhoneHandle},
    secrets::OsCrypt,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    pub dir: PathBuf,
    pub crypt: OsCrypt,
    pub cfg: Mutex<Value>,
    pub contacts: Mutex<Contacts>,
    pub history: Mutex<History>,
    pub phone: PhoneHandle,
    pub cti: Mutex<Vec<(String, cti::Client)>>, // Konto-ID -> Client (nur Konten mit CTI-Server)
    pub cti_pending: AtomicBool,
    pub backup_file: Mutex<Option<PathBuf>>, // gewählte Sicherung, bis das Passwort eingegeben ist
    pub test_copy: bool,                     // Testkopie neben der installierten Electron-Version
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

// Befehle aus der Oberfläche (wie runCommand in src/main.js).
#[tauri::command]
pub async fn command(state: State<'_, AppState>, msg: Value) -> Result<Value, ()> {
    let target = text(&msg["target"]);
    let phone = &state.phone;
    match msg["type"].as_str().unwrap_or("") {
        "dial" => {
            let account = msg["accountId"].as_str().map(String::from);
            if let Err(err) = phone.dial(target, account).await {
                return Ok(error(err));
            }
        }
        "answer" => phone.send(PhoneCmd::Answer),
        "reject" => phone.send(PhoneCmd::Reject),
        "hangup" => phone.send(PhoneCmd::Hangup),
        "dtmf" => phone.send(PhoneCmd::Dtmf(text(&msg["digit"]))),
        "hold" => phone.send(PhoneCmd::Hold(msg["on"].as_bool().unwrap_or(false))),
        "transfer" => phone.send(PhoneCmd::Transfer(target)),
        "attendedTransfer" => phone.send(PhoneCmd::AttendedTransfer(target)),
        "completeTransfer" => phone.send(PhoneCmd::CompleteTransfer),
        "cancelConsult" => phone.send(PhoneCmd::CancelConsult),
        // „Neu verbinden“ / „Übernehmen“: alle Konten anmelden, auch ruhende
        "register" => {
            phone.send(PhoneCmd::Register);
            state
                .cti
                .lock()
                .unwrap()
                .iter()
                .for_each(|(_, c)| c.retry());
        }
        kind @ ("dnd" | "away") => {
            if let Err(err) = set_cti_flag(&state, kind, msg["on"].as_bool().unwrap_or(false)) {
                return Ok(error(err));
            }
        }
        _ => {}
    }
    Ok(Value::Null)
}

// --- CTI-Server (optional je Konto): Nicht stören, Abwesend, Status der Kurzwahl, Konferenzteilnehmer ---

// Verbindungen passend zu den Konten starten/stoppen; neu verbunden wird nur bei geänderten Angaben.
pub fn sync_cti(app: &AppHandle) {
    let state = app.state::<AppState>();
    let accounts = state.cfg.lock().unwrap()["accounts"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let wanted = |a: &Value| {
        let (host, user) = (text(&a["ctiHost"]), text(&a["ctiUser"]));
        let port = match number(&a["ctiPort"]) {
            0 => 1337,
            p => p as u16,
        };
        (!host.is_empty() && !user.is_empty()).then_some((host, port, user))
    };
    {
        let mut ctis = state.cti.lock().unwrap();
        ctis.retain(|(id, c)| {
            let keep = accounts
                .iter()
                .find(|a| text(&a["id"]) == *id)
                .and_then(wanted)
                .is_some_and(|(h, p, u)| h == c.host && p == c.port && u == c.user);
            if !keep {
                c.stop();
            }
            keep
        });
        for a in &accounts {
            let id = text(&a["id"]);
            let Some((host, port, user)) = wanted(a) else {
                continue;
            };
            if ctis.iter().any(|(i, _)| *i == id) {
                continue;
            }
            let handle = app.clone();
            let on_change = Arc::new(move || cti_changed(&handle));
            ctis.push((
                id,
                cti::Client::start(host, port, user, text(&a["label"]), on_change),
            ));
        }
    }
    cti_changed(app);
}

// Nach der Anmeldung kommt je Telefon ein Event -> gesammelt (50 ms) ans Fenster schicken.
fn cti_changed(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if state.cti_pending.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let state = app.state::<AppState>();
        state.cti_pending.store(false, Ordering::SeqCst);
        let _ = app.emit("phone:cti", cti_view(&state));
        crate::desktop::update_tray(&app);
    });
}

// Nicht stören an einem der verbundenen CTI-Server (für den Tray-Text).
pub fn own_dnd(state: &AppState) -> bool {
    state
        .cti
        .lock()
        .unwrap()
        .iter()
        .any(|(_, c)| c.state.lock().unwrap().own().is_some_and(|o| o.dnd))
}

fn cti_view(state: &AppState) -> Value {
    let favorites: Vec<String> = state.cfg.lock().unwrap()["favorites"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|f| text(&f["number"]))
        .collect();
    let ctis = state.cti.lock().unwrap();
    let contacts = state.contacts.lock().unwrap();
    cti::view(&ctis, &favorites, &|n| contacts.lookup(n))
}

// Nicht stören / Abwesend: gilt für alle Konten, deren CTI-Server verbunden ist.
fn set_cti_flag(state: &AppState, kind: &str, on: bool) -> Result<(), String> {
    let ctis = state.cti.lock().unwrap();
    let connected: Vec<&cti::Client> = ctis
        .iter()
        .map(|(_, c)| c)
        .filter(|c| c.connected())
        .collect();
    if connected.is_empty() {
        return Err("CTI-Server nicht verbunden".into());
    }
    for c in connected {
        if kind == "dnd" {
            c.set_dnd(on);
        } else {
            c.set_away(on);
        }
    }
    Ok(())
}

pub fn stop_cti(state: &AppState) {
    state.cti.lock().unwrap().iter().for_each(|(_, c)| c.stop());
}

#[tauri::command]
pub fn get_cti(state: State<'_, AppState>) -> Value {
    cti_view(&state)
}

// Mikrofon-Audio aus der Oberfläche: Int16 (Little Endian) als Binärpaket.
#[tauri::command]
pub fn audio_in(state: State<'_, AppState>, request: tauri::ipc::Request<'_>) {
    if let tauri::ipc::InvokeBody::Raw(bytes) = request.body() {
        let pcm = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b))
            .collect();
        state.phone.send(PhoneCmd::Audio(pcm));
    }
}

// Sprache der Gegenstelle zur Oberfläche: ein binärer Kanal für die ganze Laufzeit.
#[tauri::command]
pub fn audio_subscribe(
    state: State<'_, AppState>,
    channel: tauri::ipc::Channel<tauri::ipc::InvokeResponseBody>,
) {
    *state.phone.audio_out.lock().unwrap() = Some(channel);
}

// Verbindung SIP-Thread -> App: Namen aus dem Telefonbuch, Verlaufseinträge nach Gesprächsende.
pub struct AppHooks(pub AppHandle);

impl crate::phone::Hooks for AppHooks {
    fn contact_name(&self, uri: &str) -> Option<String> {
        self.0
            .try_state::<AppState>()?
            .contacts
            .lock()
            .unwrap()
            .lookup(uri)
    }

    fn state_changed(&self, snapshot: &Value) {
        crate::desktop::on_state(&self.0, snapshot);
    }

    fn call_ended(&self, reason: &str, call: Value) {
        let Some(state) = self.0.try_state::<AppState>() else {
            return;
        };
        let _ = self.0.emit("phone:ended", reason);
        let entry = state.history.lock().unwrap().add(reason, &call);
        let _ = self.0.emit("phone:historyChanged", history_view(&state));
        if let Some(entry) = entry.ok().filter(|e| e["status"] == "missed") {
            crate::desktop::notify_missed(&self.0, &entry);
        }
    }
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
    for k in OPTION_KEYS
        .iter()
        .chain(["ringtonePreset", "theme", "design"].iter())
    {
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
    if let Some(hd) = options["hdVoice"].as_bool() {
        state.phone.send(PhoneCmd::SetHd(hd));
    }
    if let Some(theme) = options["theme"]
        .as_str()
        .filter(|t| ["light", "dark", "system"].contains(t))
    {
        cfg["theme"] = json!(theme);
        apply_theme(&app, theme);
    }
    // Design: „mrtools“ wie die anderen mr-Apps oder das klassische
    if let Some(design) = options["design"]
        .as_str()
        .filter(|d| ["mr", "classic"].contains(d))
    {
        cfg["design"] = json!(design);
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
pub fn save_account(app: AppHandle, state: State<'_, AppState>, data: Value) -> Value {
    let result = store_account(&state, &data);
    sync_cti(&app);
    result
}

fn store_account(state: &AppState, data: &Value) -> Value {
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
pub fn delete_account(app: AppHandle, state: State<'_, AppState>, id: String) -> Value {
    let view = {
        let mut cfg = state.cfg.lock().unwrap();
        cfg["accounts"]
            .as_array_mut()
            .unwrap()
            .retain(|a| text(&a["id"]) != id);
        state.persist(&cfg);
        accounts_view(&cfg)
    };
    state.phone.send(PhoneCmd::Remove(id)); // meldet vorher ab
    sync_cti(&app);
    json!({ "accounts": view })
}

// PhonerLite-Konto aus einer gewählten sipper.ini vorbelegen (ohne Passwort – das ist verschlüsselt).
#[tauri::command]
pub async fn import_phonerlite(app: AppHandle) -> Result<Value, ()> {
    let picked = app
        .dialog()
        .file()
        .set_title("PhonerLite-Konfiguration (sipper.ini) wählen")
        .add_filter("PhonerLite-Konfiguration", &["ini"])
        .blocking_pick_file();
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(Value::Null);
    };
    Ok(match std::fs::read(&path) {
        Err(err) => error(err.to_string()),
        Ok(bytes) => match config::parse_phonerlite(&String::from_utf8_lossy(&bytes)) {
            Some(account) => json!({ "account": account }),
            None => error("In der Datei wurde kein SIP-Konto gefunden."),
        },
    })
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
pub fn get_version(state: State<'_, AppState>) -> String {
    if state.test_copy {
        format!("{} (Tauri-Test)", env!("CARGO_PKG_VERSION"))
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

// Protokoll im Explorer zeigen (Datei markiert), damit man es weitergeben kann.
#[tauri::command]
pub fn open_log(state: State<'_, AppState>) {
    use std::os::windows::process::CommandExt;
    let file = state.dir.join("sipphone.log");
    let _ = std::process::Command::new("explorer")
        .raw_arg(format!("/select,\"{}\"", file.display()))
        .spawn();
}

// Fehler der Headset-Anbindung (WebHID in der Oberfläche) ins Protokoll.
#[tauri::command]
pub fn log_headset(text: String) {
    let text: String = text.chars().take(300).collect();
    crate::logger::warn(&format!("[Headset] {text}"));
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
        Ok(contact) => {
            contacts_changed(&app, &state);
            // Mit vorhandenem Kontakt gleichen Namens zusammengeführt: der Oberfläche Bescheid geben
            match contact.get("merged") {
                Some(added) => json!({ "merged": contact["name"], "added": added }),
                None => Value::Null,
            }
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
pub fn save_favorites(app: AppHandle, state: State<'_, AppState>, list: Value) -> Value {
    let mut cfg = state.cfg.lock().unwrap();
    let favorites = config::normalize_favorites(&list);
    let numbers = favorites.iter().map(|f| text(&f["number"])).collect();
    cfg["favorites"] = json!(favorites);
    state.persist(&cfg);
    state.phone.send(PhoneCmd::SetFavorites(numbers));
    cti_changed(&app);
    json!({ "list": cfg["favorites"] })
}

// --- Kontakte importieren/exportieren ---

fn merge_imported(app: &AppHandle, state: &AppState, found: &[Value], source: &str) -> Value {
    let result = state.contacts.lock().unwrap().merge(found, source);
    contacts_changed(app, state);
    match result {
        Ok(mut r) => {
            r["found"] = json!(found.len());
            r
        }
        Err(err) => error(err.to_string()),
    }
}

#[tauri::command]
pub async fn import_outlook(app: AppHandle, state: State<'_, AppState>) -> Result<Value, ()> {
    Ok(match imports::outlook_contacts().await {
        Err(err) => error(err),
        Ok(found) if found.is_empty() => error("Im klassischen Outlook wurden keine Kontakte mit Telefonnummer gefunden. Liegen sie im neuen Outlook oder bei Outlook.com, dort als CSV exportieren und die Datei importieren."),
        Ok(found) => merge_imported(&app, &state, &found, "outlook"),
    })
}

#[tauri::command]
pub async fn import_csv(app: AppHandle, state: State<'_, AppState>) -> Result<Value, ()> {
    let picked = app
        .dialog()
        .file()
        .set_title("Kontakte-CSV wählen")
        .add_filter("CSV-Dateien", &["csv", "txt"])
        .blocking_pick_file();
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(Value::Null);
    };
    let found = std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| imports::read_contacts_csv(&bytes));
    Ok(match found {
        Err(err) => error(err),
        Ok(found) if found.is_empty() => {
            error("In der Datei wurden keine Kontakte mit Telefonnummer gefunden.")
        }
        Ok(found) => merge_imported(&app, &state, &found, "csv"),
    })
}

#[tauri::command]
pub async fn export_csv(app: AppHandle, state: State<'_, AppState>) -> Result<Value, ()> {
    let picked = app
        .dialog()
        .file()
        .set_title("Kontakte als CSV exportieren")
        .set_file_name("kontakte.csv")
        .add_filter("CSV-Dateien", &["csv"])
        .blocking_save_file();
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(Value::Null);
    };
    let (csv, count) = {
        let contacts = state.contacts.lock().unwrap();
        (
            imports::contacts_to_csv(&contacts.entries),
            contacts.entries.len(),
        )
    };
    // BOM voran, damit Excel Umlaute als UTF-8 erkennt.
    Ok(match std::fs::write(&path, format!("\u{feff}{csv}")) {
        Ok(()) => json!({ "count": count }),
        Err(err) => error(err.to_string()),
    })
}

// --- Sicherung: alles verschlüsselt exportieren und auf einem anderen mrphone wieder einspielen ---

// .sipphone: Sicherungen von vor der Umbenennung (gleiches Format)
const BACKUP_TYPES: [&str; 2] = ["mrphone", "sipphone"];

#[tauri::command]
pub async fn export_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    password: String,
) -> Result<Value, ()> {
    if password.chars().count() < backup::MIN_PASSWORD {
        return Ok(error(format!(
            "Das Passwort braucht mindestens {} Zeichen.",
            backup::MIN_PASSWORD
        )));
    }
    let created = backup::iso_now();
    let picked = app
        .dialog()
        .file()
        .set_title("Sicherung speichern")
        .set_file_name(format!("mrphone-Sicherung-{}.mrphone", &created[..10]))
        .add_filter("mrphone-Sicherung", &BACKUP_TYPES)
        .blocking_save_file();
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(Value::Null);
    };
    let payload = {
        let cfg = state.cfg.lock().unwrap();
        let mut settings = cfg.clone();
        let obj = settings.as_object_mut().unwrap();
        obj.remove("audio"); // Audiogeräte heißen auf jedem PC anders -> bleiben außen vor
                             // Zugangsdaten im Klartext – geschützt durch die Verschlüsselung der Sicherung. Die DPAPI-Felder
                             // (*Enc) taugen auf einem anderen PC nicht.
        for a in obj
            .get_mut("accounts")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            if let Some(a) = a.as_object_mut() {
                a.remove("passwordEnc");
                a.remove("ha1Enc");
            }
        }
        let ringtone = ringtone_path(&state, &cfg).and_then(|p| {
            let data = std::fs::read(&p).ok()?;
            let ext = p.extension()?.to_string_lossy().to_string();
            Some(json!({ "name": cfg["ringtone"]["name"], "ext": ext, "data": B64.encode(data) }))
        });
        json!({
            "app": "mrphone",
            "version": env!("CARGO_PKG_VERSION"),
            "createdAt": created,
            "config": settings,
            "contacts": state.contacts.lock().unwrap().entries,
            "history": state.history.lock().unwrap().entries,
            "ringtone": ringtone,
        })
    };
    let counts = (
        payload["config"]["accounts"].as_array().map_or(0, Vec::len),
        payload["contacts"].as_array().map_or(0, Vec::len),
        payload["history"].as_array().map_or(0, Vec::len),
    );
    let sealed = tauri::async_runtime::spawn_blocking(move || backup::encrypt(&payload, &password))
        .await
        .unwrap_or_else(|e| Err(e.to_string()));
    let result = sealed.and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
    Ok(match result {
        Ok(()) => {
            crate::logger::info(&format!(
                "Sicherung exportiert: Konten {}, Kontakte {}, Verlaufseinträge {}",
                counts.0, counts.1, counts.2
            ));
            json!({ "file": path.file_name().map(|n| n.to_string_lossy().to_string()) })
        }
        Err(err) => error(err),
    })
}

#[tauri::command]
pub async fn choose_backup(app: AppHandle, state: State<'_, AppState>) -> Result<Value, ()> {
    let picked = app
        .dialog()
        .file()
        .set_title("Sicherung wählen")
        .add_filter("mrphone-Sicherung", &BACKUP_TYPES)
        .blocking_pick_file();
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(Value::Null);
    };
    let name = path.file_name().map(|n| n.to_string_lossy().to_string());
    *state.backup_file.lock().unwrap() = Some(path);
    Ok(json!({ "file": name }))
}

// Ersetzt Konten, Kontakte, Kurzwahl, Verlauf und Einstellungen durch die Sicherung (Audiogeräte bleiben).
#[tauri::command]
pub async fn import_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    password: String,
) -> Result<Value, ()> {
    let Some(path) = state.backup_file.lock().unwrap().clone() else {
        return Ok(error("Bitte zuerst eine Sicherung wählen."));
    };
    if !state.phone.snapshot.lock().unwrap()["call"].is_null() {
        return Ok(error("Während eines Gesprächs nicht möglich."));
    }
    let data = tauri::async_runtime::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        backup::decrypt(&bytes, &password)
    })
    .await
    .unwrap_or_else(|e| Err(e.to_string()));
    let data = match data {
        Ok(d) => d,
        Err(err) => return Ok(error(err)),
    };
    if !data["config"]["accounts"].is_array() {
        return Ok(error("Die Sicherung ist unvollständig."));
    }
    *state.backup_file.lock().unwrap() = None;

    let (accounts, favorites, theme, hd) = {
        let mut cfg = state.cfg.lock().unwrap();
        for a in cfg["accounts"].as_array().cloned().unwrap_or_default() {
            state.phone.send(PhoneCmd::Remove(text(&a["id"]))); // meldet ab
        }
        let mut next = config::normalize(&config::merge(
            &data["config"],
            &json!({ "audio": cfg["audio"] }),
        ));
        if let Some(old) = ringtone_path(&state, &cfg) {
            let _ = std::fs::remove_file(old);
        }
        next["ringtone"] = Value::Null;
        let rt = &data["ringtone"];
        let ext = text(&rt["ext"]);
        if let (true, Some(b64)) = (RINGTONE_TYPES.contains(&ext.as_str()), rt["data"].as_str()) {
            let buf = B64.decode(b64).unwrap_or_default();
            if !buf.is_empty() && buf.len() as u64 <= RINGTONE_MAX_BYTES {
                let file = format!("ringtone.{ext}");
                if std::fs::write(state.dir.join(&file), buf).is_ok() {
                    let name = if text(&rt["name"]).is_empty() {
                        file.clone()
                    } else {
                        text(&rt["name"])
                    };
                    next["ringtone"] = json!({ "file": file, "name": name });
                }
            }
        }
        *cfg = next;
        state.persist(&cfg); // Zugangsdaten auf diesem PC wieder per DPAPI verschlüsselt
        (
            cfg["accounts"].as_array().cloned().unwrap_or_default(),
            cfg["favorites"].as_array().cloned().unwrap_or_default(),
            text(&cfg["theme"]),
            cfg["hdVoice"].as_bool().unwrap_or(true),
        )
    };
    let _ = state.contacts.lock().unwrap().replace(&data["contacts"]);
    let _ = state
        .history
        .lock()
        .unwrap()
        .replace(data["history"].as_array().cloned().unwrap_or_default());

    state.phone.presence.lock().unwrap().clear();
    apply_theme(&app, &theme);
    state.phone.send(PhoneCmd::SetHd(hd));
    for account in &accounts {
        state.phone.send(PhoneCmd::Add(account.clone()));
    }
    state.phone.send(PhoneCmd::SetFavorites(
        favorites.iter().map(|f| text(&f["number"])).collect(),
    ));
    sync_cti(&app);
    let contacts = state.contacts.lock().unwrap().entries.len();
    crate::logger::info(&format!(
        "Sicherung importiert: Konten {}, Kontakte {contacts}, Verlaufseinträge {}",
        accounts.len(),
        state.history.lock().unwrap().entries.len()
    ));
    Ok(json!({ "accounts": accounts.len(), "contacts": contacts }))
}
