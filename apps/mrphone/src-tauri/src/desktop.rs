// Windows-Anbindung wie in src/main.js der Electron-Version: Tray, Windows-Meldungen (Anruf mit Annehmen/
// Ablehnen, verpasster Anruf), Fenster hervorholen, Bildschirmsperre, eigene Titelleiste.
// Selbsttests (MRPHONE_DATA_DIR gesetzt) zeigen keine Meldungen und holen das Fenster nicht hervor –
// sie schreiben stattdessen eine Zeile ins Protokoll.
use crate::{commands::AppState, phone::PhoneCmd};
use serde_json::Value;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, UserAttentionType, WebviewWindow, WindowEvent,
};
use windows::{
    core::{IInspectable, Interface, HSTRING},
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{ToastActivatedEventArgs, ToastNotification, ToastNotificationManager},
};

// Gleich wie identifier in tauri.conf.json – Windows ordnet Meldungen damit der App zu.
const AUMID: &str = "de.mraudev.mrphone";

const REG_TEXT: [(&str, &str); 7] = [
    ("registered", "Verbunden"),
    ("registering", "Verbinde …"),
    ("unregistering", "Melde ab …"),
    ("unregistered", "Abgemeldet"),
    ("failed", "Nicht verbunden"),
    ("locked", "Abgemeldet (PC gesperrt)"),
    ("elsewhere", "An anderem Gerät angemeldet"),
];

pub struct Desktop {
    quiet: bool,
    tray: Mutex<Option<TrayIcon>>,
    flashing: AtomicBool,
    in_call: AtomicBool,
    tray_hint_shown: AtomicBool,
    screen_locked: AtomicBool,
    call_toast: Mutex<Option<ToastNotification>>,
    missed_toast: Mutex<Option<ToastNotification>>,
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn number_of(uri: &str) -> String {
    let lower = uri.to_lowercase();
    let rest = ["sips:", "sip:", "tel:"]
        .iter()
        .find(|s| lower.starts_with(*s))
        .map_or(uri, |s| &uri[s.len()..]);
    rest.split('@').next().unwrap_or_default().to_string()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn quiet() -> bool {
    std::env::var_os("MRPHONE_DATA_DIR").is_some()
}

impl Desktop {
    fn get(app: &AppHandle) -> Option<tauri::State<'_, Desktop>> {
        app.try_state::<Desktop>()
    }
}

// --- Fenster ---

pub fn show_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

// Minimieren und Schließen legen die App ins Tray – sie bleibt erreichbar. Beenden über das Tray-Menü.
pub fn hide_to_tray(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    let Some(d) = Desktop::get(app) else { return };
    if !d.tray_hint_shown.swap(true, Ordering::SeqCst) {
        let toast = d.toast(
            "mrphone läuft weiter",
            "Du bleibst erreichbar. Beenden über das Tray-Symbol.",
            "open",
            false,
        );
        if let Some(t) = toast {
            show_toast(app, &t, |app, _| show_window(app));
        }
    }
}

// Ohne Fokus anzeigen (wie showInactive in Electron) – wer gerade tippt, tippt weiter.
fn show_inactive(win: &WebviewWindow) {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE},
    };
    if let Ok(hwnd) = win.hwnd() {
        unsafe {
            let _ = ShowWindow(HWND(hwnd.0), SW_SHOWNOACTIVATE);
        }
    }
}

// Bei eingehendem Anruf Fenster hervorholen (auch aus dem Tray) und in der Taskleiste blinken lassen.
fn attention(app: &AppHandle, d: &Desktop, call: &Value) {
    let ringing = call["state"] == "incoming";
    if ringing == d.flashing.swap(ringing, Ordering::SeqCst) {
        return;
    }
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if ringing {
        let show_on_call = app
            .try_state::<AppState>()
            .map(|s| s.cfg.lock().unwrap()["showOnCall"].as_bool() != Some(false))
            .unwrap_or(true);
        if d.quiet {
            crate::logger::info("Selbsttest: Fenster würde hervorgeholt und blinken");
        } else {
            // Option: sonst reicht die Windows-Meldung mit Annehmen/Ablehnen, das Fenster bleibt, wo es ist
            if show_on_call {
                show_inactive(&win);
            }
            let _ = win.request_user_attention(Some(UserAttentionType::Critical));
        }
        show_call_toast(app, d, call, show_on_call);
    } else {
        if !d.quiet {
            let _ = win.request_user_attention(None);
        }
        close_call_toast(d);
    }
}

// --- Windows-Meldungen ---

