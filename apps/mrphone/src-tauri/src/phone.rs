// Mehrere SIP-Konten gleichzeitig (wie src/phone.js der Electron-Version). Es gibt immer höchstens ein
// Gespräch – ein Anruf auf einem anderen Konto bekommt dann „besetzt“. Der SIP-Stack läuft auf einem
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
use tauri::{
    ipc::{Channel, InvokeResponseBody},
    AppHandle, Emitter,
};
use tokio::sync::{mpsc, oneshot};

pub enum PhoneCmd {
    Register,
    SetFavorites(Vec<String>),
    Add(Value),
    Update(Value),
    Remove(String),
    Dial {
        target: String,
        account_id: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Answer,
    Reject,
    Hangup,
    Dtmf(String),
    Hold(bool),
    Transfer(String),
    AttendedTransfer(String),
    CompleteTransfer,
    CancelConsult,
    Audio(Vec<i16>),
    SetHd(bool),
    Lock,   // PC gesperrt: Anmeldung ruhen lassen (nur die eigene abmelden)
    Unlock, // PC entsperrt: wieder anmelden
    Stop(oneshot::Sender<()>),
}

// Was der SIP-Thread von der App braucht: Namen aus dem Telefonbuch und Verlaufseinträge.
pub trait Hooks: Send + Sync {
    fn contact_name(&self, uri: &str) -> Option<String>;
    fn state_changed(&self, snapshot: &Value);
    fn call_ended(&self, reason: &str, call: Value);
}

#[derive(Clone)]
pub struct PhoneHandle {
    tx: mpsc::UnboundedSender<PhoneCmd>,
    pub snapshot: Arc<Mutex<Value>>,
    pub presence: Arc<Mutex<Map<String, Value>>>,
    // Sprache zur Oberfläche: binärer Kanal (Int16, Abtastrate des Codecs)
    pub audio_out: Arc<Mutex<Option<Channel<InvokeResponseBody>>>>,
}

impl PhoneHandle {
    pub fn send(&self, cmd: PhoneCmd) {
        let _ = self.tx.send(cmd);
    }

    pub async fn dial(&self, target: String, account_id: Option<String>) -> Result<(), String> {
        let (reply, wait) = oneshot::channel();
        self.send(PhoneCmd::Dial {
            target,
            account_id,
            reply,
        });
        wait.await
            .unwrap_or_else(|_| Err("Telefonie nicht bereit".into()))
    }

