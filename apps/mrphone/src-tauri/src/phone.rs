// Mehrere SIP-Konten gleichzeitig (wie src/phone.js der Electron-Version). Der SIP-Stack läuft auf einem
// eigenen Thread mit einer Ereignisschleife; die Tauri-Befehle schicken ihm Aufträge über einen Kanal und
// lesen den letzten Stand (snapshot) direkt.
use crate::sip::ua::{Ua, UaEvent};
use serde_json::{json, Map, Value};
use std::{
    cell::RefCell,
    net::IpAddr,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};

pub enum PhoneCmd {
    Register,
    SetFavorites(Vec<String>),
    Add(Value),
    Update(Value),
    Remove(String),
    Stop(oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct PhoneHandle {
    tx: mpsc::UnboundedSender<PhoneCmd>,
    pub snapshot: Arc<Mutex<Value>>,
    pub presence: Arc<Mutex<Map<String, Value>>>,
}

impl PhoneHandle {
    pub fn send(&self, cmd: PhoneCmd) {
        let _ = self.tx.send(cmd);
    }

    // Vor dem Beenden: alle Konten abmelden (nur die eigene Anmeldung), höchstens ein paar Sekunden.
    pub async fn stop(&self) {
        let (done, wait) = oneshot::channel();
        self.send(PhoneCmd::Stop(done));
        let _ = tokio::time::timeout(Duration::from_secs(4), wait).await;
    }
}

// Lauschadresse für SIP: alle Netze wie die Electron-Version; Selbsttests setzen MRPHONE_SIP_BIND=127.0.0.1
// (dann keine Firewall-Abfrage).
fn bind_ip() -> IpAddr {
    std::env::var("MRPHONE_SIP_BIND")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(IpAddr::from([0, 0, 0, 0]))
}

// cautious: beim Start nicht verdrängen, wenn das Konto schon an einem anderen Gerät angemeldet ist.
pub fn start(
    app: AppHandle,
    accounts: Vec<Value>,
    favorites: Vec<String>,
    cautious: bool,
) -> PhoneHandle {
    let (tx, rx) = mpsc::unbounded_channel();
    let handle = PhoneHandle {
        tx,
        snapshot: Arc::new(Mutex::new(json!({ "accounts": [], "call": null }))),
        presence: Arc::new(Mutex::new(Map::new())),
    };
    let shared = handle.clone();
    std::thread::Builder::new()
        .name("sip".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("SIP-Laufzeit");
            let local = tokio::task::LocalSet::new();
            local.block_on(&rt, run(app, accounts, favorites, cautious, rx, shared));
        })
        .expect("SIP-Thread");
    handle
}

async fn run(
    app: AppHandle,
    accounts: Vec<Value>,
    favorites: Vec<String>,
    cautious: bool,
    mut rx: mpsc::UnboundedReceiver<PhoneCmd>,
    shared: PhoneHandle,
) {
    let lines: Rc<RefCell<Vec<Ua>>> = Rc::new(RefCell::new(Vec::new()));
    let publish: Rc<dyn Fn()> = {
        let (lines, app, snapshot) = (lines.clone(), app.clone(), shared.snapshot.clone());
        Rc::new(move || {
            let accounts: Vec<Value> = lines.borrow().iter().map(Ua::snapshot).collect();
            let s = json!({ "accounts": accounts, "call": null });
            *snapshot.lock().unwrap() = s.clone();
            let _ = app.emit("phone:state", s);
        })
    };
    let events: Rc<dyn Fn(UaEvent)> = {
        let (publish, app, presence) = (publish.clone(), app.clone(), shared.presence.clone());
        Rc::new(move |ev| match ev {
            UaEvent::State => publish(),
            UaEvent::Presence { ext, state } => {
                presence.lock().unwrap().insert(ext.clone(), json!(state));
                let _ = app.emit("phone:presence", json!({ "ext": ext, "state": state }));
            }
            UaEvent::Info(text) => {
                let _ = app.emit("phone:info", text);
            }
        })
    };
    let add = |account: Value, cautious: bool| {
        let ua = Ua::new(account, bind_ip(), events.clone());
        lines.borrow_mut().push(ua.clone());
        let label = ua.id();
        tokio::task::spawn_local(async move {
            if let Err(err) = ua.start(cautious).await {
                crate::logger::warn(&format!("SIP-Konto {label}: {err}"));
            }
        });
    };
    for account in accounts {
        add(account, cautious);
    }
    publish();
    // Kurzwahl-Status (BLF) über das erste Konto – die Nebenstellen liegen auf derselben Anlage.
    let watch = |numbers: Vec<String>| {
        if let Some(first) = lines.borrow().first() {
            first.set_watch(numbers);
        }
    };
    watch(favorites);

    while let Some(cmd) = rx.recv().await {
        match cmd {
            PhoneCmd::Register => {
                let all = lines.borrow().clone();
                for ua in all {
                    tokio::task::spawn_local(async move { ua.resume().await });
                }
            }
            PhoneCmd::SetFavorites(numbers) => {
                shared
                    .presence
                    .lock()
                    .unwrap()
                    .retain(|ext, _| numbers.contains(ext));
                watch(numbers);
            }
            PhoneCmd::Add(account) => {
                add(account, false);
                publish();
            }
            PhoneCmd::Update(account) => {
                let id = account["id"].as_str().unwrap_or_default().to_string();
                if let Some(ua) = lines.borrow().iter().find(|u| u.id() == id).cloned() {
                    tokio::task::spawn_local(async move { ua.reconfigure(account).await });
                }
            }
            PhoneCmd::Remove(id) => {
                let removed = {
                    let mut l = lines.borrow_mut();
                    let pos = l.iter().position(|u| u.id() == id);
                    pos.map(|p| l.remove(p))
                };
                publish();
                if let Some(ua) = removed {
                    tokio::task::spawn_local(async move { ua.stop().await }); // meldet ab
                }
            }
            PhoneCmd::Stop(done) => {
                let all: Vec<Ua> = lines.borrow().clone();
                let tasks: Vec<_> = all
                    .into_iter()
                    .map(|ua| tokio::task::spawn_local(async move { ua.stop().await }))
                    .collect();
                for t in tasks {
                    let _ = t.await;
                }
                let _ = done.send(());
                return;
            }
        }
    }
}