// Anruf-Meldung: im Vordergrund bis zum Ende des Klingelns, stumm (Klingelton spielt die App selbst auf
// dem gewählten Klingelgerät), mit Annehmen/Ablehnen.
const CALL_TOAST: &str = r#" scenario="incomingCall""#;
const CALL_ACTIONS: &str = r#"<audio silent="true"/><actions><action content="Annehmen" arguments="answer" activationType="foreground"/><action content="Ablehnen" arguments="reject" activationType="foreground"/></actions>"#;

impl Desktop {
    // launch: Argument beim Klick auf die Meldung selbst.
    fn toast(
        &self,
        title: &str,
        body: &str,
        launch: &str,
        call: bool,
    ) -> Option<ToastNotification> {
        if self.quiet {
            crate::logger::info(&format!(
                "Selbsttest: Meldung „{title}“: {}",
                body.replace('\n', " / ")
            ));
            return None;
        }
        let (attrs, tail) = if call {
            (CALL_TOAST, CALL_ACTIONS)
        } else {
            ("", "")
        };
        let xml = format!(
            r#"<toast launch="{launch}"{attrs}><visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual>{tail}</toast>"#,
            xml_escape(title),
            xml_escape(body)
        );
        let build = || -> windows::core::Result<ToastNotification> {
            let doc = XmlDocument::new()?;
            doc.LoadXml(&HSTRING::from(xml))?;
            ToastNotification::CreateToastNotification(&doc)
        };
        build()
            .inspect_err(|e| crate::logger::warn(&format!("Windows-Meldung: {e}")))
            .ok()
    }
}

// Klick auf die Meldung oder einen Knopf -> on_click(app, Argument).
fn show_toast(app: &AppHandle, toast: &ToastNotification, on_click: fn(&AppHandle, &str)) {
    let handle = app.clone();
    let handler = TypedEventHandler::<ToastNotification, IInspectable>::new(move |_, insp| {
        let insp: &Option<IInspectable> = &insp;
        let arg = insp
            .as_ref()
            .and_then(|i| i.cast::<ToastActivatedEventArgs>().ok())
            .and_then(|a| a.Arguments().ok())
            .map(|h| h.to_string())
            .unwrap_or_default();
        on_click(&handle, &arg);
        Ok(())
    });
    let shown = (|| -> windows::core::Result<()> {
        toast.Activated(&handler)?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))?.Show(toast)
    })();
    if let Err(e) = shown {
        crate::logger::warn(&format!("Windows-Meldung: {e}"));
    }
}

// Bei mehreren Konten steht in Meldungen, welches Konto gemeint ist.
fn account_hint(app: &AppHandle, label: &str) -> String {
    let many = app.try_state::<AppState>().is_some_and(|s| {
        s.cfg.lock().unwrap()["accounts"]
            .as_array()
            .map_or(0, Vec::len)
            > 1
    });
    if many && !label.is_empty() {
        format!("\nfür {label}")
    } else {
        String::new()
    }
}

// Mit Annehmen/Ablehnen; bleibt stehen, bis der Anruf angenommen oder beendet ist.
fn show_call_toast(app: &AppHandle, d: &Desktop, call: &Value, show_on_call: bool) {
    let number = number_of(&text(&call["remoteUri"]));
    let name = [text(&call["contactName"]), text(&call["remoteName"])]
        .into_iter()
        .find(|n| !n.is_empty());
    let body = match name {
        Some(n) => format!("{n} ({number})"),
        None => number,
    } + &account_hint(app, &text(&call["accountLabel"]));
    let Some(toast) = d.toast("Eingehender Anruf", &body, "open", true) else {
        return;
    };
    let on_click: fn(&AppHandle, &str) = if show_on_call {
        |app, arg| {
            let Some(state) = app.try_state::<AppState>() else {
                return;
            };
            match arg {
                "answer" => {
                    state.phone.send(PhoneCmd::Answer);
                    show_window(app);
                }
                "reject" => state.phone.send(PhoneCmd::Reject),
                _ => show_window(app),
            }
        }
    } else {
        // Fenster bleibt, wo es ist (Klick auf die Meldung selbst öffnet es)
        |app, arg| {
            let Some(state) = app.try_state::<AppState>() else {
                return;
            };
            match arg {
                "answer" => state.phone.send(PhoneCmd::Answer),
                "reject" => state.phone.send(PhoneCmd::Reject),
                _ => show_window(app),
            }
        }
    };
    show_toast(app, &toast, on_click);
    *d.call_toast.lock().unwrap() = Some(toast);
}

fn close_call_toast(d: &Desktop) {
    if let Some(toast) = d.call_toast.lock().unwrap().take() {
        if let Ok(notifier) =
            ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))
        {
            let _ = notifier.Hide(&toast);
        }
    }
}