    // Vor dem Beenden: alle Gespräche beenden und abmelden (nur die eigene Anmeldung), höchstens ein paar Sekunden.
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

pub struct Setup {
    pub accounts: Vec<Value>,
    pub favorites: Vec<String>,
    pub cautious: bool, // beim Start nicht verdrängen, wenn das Konto schon an einem anderen Gerät angemeldet ist
    pub hd_voice: bool,
}

pub fn start(app: AppHandle, setup: Setup, hooks: Arc<dyn Hooks>) -> PhoneHandle {
    let (tx, rx) = mpsc::unbounded_channel();
    let handle = PhoneHandle {
        tx,
        snapshot: Arc::new(Mutex::new(json!({ "accounts": [], "call": null }))),
        presence: Arc::new(Mutex::new(Map::new())),
        audio_out: Arc::new(Mutex::new(None)),
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
            local.block_on(&rt, run(app, setup, hooks, rx, shared));
        })
        .expect("SIP-Thread");
    handle
}

type Lines = Rc<RefCell<Vec<Ua>>>;

fn active(lines: &Lines) -> Option<Ua> {
    lines.borrow().iter().find(|u| u.has_call()).cloned()
}

async fn run(
    app: AppHandle,
    setup: Setup,
    hooks: Arc<dyn Hooks>,
    mut rx: mpsc::UnboundedReceiver<PhoneCmd>,
    shared: PhoneHandle,
) {
    let lines: Lines = Rc::new(RefCell::new(Vec::new()));
    let hd_voice = Rc::new(RefCell::new(setup.hd_voice));
    let publish: Rc<dyn Fn()> = {
        let (lines, app, snapshot, hooks) = (
            lines.clone(),
            app.clone(),
            shared.snapshot.clone(),
            hooks.clone(),
        );
        Rc::new(move || {
            let accounts: Vec<Value> = lines.borrow().iter().map(Ua::snapshot).collect();
            let call = active(&lines).and_then(|ua| {
                let mut c = ua.call_snapshot()?;
                c["accountId"] = json!(ua.id());
                c["accountLabel"] = json!(ua.label());
                c["contactName"] = json!(hooks.contact_name(c["remoteUri"].as_str().unwrap_or("")));
                Some(c)
            });
            let s = json!({ "accounts": accounts, "call": call });
            *snapshot.lock().unwrap() = s.clone();
            hooks.state_changed(&s);
            let _ = app.emit("phone:state", s);
        })
    };
    // Ereignisse je Konto (für den Verlauf muss bekannt sein, über welches Konto das Gespräch lief).
    let events_for = |id: String, label: String| -> Rc<dyn Fn(UaEvent)> {
        let (publish, app, presence, audio_out, hooks) = (
            publish.clone(),
            app.clone(),
            shared.presence.clone(),
            shared.audio_out.clone(),
            hooks.clone(),
        );
        Rc::new(move |ev| match ev {
            UaEvent::State => publish(),
            UaEvent::Presence { ext, state } => {
                presence.lock().unwrap().insert(ext.clone(), json!(state));
                let _ = app.emit("phone:presence", json!({ "ext": ext, "state": state }));
            }
            UaEvent::Info(text) => {
                let _ = app.emit("phone:info", text);
            }
            UaEvent::Audio(pcm) => {
                let channel = audio_out.lock().unwrap().clone();
                if let Some(ch) = channel {
                    let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
                    let _ = ch.send(InvokeResponseBody::Raw(bytes));
                }
            }
            UaEvent::Format(rate) => {
                let _ = app.emit("phone:audioFormat", json!({ "rate": rate }));
            }
            UaEvent::Ended { reason, mut call } => {
                call["accountId"] = json!(id);
                call["accountLabel"] = json!(label);
                hooks.call_ended(&reason, call);
            }
        })
    };
    let is_busy: Rc<dyn Fn() -> bool> = {
        let weak = Rc::downgrade(&lines);
        Rc::new(move || {
            weak.upgrade()
                .is_some_and(|l| l.borrow().iter().any(Ua::has_call))
        })
    };
    let add = |account: Value, cautious: bool| {
        let id = account["id"].as_str().unwrap_or_default().to_string();
        let label = account["label"].as_str().unwrap_or_default().to_string();
        let ua = Ua::new(account, bind_ip(), events_for(id, label), is_busy.clone());
        ua.set_hd_voice(*hd_voice.borrow());
        lines.borrow_mut().push(ua.clone());
        tokio::task::spawn_local(async move {
            if let Err(err) = ua.start(cautious).await {
                crate::logger::warn(&format!("SIP-Konto {}: {err}", ua.id()));
            }
        });
    };
    for account in setup.accounts {
        add(account, setup.cautious);
    }
    publish();
    // Kurzwahl-Status (BLF) über das erste Konto – die Nebenstellen liegen auf derselben Anlage.
    let watch = |numbers: Vec<String>| {
        if let Some(first) = lines.borrow().first() {
            first.set_watch(numbers);
        }
    };
    watch(setup.favorites);

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
                let ua = lines.borrow().iter().find(|u| u.id() == id).cloned();
                if let Some(ua) = ua {
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
            // Ohne (gültige) Kontoangabe: das erste angemeldete Konto.
            PhoneCmd::Dial {
                target,
                account_id,
                reply,
            } => {
                let result = if active(&lines).is_some() {
                    Err("Es läuft bereits ein Gespräch".to_string())
                } else {
                    let line = {
                        let l = lines.borrow();
                        account_id
                            .and_then(|id| l.iter().find(|u| u.id() == id).cloned())
                            .or_else(|| l.iter().find(|u| u.reg_state() == "registered").cloned())
                            .or_else(|| l.first().cloned())
                    };
                    match line {
                        Some(ua) => ua.dial(&target),
                        None => Err("Kein Konto eingerichtet".into()),
                    }
                };
                let _ = reply.send(result);
            }
            PhoneCmd::Answer => active(&lines).iter().for_each(Ua::answer),
            PhoneCmd::Reject => active(&lines).iter().for_each(Ua::reject),
            PhoneCmd::Hangup => active(&lines).iter().for_each(Ua::hangup),
            PhoneCmd::Dtmf(digit) => active(&lines).iter().for_each(|u| u.send_dtmf(&digit)),
            PhoneCmd::Hold(on) => active(&lines).iter().for_each(|u| u.hold(on)),
            PhoneCmd::Transfer(target) => active(&lines)
                .iter()
                .for_each(|u| u.transfer(&target, None)),
            PhoneCmd::AttendedTransfer(target) => active(&lines)
                .iter()
                .for_each(|u| u.attended_transfer(&target)),
            PhoneCmd::CompleteTransfer => active(&lines).iter().for_each(Ua::complete_transfer),
            PhoneCmd::CancelConsult => active(&lines).iter().for_each(Ua::cancel_consult),
            PhoneCmd::Audio(pcm) => active(&lines).iter().for_each(|u| u.push_audio(&pcm)),
            PhoneCmd::SetHd(on) => {
                *hd_voice.borrow_mut() = on;
                lines.borrow().iter().for_each(|u| u.set_hd_voice(on));
            }
            PhoneCmd::Lock => {
                let all = lines.borrow().clone();
                for ua in all {
                    tokio::task::spawn_local(async move { ua.standby("locked").await });
                }
            }
            PhoneCmd::Unlock => {
                let all = lines.borrow().clone();
                for ua in all {
                    if ua.standby_reason().as_deref() == Some("locked") {
                        tokio::task::spawn_local(async move { ua.resume().await });
                    }
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
