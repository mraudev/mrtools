// Anbindung an den CTI-Server der Telefonanlage (optional je Konto) – wie src/cti.js der Electron-Version:
// „Nicht stören“, „Abwesend“, Status der anderen Telefone und Teilnehmer von Konferenzen.
// Protokoll: TCP, je Nachricht "<Typname>-<JSON>" + "\0". Client -> Server: Actions, Server -> Client: Events.
use serde_json::{json, Value};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};

const CLIENT_VERSION: &str = "2.0.0.0"; // muss exakt der Protokollversion des Servers entsprechen
const MAX_FRAME: usize = 1024 * 1024; // Schutz: so lange Nachrichten schickt der Server nie
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const SILENCE: Duration = Duration::from_secs(65); // Server schickt etwa alle 20 s ein HeartbeatEvent
                                                   // Server erlaubt höchstens 6 neue Verbindungen je IP in 5 Minuten -> mit Abstand darunter bleiben.
const RATE_WINDOW: Duration = Duration::from_secs(5 * 60);
const RATE_MAX: usize = 5;
const BACKOFF_SECS: [u64; 6] = [5, 15, 30, 60, 120, 300];
const STABLE: Duration = Duration::from_secs(60); // so lange angemeldet = stabil, Wartezeit wieder von vorn

fn refused_text(reason: i64) -> &'static str {
    match reason {
        1 => "Anmeldung abgelehnt: kein Benutzer angegeben",
        2 => "Anmeldung abgelehnt: zum Benutzer ist kein Telefon hinterlegt",
        3 => "Anmeldung abgelehnt: Benutzer nicht gefunden",
        _ => "Anmeldung abgelehnt (unbekannt/Protokollversion)",
    }
}

fn net_error(err: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    match err.kind() {
        ConnectionRefused => "CTI-Server lehnt die Verbindung ab".into(),
        TimedOut => "CTI-Server nicht erreichbar".into(),
        HostUnreachable => "CTI-Server nicht erreichbar".into(),
        NetworkUnreachable => "CTI-Server nicht erreichbar (kein Netz/VPN?)".into(),
        ConnectionReset | ConnectionAborted => "Verbindung vom CTI-Server getrennt".into(),
        _ if err.to_string().contains("No such host") || err.raw_os_error() == Some(11001) => {
            "CTI-Server-Name nicht gefunden".into()
        }
        _ => err.to_string(),
    }
}

fn frame(type_name: &str, data: &str) -> Vec<u8> {
    format!("{type_name}-{data}\0").into_bytes()
}