pub fn notify_missed(app: &AppHandle, entry: &Value) {
    let Some(d) = Desktop::get(app) else { return };
    let focused = app
        .get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false));
    if focused {
        return;
    }
    let uri = text(&entry["remoteUri"]);
    let known = app
        .try_state::<AppState>()
        .and_then(|s| s.contacts.lock().unwrap().lookup(&uri));
    let body = known
        .or_else(|| Some(text(&entry["remoteName"])).filter(|n| !n.is_empty()))
        .unwrap_or_else(|| number_of(&uri))
        + &account_hint(app, &text(&entry["accountLabel"]));
    let Some(toast) = d.toast("Verpasster Anruf", &body, "history", false) else {
        return;
    };
    show_toast(app, &toast, |app, _| {
        show_window(app);
        let _ = app.emit("phone:showHistory", ());
    });
    *d.missed_toast.lock().unwrap() = Some(toast);
}

// --- Tray ---

fn connection_summary(accounts: &[Value]) -> String {
    if accounts.is_empty() {
        return "Kein Konto".into();
    }
    let registered = accounts
        .iter()
        .filter(|a| a["state"] == "registered")
        .count();
    if registered == accounts.len() {
        return "Verbunden".into();
    }
    if registered > 0 {
        return format!("{registered} von {} verbunden", accounts.len());
    }
    if accounts.len() == 1 {
        let s = text(&accounts[0]["state"]);
        return REG_TEXT
            .iter()
            .find(|(k, _)| *k == s)
            .map_or(s.clone(), |(_, t)| t.to_string());
    }
    "Nicht verbunden".into()
}

pub fn update_tray(app: &AppHandle) {
    let (Some(d), Some(state)) = (Desktop::get(app), app.try_state::<AppState>()) else {
        return;
    };
    let s = state.phone.snapshot.lock().unwrap().clone();
    let dnd = crate::commands::own_dnd(&state);
    let tip = format!(
        "mrphone – {}{}",
        if s["call"].is_null() {
            connection_summary(s["accounts"].as_array().map_or(&[], Vec::as_slice))
        } else {
            "Im Gespräch".into()
        },
        if dnd { " · Nicht stören" } else { "" }
    );
    let tray = d.tray.lock().unwrap();
    if let Some(tray) = tray.as_ref() {
        let _ = tray.set_tooltip(Some(tip));
    }
}

// Jeder neue Stand der Telefonie: Tray-Text, bei Anruf Fenster/Meldung.
pub fn on_state(app: &AppHandle, snapshot: &Value) {
    let Some(d) = Desktop::get(app) else { return };
    attention(app, &d, &snapshot["call"]);
    // Gespräch zu Ende, PC inzwischen gesperrt: jetzt abmelden
    if d.in_call
        .swap(!snapshot["call"].is_null(), Ordering::SeqCst)
        && snapshot["call"].is_null()
    {
        apply_screen_lock(app);
    }
    let app = app.clone();
    // Tray-Text erst nach dem Speichern des Stands (on_state läuft vorher im SIP-Thread)
    tauri::async_runtime::spawn(async move { update_tray(&app) });
}

// --- Bildschirmsperre ---

// Gesperrter PC = nicht am Platz: abmelden, damit ein vergessenes mrphone (z. B. im Büro) nicht die
// Anmeldung eines anderen Geräts (Homeoffice) zurückholt. Ein laufendes Gespräch geht vor.
fn apply_screen_lock(app: &AppHandle) {
    let (Some(d), Some(state)) = (Desktop::get(app), app.try_state::<AppState>()) else {
        return;
    };
    let wanted = state.cfg.lock().unwrap()["lockUnregister"].as_bool() != Some(false);
    let in_call = !state.phone.snapshot.lock().unwrap()["call"].is_null();
    if d.screen_locked.load(Ordering::SeqCst) && wanted && !in_call {
        state.phone.send(PhoneCmd::Lock);
    }
}

fn screen_lock_changed(app: &AppHandle, locked: bool) {
    let Some(d) = Desktop::get(app) else { return };
    d.screen_locked.store(locked, Ordering::SeqCst);
    crate::logger::info(if locked {
        "PC gesperrt"
    } else {
        "PC entsperrt"
    });
    if locked {
        apply_screen_lock(app);
    } else if let Some(state) = app.try_state::<AppState>() {
        state.phone.send(PhoneCmd::Unlock);
    }
}

static APP: OnceLock<AppHandle> = OnceLock::new();
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
const WTS_SESSION_LOCK: usize = 7;
const WTS_SESSION_UNLOCK: usize = 8;