#[derive(Clone)]
pub struct Phone {
    pub id: String,
    pub number: String,
    pub name: String,
    pub state: &'static str,
    pub dnd: bool,
    pub away: bool,
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn phone_info(p: &Value) -> Phone {
    let name = [&p["Vorname"], &p["Nachname"]]
        .iter()
        .map(|s| text(s).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Phone {
        id: text(&p["UniqueId"]),
        number: text(&p["Num"]),
        name,
        state: match p["State"].as_i64() {
            Some(1) => "idle",
            Some(2) => "busy",
            Some(3) => "ringing",
            Some(4) => "offline",
            _ => "unknown",
        },
        dnd: p["Dnd"].as_bool().unwrap_or(false),
        away: p["Abwesend"].as_bool().unwrap_or(false),
    }
}

#[derive(Clone)]
pub struct Channel {
    pub id: String,
    pub number: String,
    pub invited: bool,
}

#[derive(Clone, Default)]
pub struct Conference {
    pub id: String,
    pub owner_device: String,
    pub channels: Vec<Channel>, // Reihenfolge wie gemeldet
}

#[derive(Default)]
pub struct State {
    pub status: &'static str, // connecting | connected | offline | refused
    pub reason: String,
    pub own_id: Option<String>,
    pub phones: Vec<Phone>,
    pub conferences: Vec<Conference>,
}

impl State {
    pub fn own(&self) -> Option<&Phone> {
        let id = self.own_id.as_ref()?;
        self.phones.iter().find(|p| &p.id == id)
    }

    fn set_phone(&mut self, p: Phone) {
        match self.phones.iter_mut().find(|x| x.id == p.id) {
            Some(x) => *x = p,
            None => self.phones.push(p),
        }
    }

    // Legt die Konferenz bei Bedarf an (auch wenn ein Add vor dem NewConferenceEvent käme).
    fn update_conference(&mut self, data: &Value, remove: bool) {
        let id = text(&data["UniqueId"]);
        if id.is_empty() {
            return;
        }
        if !self.conferences.iter().any(|c| c.id == id) {
            self.conferences.push(Conference {
                id: id.clone(),
                ..Default::default()
            });
        }
        let conf = self.conferences.iter_mut().find(|c| c.id == id).unwrap();
        if let Some(owner) = data["OwnerDevice"].as_str().filter(|s| !s.is_empty()) {
            conf.owner_device = owner.into();
        }
        for c in data["Channels"].as_array().cloned().unwrap_or_default() {
            let ch = Channel {
                id: text(&c["UniqueId"]),
                number: text(&c["Number"]),
                invited: c["Type"].as_i64() == Some(1),
            };
            if ch.id.is_empty() {
                continue;
            }
            let pos = conf.channels.iter().position(|x| x.id == ch.id);
            match (remove, pos) {
                (true, Some(p)) => {
                    conf.channels.remove(p);
                }
                (true, None) => {}
                // Neuer Teilnehmer oder geänderter Status; fehlt die Nummer, bleibt die bekannte.
                (false, Some(p)) => {
                    let number = if ch.number.is_empty() {
                        conf.channels[p].number.clone()
                    } else {
                        ch.number
                    };
                    conf.channels[p] = Channel { number, ..ch };
                }
                (false, None) => conf.channels.push(ch),
            }
        }
    }

    fn clear(&mut self) {
        self.own_id = None;
        self.phones.clear();
        self.conferences.clear();
    }
}

enum Cmd {
    Action(Vec<u8>),
    Retry,
    Stop,
}

pub struct Client {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub state: Arc<Mutex<State>>,
    tx: mpsc::UnboundedSender<Cmd>,
}

impl Client {
    // on_change: nach jeder Änderung (Status, Telefone, Konferenzen) aufgerufen.
    pub fn start(
        host: String,
        port: u16,
        user: String,
        label: String,
        on_change: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let state = Arc::new(Mutex::new(State {
            status: "offline",
            ..Default::default()
        }));
        let worker = Worker {
            host: host.clone(),
            port,
            user: user.clone(),
            label,
            state: state.clone(),
            on_change,
            attempts: Vec::new(),
            backoff: 0,
        };
        tauri::async_runtime::spawn(worker.run(rx));
        Client {
            host,
            port,
            user,
            state,
            tx,
        }
    }

    pub fn connected(&self) -> bool {
        self.state.lock().unwrap().status == "connected"
    }

    pub fn set_dnd(&self, on: bool) {
        let _ = self.tx.send(Cmd::Action(frame(
            "SetDNDAction",
            &format!("{{\"Value\":{on}}}"),
        )));
    }

    pub fn set_away(&self, on: bool) {
        let _ = self.tx.send(Cmd::Action(frame(
            "SetAbwesendAction",
            &format!("{{\"Value\":{on}}}"),
        )));
    }

    // „Neu verbinden“: sofort versuchen, aber nur wenn nicht verbunden und die Begrenzung es zulässt.
    pub fn retry(&self) {
        let _ = self.tx.send(Cmd::Retry);
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Cmd::Stop);
    }
}

struct Worker {
    host: String,
    port: u16,
    user: String,
    label: String,
    state: Arc<Mutex<State>>,
    on_change: Arc<dyn Fn() + Send + Sync>,
    attempts: Vec<Instant>,
    backoff: usize,
}

enum End {
    Stop,
    Fail {
        reason: String,
        signed_in_at: Option<Instant>,
    },
}

impl Worker {
    fn set_status(&self, status: &'static str, reason: &str) {
        {
            let mut s = self.state.lock().unwrap();
            s.status = status;
            s.reason = reason.into();
        }
        (self.on_change)();
    }

    // Wartezeit, bis ein neuer Versuch die Begrenzung (RATE_MAX in RATE_WINDOW) einhält.
    fn wait_for_rate(&mut self) -> Duration {
        let now = Instant::now();
        self.attempts.retain(|t| now - *t < RATE_WINDOW);
        if self.attempts.len() < RATE_MAX {
            Duration::ZERO
        } else {
            (self.attempts[0] + RATE_WINDOW + Duration::from_secs(1)).saturating_duration_since(now)
        }
    }

    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Cmd>) {
        loop {
            self.attempts.push(Instant::now());
            self.set_status("connecting", "");
            let end = self.session(&mut rx).await;
            let was_refused = self.state.lock().unwrap().status == "refused";
            self.state.lock().unwrap().clear();
            let (reason, signed_in_at) = match end {
                End::Stop => return,
                End::Fail {
                    reason,
                    signed_in_at,
                } => (reason, signed_in_at),
            };
            if !was_refused {
                crate::logger::warn(&format!("CTI {}: {reason}", self.label));
            }
            if signed_in_at.is_some_and(|t| t.elapsed() >= STABLE) {
                self.backoff = 0;
            }
            let base = if was_refused {
                BACKOFF_SECS[BACKOFF_SECS.len() - 1]
            } else {
                BACKOFF_SECS[self.backoff.min(BACKOFF_SECS.len() - 1)]
            };
            self.backoff += 1;
            let delay = Duration::from_secs(base).max(self.wait_for_rate());
            if was_refused {
                let reason = self.state.lock().unwrap().reason.clone();
                self.set_status("refused", &reason);
            } else {
                self.set_status("offline", &reason);
            }
            // Warten – „Neu verbinden“ verkürzt, wenn die Begrenzung es zulässt.
            let until = tokio::time::Instant::now() + delay;
            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(until) => break,
                    cmd = rx.recv() => match cmd {
                        None | Some(Cmd::Stop) => return,
                        Some(Cmd::Retry) if self.wait_for_rate().is_zero() => break,
                        _ => {}
                    },
                }
            }
        }
    }

    async fn session(&mut self, rx: &mut mpsc::UnboundedReceiver<Cmd>) -> End {
        let fail = |reason: String| End::Fail {
            reason,
            signed_in_at: None,
        };
        let connect = tokio::time::timeout(
            CONNECT_TIMEOUT,
            TcpStream::connect((self.host.as_str(), self.port)),
        );
        let mut stream = tokio::select! {
            r = connect => match r {
                Err(_) => return fail("Keine Antwort vom CTI-Server".into()),
                Ok(Err(err)) => return fail(net_error(&err)),
                Ok(Ok(s)) => s,
            },
            cmd = rx.recv() => if matches!(cmd, None | Some(Cmd::Stop)) { return End::Stop } else { return fail("Verbindungsaufbau unterbrochen".into()) },
        };
        // Anmeldung muss spätestens 3 s nach dem Verbindungsaufbau kommen.
        let user = serde_json::to_string(&self.user).unwrap_or_default();
        let sign_in = frame(
            "SignInAction",
            &format!("{{\"User\":{user},\"ClientVersion\":\"{CLIENT_VERSION}\"}}"),
        );
        if let Err(err) = stream.write_all(&sign_in).await {
            return fail(net_error(&err));
        }
        let mut buf: Vec<u8> = Vec::new();
        let mut chunk = vec![0u8; 16384];
        let mut last_data = tokio::time::Instant::now();
        let mut signed_in_at: Option<Instant> = None;
        loop {
            tokio::select! {
                r = stream.read(&mut chunk) => {
                    let n = match r {
                        Ok(0) => {
                            let refused = self.state.lock().unwrap().status == "refused";
                            let reason = if refused { self.state.lock().unwrap().reason.clone() } else { "Verbindung vom CTI-Server getrennt".into() };
                            return End::Fail { reason, signed_in_at };
                        }
                        Ok(n) => n,
                        Err(err) => return End::Fail { reason: net_error(&err), signed_in_at },
                    };
                    last_data = tokio::time::Instant::now();
                    buf.extend_from_slice(&chunk[..n]);
                    // Nachrichten an den Null-Bytes trennen, dann erst als UTF-8 lesen (Umlaute an Paketgrenzen)
                    while let Some(end) = buf.iter().position(|&b| b == 0) {
                        let raw: Vec<u8> = buf.drain(..=end).collect();
                        let text = String::from_utf8_lossy(&raw[..raw.len() - 1]).trim().to_string();
                        if !text.is_empty() && self.on_frame(&text) {
                            signed_in_at = Some(Instant::now());
                        }
                    }
                    if buf.len() > MAX_FRAME {
                        return End::Fail { reason: "Ungültige Daten vom CTI-Server".into(), signed_in_at };
                    }
                }
                cmd = rx.recv() => match cmd {
                    None | Some(Cmd::Stop) => return End::Stop,
                    Some(Cmd::Action(data)) => {
                        if self.state.lock().unwrap().status == "connected" {
                            if let Err(err) = stream.write_all(&data).await {
                                return End::Fail { reason: net_error(&err), signed_in_at };
                            }
                        }
                    }
                    Some(Cmd::Retry) => {}
                },
                _ = tokio::time::sleep_until(last_data + SILENCE) => {
                    return End::Fail { reason: "Verbindung zum CTI-Server abgerissen".into(), signed_in_at };
                }
            }
        }
    }

    // true bei erfolgreicher Anmeldung.
    fn on_frame(&self, line: &str) -> bool {
        let Some(dash) = line.find('-').filter(|&d| d > 0) else {
            return false;
        };
        let type_name = &line[..dash];
        let Ok(data) = serde_json::from_str::<Value>(&line[dash + 1..]) else {
            return false;
        };
        if !data.is_object() {
            return false;
        }
        match type_name {
            "SignInSuccessEvent" => {
                crate::logger::info(&format!(
                    "CTI {}: angemeldet als {} ({}:{})",
                    self.label, self.user, self.host, self.port
                ));
                self.set_status("connected", "");
                return true;
            }
            "SignInRefusedEvent" => {
                let reason = refused_text(data["Reason"].as_i64().unwrap_or(0));
                crate::logger::warn(&format!("CTI {}: {reason}", self.label));
                self.set_status("refused", reason);
                return false; // der Server trennt danach selbst
            }
            "HeartbeatEvent" => return false,
            _ => {}
        }
        {
            let mut s = self.state.lock().unwrap();
            match type_name {
                "OwnPhoneEvent" => {
                    let p = phone_info(&data);
                    s.own_id = Some(p.id.clone());
                    s.set_phone(p);
                }
                "NewPhoneEvent" | "PhoneChangedEvent" => s.set_phone(phone_info(&data)),
                "PhoneRemovedEvent" => {
                    let id = text(&data["UniqueId"]);
                    s.phones.retain(|p| p.id != id);
                }
                "NewConferenceEvent" => {
                    // vollständige Teilnehmerliste -> neu aufbauen
                    let id = text(&data["UniqueId"]);
                    s.conferences.retain(|c| c.id != id);
                    s.update_conference(&data, false);
                }
                "ConferenceChannelAddEvent" | "ConferenceChannelChangedEvent" => {
                    s.update_conference(&data, false)
                }
                "ConferenceChannelRemovedEvent" => s.update_conference(&data, true),
                "ConferenceRemovedEvent" => {
                    let id = text(&data["UniqueId"]);
                    s.conferences.retain(|c| c.id != id);
                }
                _ => return false, // z. B. Kanäle aktiver Gespräche – braucht die App nicht
            }
        }
        (self.on_change)();
        false
    }
}