unsafe extern "system" fn subclass_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    _data: usize,
) -> windows::Win32::Foundation::LRESULT {
    if msg == WM_WTSSESSION_CHANGE && matches!(wparam.0, WTS_SESSION_LOCK | WTS_SESSION_UNLOCK) {
        if let Some(app) = APP.get() {
            screen_lock_changed(app, wparam.0 == WTS_SESSION_LOCK);
        }
    }
    unsafe { windows::Win32::UI::Shell::DefSubclassProc(hwnd, msg, wparam, lparam) }
}

fn watch_screen_lock(app: &AppHandle, win: &WebviewWindow) {
    use windows::Win32::{
        Foundation::HWND,
        System::RemoteDesktop::{WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION},
        UI::Shell::SetWindowSubclass,
    };
    let _ = APP.set(app.clone());
    let Ok(hwnd) = win.hwnd() else { return };
    let hwnd = HWND(hwnd.0);
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(subclass_proc), 1, 0);
        if let Err(e) = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) {
            crate::logger::warn(&format!("Bildschirmsperre wird nicht erkannt: {e}"));
        }
    }
}

// --- Einrichtung ---

// Meldungen brauchen eine registrierte App-Kennung (AUMID) mit Name und Symbol – ohne Startmenü-
// Verknüpfung (z. B. Testversion) über HKCU\Software\Classes\AppUserModelId.
fn register_aumid(dir: &Path) {
    use windows::Win32::{
        System::Registry::{RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ},
        UI::Shell::SetCurrentProcessExplicitAppUserModelID,
    };
    let icon = dir.join("mrphone.png");
    if std::fs::write(&icon, include_bytes!("../icons/128x128.png")).is_err() {
        return;
    }
    let key = HSTRING::from(format!("Software\\Classes\\AppUserModelId\\{AUMID}"));
    for (name, value) in [
        ("DisplayName", "mrphone".to_string()),
        ("IconUri", icon.display().to_string()),
    ] {
        let data: Vec<u16> = value.encode_utf16().chain([0]).collect();
        unsafe {
            let _ = RegSetKeyValueW(
                HKEY_CURRENT_USER,
                &key,
                &HSTRING::from(name),
                REG_SZ.0,
                Some(data.as_ptr().cast()),
                (data.len() * 2) as u32,
            );
        }
    }
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(AUMID));
    }
}

fn create_tray(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let open = MenuItem::with_id(app, "open", "Öffnen", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &PredefinedMenuItem::separator(app)?, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("mrphone")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)
}

pub fn setup(app: &AppHandle, win: &WebviewWindow, dir: &Path) -> tauri::Result<()> {
    let quiet = quiet();
    if !quiet {
        register_aumid(dir);
    }
    let tray = create_tray(app)?;
    app.manage(Desktop {
        quiet,
        tray: Mutex::new(Some(tray)),
        flashing: AtomicBool::new(false),
        in_call: AtomicBool::new(false),
        tray_hint_shown: AtomicBool::new(false),
        screen_locked: AtomicBool::new(false),
        call_toast: Mutex::new(None),
        missed_toast: Mutex::new(None),
    });
    let handle = app.clone();
    win.on_window_event(move |event| match event {
        // Selbsttests beenden die App von außen (taskkill ohne /F) – dort beendet Schließen wirklich
        // (mit Abmelden); der Schließen-Knopf der Titelleiste legt auch dort ins Tray.
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if quiet {
                handle.exit(0);
            } else {
                hide_to_tray(&handle);
            }
        }
        WindowEvent::Resized(_) => {
            let minimized = handle
                .get_webview_window("main")
                .is_some_and(|w| w.is_minimized().unwrap_or(false));
            if minimized {
                hide_to_tray(&handle);
            }
        }
        _ => {}
    });
    watch_screen_lock(app, win);
    update_tray(app);
    Ok(())
}

// Fensterknöpfe der eigenen Titelleiste (bridge.js zeichnet sie).
#[tauri::command]
pub fn window_control(app: AppHandle, action: String) -> bool {
    let Some(win) = app.get_webview_window("main") else {
        return false;
    };
    match action.as_str() {
        "minimize" | "close" => hide_to_tray(&app),
        "maximize" => {
            let _ = if win.is_maximized().unwrap_or(false) {
                win.unmaximize()
            } else {
                win.maximize()
            };
        }
        _ => {}
    }
    win.is_maximized().unwrap_or(false)
}

#[tauri::command]
pub fn window_maximized(app: AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|w| w.is_maximized().unwrap_or(false))
}