// Stand für die Oberfläche (wie ctiView in src/main.js): je Konto Verbindung und eigenes DND/Abwesend,
// Status der Kurzwahl-Nummern, erste Konferenz mit Teilnehmern.
pub fn view(
    clients: &[(String, Client)],
    favorites: &[String],
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Value {
    let states: Vec<(&String, std::sync::MutexGuard<State>)> = clients
        .iter()
        .map(|(id, c)| (id, c.state.lock().unwrap()))
        .collect();
    let mut phones = serde_json::Map::new();
    let mut names = std::collections::HashMap::new();
    for (_, s) in &states {
        for p in &s.phones {
            if !p.number.is_empty() && !p.name.is_empty() {
                names
                    .entry(p.number.clone())
                    .or_insert_with(|| p.name.clone());
            }
            if favorites.contains(&p.number) && !phones.contains_key(&p.number) {
                phones.insert(
                    p.number.clone(),
                    json!({ "state": p.state, "dnd": p.dnd, "away": p.away }),
                );
            }
        }
    }
    let mut conference = Value::Null;
    for (_, s) in &states {
        let Some(conf) = s.conferences.first() else {
            continue;
        };
        let own = s.own();
        let channels: Vec<Value> = conf
            .channels
            .iter()
            .map(|ch| {
                json!({
                    "id": ch.id,
                    "number": ch.number,
                    "name": lookup(&ch.number).or_else(|| names.get(&ch.number).cloned()).unwrap_or_default(),
                    "invited": ch.invited,
                    "self": own.is_some_and(|o| o.number == ch.number),
                })
            })
            .collect();
        conference = json!({ "id": conf.id, "owner": own.is_some_and(|o| o.id == conf.owner_device), "channels": channels });
        break;
    }
    let accounts: Vec<Value> = states
        .iter()
        .map(|(id, s)| {
            let own = s.own();
            json!({ "id": id, "status": s.status, "reason": s.reason, "dnd": own.map(|o| o.dnd), "away": own.map(|o| o.away) })
        })
        .collect();
    json!({ "accounts": accounts, "phones": phones, "conference": conference })
}
