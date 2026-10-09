// SIP-Benutzeragent eines Kontos: Anmeldung (REGISTER mit Digest), Übernahme-Erkennung, Besetztlampenfeld,
// Gespräche (ein- und ausgehend, Halten, Weiterleiten mit und ohne Rückfrage, Tastentöne).
// Entspricht der Klasse SipUA in src/sip.js der Electron-Version. Läuft auf dem SIP-Thread (eine
// Ereignisschleife wie Node.js) – daher Rc/RefCell und spawn_local; Ausleihen nie über ein await halten.
use super::msg::{self, Msg};
use super::rtp::{Rtp, RtpEvent};
use super::sdp::{self, Codec};
use md5::{Digest, Md5};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    rc::Rc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{net::UdpSocket, sync::oneshot, task::AbortHandle};

const T1: u64 = 500;
const T2: u64 = 4000;
const TX_TIMEOUT: u64 = 64 * T1;
const KEEPALIVE_SECS: u64 = 25;
const SUB_EXPIRES: u32 = 3600;
const USER_AGENT: &str = "mrphone/0.1";
const ALLOW: &str = "INVITE, ACK, CANCEL, BYE, OPTIONS, NOTIFY, UPDATE";

pub enum UaEvent {
    State,
    Presence { ext: String, state: &'static str },
    Info(String),
    // Sprache der Gegenstelle (PCM in der Abtastrate des Codecs) und Wechsel dieser Rate
    Audio(Vec<i16>),
    Format(u32),
    // Gespräch beendet (für Verlauf und Hinweis): direction, remoteUri, remoteName, createdAt, startedAt, rejected
    Ended { reason: String, call: Value },
}

const FAILURE_TEXT: [(u16, &str); 9] = [
    (404, "Nummer nicht gefunden"),
    (408, "Zeitüberschreitung"),
    (480, "Nicht erreichbar"),
    (486, "Besetzt"),
    (487, "Abgebrochen"),
    (488, "Kein gemeinsamer Codec"),
    (503, "Dienst nicht verfügbar"),
    (600, "Besetzt"),
    (603, "Abgelehnt"),
];

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn rand_hex(len: usize) -> String {
    let mut s = String::new();
    while s.len() < len {
        s.push_str(&uuid::Uuid::new_v4().simple().to_string());
    }
    s.truncate(len);
    s
}

fn md5_hex(s: &str) -> String {
    format!("{:x}", Md5::digest(s.as_bytes()))
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

// IP-Adresse, über die der Server erreichbar ist (wie localIpFor in sip.js).
fn local_ip_for(addr: SocketAddr, bind: IpAddr) -> std::io::Result<IpAddr> {
    let s = std::net::UdpSocket::bind((bind, 0))?;
    s.connect(addr)?;
    Ok(s.local_addr()?.ip())
}

#[derive(Clone)]
pub struct Request {
    method: String,
    uri: String,
    branch: String,
    headers: Vec<(String, String)>,
    body: String,
    content_type: Option<String>,
}

impl Request {
    fn header(&self, name: &str) -> String {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }
}

#[derive(Default)]
pub struct ReqOpts {
    call_id: String,
    from: String,
    to: String,
    cseq: u32,
    contact: bool,
    route: Vec<String>,
    extra: Vec<(String, String)>,
    body: String,
    content_type: Option<String>,
}

type ResponseFn = Rc<dyn Fn(Msg)>;

struct Tx {
    cb: ResponseFn,
    provisional: bool,
    task: AbortHandle,
    req: Rc<Request>,
}

// Ein Gespräch (Haupt- oder Rückfragegespräch) – Felder wie das call-Objekt in sip.js.
struct Call {
    direction: &'static str,
    state: &'static str, // calling | ringing | incoming | active
    remote_uri: String,
    remote_name: String,
    rtp: Rtp,
    cseq: u32,
    route_set: Vec<String>,
    created_at: u64,
    started_at: Option<u64>,
    sdp_id: u64,
    sdp_version: u32,
    call_id: String,
    local: String,
    remote: String,
    remote_target: String,
    local_tag: String,
    invite_in: Option<(Msg, SocketAddr)>,
    provisional: bool,
    cancel_pending: bool,
    ended: bool,
    codec: Option<Codec>,
    dtmf_pt: Option<u8>,
    local_direction: String,
    held: bool,
    early_media: bool,
    awaiting_offer: bool,
    ack_data: Option<Vec<u8>>,
    ok_timer: Option<AbortHandle>,
    rejected: bool,
    invite_out: Option<Rc<Request>>,
}

type CallRef = Rc<RefCell<Call>>;

#[derive(Default)]
struct RespOpts<'a> {
    to_tag: Option<String>,
    contact: bool,
    body: String,
    content_type: Option<&'a str>,
    extra: Vec<(&'a str, &'a str)>,
}

struct Sub {
    target: String,
    call_id: String,
    tag: String,
    cseq: u32,
    timer: Option<AbortHandle>,
}

struct Inner {
    cfg: Value,
    bind_ip: IpAddr,
    // Senden direkt über das Betriebssystem: Tokios try_send_to meldet unter Windows beim ersten Paket
    // „nicht bereit“, ohne zu senden – SIP-Pakete gingen dann verloren.
    sock: Option<Rc<std::net::UdpSocket>>,
    local_port: u16,
    proxy_addr: Option<SocketAddr>,
    local_ip: Option<IpAddr>,
    reg_state: String,
    reg_reason: String,
    reg_call_id: String,
    reg_tag: String,
    reg_cseq: u32,
    instance: String,
    binding_check: bool,
    standby: Option<String>,
    reg_epoch: u32,
    tx: HashMap<String, Tx>,
    stx: HashMap<String, Option<Vec<u8>>>,
    reg_timer: Option<AbortHandle>,
    tasks: Vec<AbortHandle>,
    watch: Vec<String>,
    subs: HashMap<String, Sub>,
    events: Rc<dyn Fn(UaEvent)>,
    call: Option<CallRef>,
    consult: Option<CallRef>, // zweites Gespräch bei Weiterleiten mit Rückfrage
    transferring: bool,       // während des Verbindens keine Rückkehr zum gehaltenen Gespräch
    hd_voice: bool,           // HD-Codecs anbieten/annehmen
    is_busy: Rc<dyn Fn() -> bool>, // telefoniert gerade ein anderes Konto?
}

#[derive(Clone)]
pub struct Ua(Rc<RefCell<Inner>>);

impl Ua {
    pub fn new(
        cfg: Value,
        bind_ip: IpAddr,
        events: Rc<dyn Fn(UaEvent)>,
        is_busy: Rc<dyn Fn() -> bool>,
    ) -> Self {
        Ua(Rc::new(RefCell::new(Inner {
            cfg,
            bind_ip,
            sock: None,
            local_port: 0,
            proxy_addr: None,
            local_ip: None,
            reg_state: "idle".into(),
            reg_reason: String::new(),
            reg_call_id: format!("{}@mrphone", rand_hex(24)),
            reg_tag: rand_hex(12),
            reg_cseq: 0,
            instance: rand_hex(12),
            binding_check: false,
            standby: None,
            reg_epoch: 0,
            tx: HashMap::new(),
            stx: HashMap::new(),
            reg_timer: None,
            tasks: Vec::new(),
            watch: Vec::new(),
            subs: HashMap::new(),
            events,
            call: None,
            consult: None,
            transferring: false,
            hd_voice: false,
            is_busy,
        })))
    }

    fn cfg(&self, key: &str) -> String {
        text(&self.0.borrow().cfg[key])
    }

    fn emit(&self, ev: UaEvent) {
        let events = self.0.borrow().events.clone();
        events(ev);
    }

    pub fn id(&self) -> String {
        self.cfg("id")
    }

    pub fn label(&self) -> String {
        self.cfg("label")
    }

    pub fn snapshot(&self) -> Value {
        let i = self.0.borrow();
        serde_json::json!({
            "id": i.cfg["id"], "label": i.cfg["label"],
            "aor": format!("sip:{}@{}", text(&i.cfg["username"]), text(&i.cfg["domain"])),
            "server": format!("{}:{}", text(&i.cfg["proxy"]), text(&i.cfg["proxyPort"])),
            "state": i.reg_state, "reason": i.reg_reason,
        })
    }

    pub fn reg_state(&self) -> String {
        self.0.borrow().reg_state.clone()
    }

    fn aor(&self) -> String {
        format!("sip:{}@{}", self.cfg("username"), self.cfg("domain"))
    }

    fn expires(&self) -> u32 {
        self.0.borrow().cfg["expires"].as_u64().unwrap_or(600) as u32
    }

    // --- Start / Stopp ---

    // cautious: erst nachsehen, ob ein anderes Gerät angemeldet ist, und es dann nicht verdrängen
    // (Testversion neben der installierten App).
    pub async fn start(&self, cautious: bool) -> std::io::Result<()> {
        let (bind, port) = {
            let i = self.0.borrow();
            (i.bind_ip, i.cfg["sipPort"].as_u64().unwrap_or(0) as u16)
        };
        let std_sock = std::net::UdpSocket::bind((bind, port))?;
        std_sock.set_nonblocking(true)?;
        let send_sock = Rc::new(std_sock.try_clone()?);
        let sock = UdpSocket::from_std(std_sock)?;
        let local_port = sock.local_addr()?.port();
        crate::logger::info(&format!(
            "SIP lauscht auf UDP-Port {local_port} ({})",
            self.cfg("label")
        ));
        {
            let mut i = self.0.borrow_mut();
            i.sock = Some(send_sock);
            i.local_port = local_port;
        }
        let me = self.clone();
        let recv = tokio::task::spawn_local(async move {
            let mut buf = vec![0u8; 65535];
            loop {
                let Ok((n, from)) = sock.recv_from(&mut buf).await else {
                    continue;
                };
                me.on_message(&buf[..n], from);
            }
        });
        let me = self.clone();
        let keepalive = tokio::task::spawn_local(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(KEEPALIVE_SECS)).await;
                // CRLF-Keepalive hält NAT/Firewall-Pinholes offen, damit eingehende Anrufe ankommen.
                if me.reg_state() == "registered" {
                    me.transmit(b"\r\n\r\n", None);
                }
            }
        });
        self.0
            .borrow_mut()
            .tasks
            .extend([recv.abort_handle(), keepalive.abort_handle()]);
        if cautious {
            let me = self.clone();
            tokio::task::spawn_local(async move { me.start_cautious().await });
        } else {
            let me = self.clone();
            let exp = self.expires();
            tokio::task::spawn_local(async move { me.register(exp, false).await });
        }
        Ok(())
    }

    async fn start_cautious(&self) {
        if self.resolve_network().await.is_ok() && self.bound_elsewhere().await {
            self.0.borrow_mut().standby = Some("elsewhere".into());
            self.set_reg("elsewhere", "");
            return;
        }
        self.register(self.expires(), false).await;
    }

    pub async fn stop(&self) {
        // Beide Gespräche beenden (auch eine Rückfrage): end_call macht die Rückfrage zum Hauptgespräch.
        while self.0.borrow().call.is_some() {
            self.hangup();
        }
        self.stop_watch();
        if let Some(t) = self.0.borrow_mut().reg_timer.take() {
            t.abort();
        }
        let _ = tokio::time::timeout(Duration::from_secs(2), self.unregister()).await;
        let mut i = self.0.borrow_mut();
        for (_, tx) in i.tx.drain() {
            tx.task.abort();
        }
        for t in i.tasks.drain(..) {
            t.abort();
        }
        i.sock = None;
    }

    // --- Transport ---

    fn transmit(&self, data: &[u8], to: Option<SocketAddr>) {
        let (sock, addr) = {
            let i = self.0.borrow();
            (i.sock.clone(), to.or(i.proxy_addr))
        };
        if let (Some(sock), Some(addr)) = (sock, addr) {
            if std::env::var("SIP_TRACE").as_deref() == Ok("1") {
                crate::logger::info(&format!(">>> an {addr}\n{}", String::from_utf8_lossy(data)));
            }
            if let Err(err) = sock.send_to(data, addr) {
                crate::logger::warn(&format!("SIP senden an {addr}: {err}"));
            }
        }
    }

    fn on_message(&self, buf: &[u8], from: SocketAddr) {
        // Nur der eigene Server spricht mit uns – Pakete anderer Adressen (SIP-Scanner) werden verworfen.
        if Some(from.ip()) != self.0.borrow().proxy_addr.map(|a| a.ip()) {
            return;
        }
        let Some(msg) = msg::parse(buf) else { return };
        if std::env::var("SIP_TRACE").as_deref() == Ok("1") {
            crate::logger::info(&format!("<<< von {from}\n{}", String::from_utf8_lossy(buf)));
        }
        if msg.method.is_some() {
            self.on_request(msg, from);
        } else {
            self.on_response(msg);
        }
    }

    fn own_from(&self) -> String {
        let name = self.cfg("displayName");
        if name.is_empty() {
            format!("<{}>", self.aor())
        } else {
            format!("\"{name}\" <{}>", self.aor())
        }
    }

    fn contact_header(&self) -> String {
        let i = self.0.borrow();
        format!(
            "<sip:{}@{}:{};transport=udp;line={}>",
            text(&i.cfg["username"]),
            i.local_ip.map(|ip| ip.to_string()).unwrap_or_default(),
            i.local_port,
            i.instance
        )
    }

    fn build_request(&self, method: &str, uri: &str, o: ReqOpts) -> Request {
        let branch = format!("z9hG4bK{}", rand_hex(16));
        let (local_ip, local_port) = {
            let i = self.0.borrow();
            (
                i.local_ip.map(|ip| ip.to_string()).unwrap_or_default(),
                i.local_port,
            )
        };
        let mut headers = vec![
            (
                "Via".to_string(),
                format!("SIP/2.0/UDP {local_ip}:{local_port};branch={branch};rport"),
            ),
            ("Max-Forwards".into(), "70".into()),
        ];
        headers.extend(o.route.into_iter().map(|r| ("Route".to_string(), r)));
        headers.extend([
            ("From".into(), o.from),
            ("To".into(), o.to),
            ("Call-ID".into(), o.call_id),
            ("CSeq".into(), format!("{} {method}", o.cseq)),
        ]);
        if o.contact {
            headers.push(("Contact".into(), self.contact_header()));
        }
        headers.push(("User-Agent".into(), USER_AGENT.into()));
        if method == "INVITE" || method == "REGISTER" {
            headers.push(("Allow".into(), ALLOW.into()));
        }
        headers.extend(o.extra);
        Request {
            method: method.into(),
            uri: uri.into(),
            branch,
            headers,
            body: o.body,
            content_type: o.content_type,
        }
    }

    // Client-Transaktion mit UDP-Wiederholungen (RFC 3261 Timer A/E). cb bekommt vorläufige und die
    // endgültige Antwort, nach 32 s ohne Antwort eine 408. INVITE: nach einer vorläufigen Antwort keine
    // Wiederholungen und kein Zeitlimit mehr (die Gegenstelle klingelt).
    fn send_request_cb(&self, req: Request, cb: ResponseFn) {
        let data = msg::serialize(
            &format!("{} {} SIP/2.0", req.method, req.uri),
            &req.headers,
            &req.body,
            req.content_type.as_deref(),
        );
        let key = format!("{}:{}", req.branch, req.method);
        let invite = req.method == "INVITE";
        let me = self.clone();
        let k = key.clone();
        let task = tokio::task::spawn_local(async move {
            let start = Instant::now();
            let mut interval = T1;
            loop {
                let provisional = match me.0.borrow().tx.get(&k) {
                    Some(t) => t.provisional,
                    None => return,
                };
                if provisional && invite {
                    return;
                }
                if !provisional {
                    me.transmit(&data, None);
                }
                let left = TX_TIMEOUT.saturating_sub(start.elapsed().as_millis() as u64);
                if left == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(interval.min(left))).await;
                interval = if invite {
                    interval * 2
                } else {
                    (interval * 2).min(T2)
                };
            }
            let cb = me.0.borrow_mut().tx.remove(&k).map(|t| t.cb);
            if let Some(cb) = cb {
                cb(Msg::synthetic(408, "Request Timeout"));
            }
        });
        self.0.borrow_mut().tx.insert(
            key,
            Tx {
                cb,
                provisional: false,
                task: task.abort_handle(),
                req: Rc::new(req),
            },
        );
    }

    // Nur die endgültige Antwort (REGISTER, SUBSCRIBE).
    fn send_request(&self, req: &Request) -> oneshot::Receiver<Msg> {
        let (tx, rx) = oneshot::channel();
        let tx = RefCell::new(Some(tx));
        self.send_request_cb(
            req.clone(),
            Rc::new(move |res: Msg| {
                if res.status >= 200 {
                    if let Some(tx) = tx.borrow_mut().take() {
                        let _ = tx.send(res);
                    }
                }
            }),
        );
        rx
    }

    fn on_response(&self, res: Msg) {
        let branch = msg::via_branch(res.header("via"));
        let method = res
            .header("cseq")
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .to_string();
        let key = format!("{branch}:{method}");
        let found = {
            let mut i = self.0.borrow_mut();
            match i.tx.get_mut(&key) {
                None => None,
                Some(tx) if res.status < 200 => {
                    tx.provisional = true;
                    Some((tx.cb.clone(), None))
                }
                Some(_) => {
                    let tx = i.tx.remove(&key).unwrap();
                    tx.task.abort();
                    Some((tx.cb, Some(tx.req)))
                }
            }
        };
        let Some((cb, finished)) = found else {
            // Wiederholtes 200 OK auf INVITE -> ACK des passenden Dialogs erneut senden.
            if method == "INVITE" && res.status < 300 {
                let id = res.header("call-id").to_string();
                let ack = self.calls().into_iter().find_map(|c| {
                    let c = c.borrow();
                    if c.call_id == id {
                        c.ack_data.clone()
                    } else {
                        None
                    }
                });
                if let Some(data) = ack {
                    self.transmit(&data, None);
                }
            }
            return;
        };
        if let Some(req) = finished {
            if method == "INVITE" && res.status >= 300 {
                self.ack_non2xx(&req, &res);
            }
        }
        cb(res);
    }

    fn ack_non2xx(&self, req: &Request, res: &Msg) {
        let mut headers = vec![
            ("Via".to_string(), req.header("Via")),
            ("Max-Forwards".into(), "70".into()),
        ];
        headers.extend(req.headers.iter().filter(|(n, _)| n == "Route").cloned());
        headers.extend([
            ("From".to_string(), req.header("From")),
            ("To".into(), res.header("to").to_string()),
            ("Call-ID".into(), req.header("Call-ID")),
            (
                "CSeq".into(),
                format!(
                    "{} ACK",
                    req.header("CSeq").split(' ').next().unwrap_or("1")
                ),
            ),
        ]);
        self.transmit(
            &msg::serialize(&format!("ACK {} SIP/2.0", req.uri), &headers, "", None),
            None,
        );
    }

    // Digest-Authentifizierung mit Klartext-Passwort oder dem HA1-Hash aus Linphone.
    fn authorize(&self, req: &Request, res: &Msg) -> Option<(String, String)> {
        let proxy = res.status == 407;
        let challenge = res.header(if proxy {
            "proxy-authenticate"
        } else {
            "www-authenticate"
        });
        if challenge.len() < 6 || !challenge[..6].eq_ignore_ascii_case("digest") {
            return None;
        }
        let p = parse_challenge(&challenge[6..]);
        if p.get("algorithm")
            .is_some_and(|a| !a.eq_ignore_ascii_case("md5"))
        {
            crate::logger::warn(&format!(
                "Nicht unterstützter Digest-Algorithmus: {}",
                p["algorithm"]
            ));
            return None;
        }
        let user = {
            let auth = self.cfg("authUsername");
            if auth.is_empty() {
                self.cfg("username")
            } else {
                auth
            }
        };
        let realm = p.get("realm").cloned().unwrap_or_default();
        let nonce = p.get("nonce").cloned().unwrap_or_default();
        let password = self.cfg("password");
        let ha1 = if password.is_empty() {
            self.cfg("ha1")
        } else {
            md5_hex(&format!("{user}:{realm}:{password}"))
        };
        if ha1.is_empty() {
            return None;
        }
        let ha2 = md5_hex(&format!("{}:{}", req.method, req.uri));
        let qop = p
            .get("qop")
            .filter(|q| q.split(',').any(|s| s.trim() == "auth"))
            .map(|_| "auth");
        let nc = "00000001";
        let cnonce = rand_hex(16);
        let response = match qop {
            Some(q) => md5_hex(&format!("{ha1}:{nonce}:{nc}:{cnonce}:{q}:{ha2}")),
            None => md5_hex(&format!("{ha1}:{nonce}:{ha2}")),
        };
        let mut value = format!("Digest username=\"{user}\", realm=\"{realm}\", nonce=\"{nonce}\", uri=\"{}\", response=\"{response}\", algorithm=MD5", req.uri);
        if let Some(opaque) = p.get("opaque") {
            value.push_str(&format!(", opaque=\"{opaque}\""));
        }
        if let Some(q) = qop {
            value.push_str(&format!(", qop={q}, nc={nc}, cnonce=\"{cnonce}\""));
        }
        Some((
            (if proxy {
                "Proxy-Authorization"
            } else {
                "Authorization"
            })
            .into(),
            value,
        ))
    }

    // --- Registrierung ---

    async fn resolve_network(&self) -> Result<(), String> {
        let (host, port, bind) = {
            let i = self.0.borrow();
            (
                text(&i.cfg["proxy"]),
                i.cfg["proxyPort"].as_u64().unwrap_or(5060) as u16,
                i.bind_ip,
            )
        };
        let addr = tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|e| e.to_string())?
            .find(|a| a.is_ipv4())
            .ok_or_else(|| "keine IPv4-Adresse".to_string())?;
        let local = local_ip_for(addr, bind).map_err(|e| e.to_string())?;
        let mut i = self.0.borrow_mut();
        i.proxy_addr = Some(addr);
        i.local_ip = Some(local);
        Ok(())
    }

    fn set_reg(&self, state: &str, reason: &str) {
        {
            let mut i = self.0.borrow_mut();
            i.reg_state = state.into();
            i.reg_reason = reason.into();
        }
        crate::logger::info(&format!(
            "Registrierung {}: {state}{}",
            self.cfg("label"),
            if reason.is_empty() {
                String::new()
            } else {
                format!(" ({reason})")
            }
        ));
        self.emit(UaEvent::State);
    }

    fn schedule_register(&self, seconds: u64, refresh: bool) {
        if let Some(t) = self.0.borrow_mut().reg_timer.take() {
            t.abort();
        }
        let me = self.clone();
        let task = tokio::task::spawn_local(async move {
            tokio::time::sleep(Duration::from_secs(seconds)).await;
            me.0.borrow_mut().reg_timer = None; // sonst bräche register() diese eigene Aufgabe ab
            let exp = me.expires();
            me.register(exp, refresh).await;
        });
        self.0.borrow_mut().reg_timer = Some(task.abort_handle());
    }

    // refresh: turnusmäßige Erneuerung – vorher fragen, ob inzwischen ein anderes Gerät angemeldet ist
    // (dessen Anmeldung wird nicht zurückgeholt). expires 0 = abmelden.
    pub fn register(
        &self,
        expires: u32,
        refresh: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>> {
        let me = self.clone();
        Box::pin(async move {
            if let Some(t) = me.0.borrow_mut().reg_timer.take() {
                t.abort();
            }
            if expires > 0 && me.0.borrow().standby.is_some() {
                return;
            }
            if me.cfg("proxy").is_empty() || me.cfg("username").is_empty() {
                me.set_reg("failed", "Kein Konto eingerichtet");
                return;
            }
            if let Err(err) = me.resolve_network().await {
                me.set_reg(
                    "failed",
                    &format!("Server {} nicht erreichbar ({err})", me.cfg("proxy")),
                );
                if expires > 0 {
                    me.schedule_register(30, false);
                }
                return;
            }
            let check = me.0.borrow().binding_check; // Ausleihe nicht über das await halten
            if refresh && check && me.bound_elsewhere().await {
                if me.0.borrow().standby.is_none() {
                    me.0.borrow_mut().standby = Some("elsewhere".into());
                    me.set_reg("elsewhere", "");
                }
                return;
            }
            if expires > 0 && me.0.borrow().standby.is_some() {
                return;
            }
            if expires > 0 {
                me.0.borrow_mut().reg_epoch += 1;
            }
            me.set_reg(
                if expires > 0 {
                    "registering"
                } else {
                    "unregistering"
                },
                "",
            );
            let res = me.register_request(Some(expires)).await;
            if res.status < 300 {
                let standby = me.0.borrow().standby.clone();
                if expires == 0 {
                    me.set_reg(standby.as_deref().unwrap_or("unregistered"), "");
                } else if let Some(reason) = standby {
                    // Während der Anmeldung in Ruhe versetzt -> gleich wieder abmelden
                    me.register(0, false).await;
                    me.set_reg(&reason, "");
                } else {
                    let own = me.is_own_binding(&res);
                    me.0.borrow_mut().binding_check = own;
                    me.set_reg("registered", "");
                    let granted = me.granted_expires(&res, expires);
                    me.schedule_register(((granted as f64 * 0.9) as u64).max(10), true);
                    let start_watch = {
                        let i = me.0.borrow();
                        !i.watch.is_empty() && i.subs.is_empty()
                    };
                    if start_watch {
                        me.start_watch();
                    }
                }
            } else {
                let reason = if res.status == 401 || res.status == 403 {
                    format!("{} Zugangsdaten abgelehnt", res.status)
                } else {
                    format!("{} {}", res.status, res.reason)
                };
                me.set_reg("failed", &reason);
                if expires > 0 {
                    me.schedule_register(60, false);
                }
            }
        })
    }

    // REGISTER mit Digest. expires None: nur abfragen, wer angemeldet ist (ohne Contact/Expires).
    async fn register_request(&self, expires: Option<u32>) -> Msg {
        let mut auth: Option<(String, String)> = None;
        loop {
            let (call_id, tag, cseq) = {
                let mut i = self.0.borrow_mut();
                i.reg_cseq += 1;
                (i.reg_call_id.clone(), i.reg_tag.clone(), i.reg_cseq)
            };
            let mut extra = Vec::new();
            if let Some(e) = expires {
                extra.push(("Expires".to_string(), e.to_string()));
            }
            if let Some(a) = auth.clone() {
                extra.push(a);
            }
            let domain = self.cfg("domain");
            let req = self.build_request(
                "REGISTER",
                &format!("sip:{domain}"),
                ReqOpts {
                    call_id,
                    from: format!("{};tag={tag}", self.own_from()),
                    to: format!("<{}>", self.aor()),
                    cseq,
                    contact: expires.is_some(),
                    extra,
                    ..Default::default()
                },
            );
            let res = self
                .send_request(&req)
                .await
                .unwrap_or_else(|_| Msg::synthetic(408, "Request Timeout"));
            if (res.status == 401 || res.status == 407) && auth.is_none() {
                if let Some(a) = self.authorize(&req, &res) {
                    auth = Some(a);
                    continue;
                }
            }
            return res;
        }
    }

    fn is_own_binding(&self, res: &Msg) -> bool {
        let own = format!(";line={}", self.0.borrow().instance);
        res.all("contact").iter().any(|c| {
            c.find(&own).is_some_and(|i| {
                let rest = &c[i + own.len()..];
                !rest
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
            })
        })
    }

    // true nur, wenn der Server eine Anmeldung meldet und sie nicht von diesem Gerät stammt.
    async fn bound_elsewhere(&self) -> bool {
        let res = self.register_request(None).await;
        res.status < 300 && !res.all("contact").is_empty() && !self.is_own_binding(&res)
    }

    // Meldet nur die eigene Anmeldung ab: chan_sip löscht bei "Expires: 0" die Anmeldung des Kontos, egal
    // von welchem Gerät – ein vergessenes Büro-Telefon würde sonst das im Homeoffice abmelden.
    async fn unregister(&self) {
        if self.reg_state() != "registered" {
            return;
        }
        let epoch = self.0.borrow().reg_epoch;
        let check = self.0.borrow().binding_check;
        if check && self.bound_elsewhere().await {
            return;
        }
        if epoch != self.0.borrow().reg_epoch {
            return; // Abfrage kam zu spät, inzwischen neu angemeldet
        }
        self.register(0, false).await;
    }

    // Anmeldung ruhen lassen, bis resume(): "locked" = PC gesperrt ("elsewhere" setzt register() selbst).
    pub async fn standby(&self, reason: &str) {
        self.0.borrow_mut().standby = Some(reason.into());
        self.stop_watch();
        let timer = self.0.borrow_mut().reg_timer.take();
        if let Some(t) = timer {
            t.abort();
        }
        let _ = tokio::time::timeout(Duration::from_secs(2), self.unregister()).await;
        let still = self.0.borrow().standby.as_deref() == Some(reason);
        if still {
            self.set_reg(reason, "");
        }
    }

    pub fn standby_reason(&self) -> Option<String> {
        self.0.borrow().standby.clone()
    }

    pub async fn resume(&self) {
        self.0.borrow_mut().standby = None;
        let exp = self.expires();
        self.register(exp, false).await;
    }

    // Geändertes Konto: alte Anmeldung abmelden, Änderungen übernehmen, neu anmelden.
    pub async fn reconfigure(&self, account: Value) {
        if let Some(t) = self.0.borrow_mut().reg_timer.take() {
            t.abort();
        }
        let _ = tokio::time::timeout(Duration::from_secs(2), self.unregister()).await;
        {
            let mut i = self.0.borrow_mut();
            i.cfg = account;
            i.reg_call_id = format!("{}@mrphone", rand_hex(24));
            i.reg_tag = rand_hex(12);
            i.reg_cseq = 0;
            i.proxy_addr = None;
            i.binding_check = false;
            i.standby = None;
        }
        let exp = self.expires();
        self.register(exp, false).await;
    }

    fn granted_expires(&self, res: &Msg, requested: u32) -> u32 {
        let own = {
            let i = self.0.borrow();
            format!(
                "{}:{}",
                i.local_ip.map(|ip| ip.to_string()).unwrap_or_default(),
                i.local_port
            )
        };
        let from_contact = res
            .all("contact")
            .iter()
            .find(|c| c.contains(&own))
            .and_then(|c| {
                msg::parse_addr(c)
                    .params
                    .get("expires")
                    .and_then(|e| e.parse().ok())
            });
        from_contact
            .or_else(|| res.header("expires").trim().parse().ok())
            .unwrap_or(requested)
    }

    // --- Besetztlampenfeld (BLF): Status der Kurzwahl-Nebenstellen per SUBSCRIBE/NOTIFY ---

    pub fn set_watch(&self, exts: Vec<String>) {
        let mut next: Vec<String> = Vec::new();
        for e in exts.into_iter().filter(|e| !e.is_empty()) {
            if !next.contains(&e) {
                next.push(e);
            }
        }
        let gone: Vec<String> = self
            .0
            .borrow()
            .subs
            .keys()
            .filter(|k| !next.contains(k))
            .cloned()
            .collect();
        self.0.borrow_mut().watch = next.clone();
        for ext in gone {
            self.unsubscribe(&ext);
        }
        if self.reg_state() == "registered" {
            for ext in next {
                if !self.0.borrow().subs.contains_key(&ext) {
                    self.subscribe(&ext, SUB_EXPIRES);
                }
            }
        }
    }

    fn start_watch(&self) {
        let watch = self.0.borrow().watch.clone();
        for ext in watch {
            self.subscribe(&ext, SUB_EXPIRES);
        }
    }

    fn stop_watch(&self) {
        let subs: Vec<(String, Sub)> = self.0.borrow_mut().subs.drain().collect();
        for (ext, sub) in subs {
            if let Some(t) = sub.timer {
                t.abort();
            }
            self.emit(UaEvent::Presence {
                ext,
                state: "unknown",
            });
        }
    }

    fn target_uri(&self, target: &str) -> String {
        let t = target.trim();
        let t = if t.len() >= 4 && t[..4].eq_ignore_ascii_case("tel:") {
            &t[4..]
        } else {
            t
        };
        let lower = t.to_lowercase();
        if lower.starts_with("sip:") || lower.starts_with("sips:") {
            return t.into();
        }
        if t.contains('@') {
            return format!("sip:{t}");
        }
        let clean: String = t
            .chars()
            .filter(|c| !matches!(c, ' ' | '(' | ')' | '/' | '-'))
            .collect();
        format!("sip:{clean}@{}", self.cfg("domain"))
    }

    fn subscribe(&self, ext: &str, expires: u32) {
        let target = self.target_uri(ext);
        if !self.0.borrow().subs.contains_key(ext) {
            let sub = Sub {
                target: target.clone(),
                call_id: format!("{}@mrphone", rand_hex(24)),
                tag: rand_hex(12),
                cseq: 0,
                timer: None,
            };
            self.0.borrow_mut().subs.insert(ext.into(), sub);
            self.emit(UaEvent::Presence {
                ext: ext.into(),
                state: "unknown",
            });
        }
        let me = self.clone();
        let ext_owned = ext.to_string();
        tokio::task::spawn_local(async move {
            let mut auth: Option<(String, String)> = None;
            loop {
                let Some((call_id, tag, cseq)) = ({
                    let mut i = me.0.borrow_mut();
                    i.subs.get_mut(&ext_owned).map(|s| {
                        s.cseq += 1;
                        (s.call_id.clone(), s.tag.clone(), s.cseq)
                    })
                }) else {
                    return;
                };
                let mut extra = vec![
                    ("Event".to_string(), "dialog".to_string()),
                    ("Accept".into(), "application/dialog-info+xml".into()),
                    ("Expires".into(), expires.to_string()),
                ];
                if let Some(a) = auth.clone() {
                    extra.push(a);
                }
                let req = me.build_request(
                    "SUBSCRIBE",
                    &target,
                    ReqOpts {
                        call_id,
                        from: format!("{};tag={tag}", me.own_from()),
                        to: format!("<{target}>"),
                        cseq,
                        contact: true,
                        extra,
                        ..Default::default()
                    },
                );
                let res = me
                    .send_request(&req)
                    .await
                    .unwrap_or_else(|_| Msg::synthetic(408, "Request Timeout"));
                if (res.status == 401 || res.status == 407) && auth.is_none() {
                    if let Some(a) = me.authorize(&req, &res) {
                        auth = Some(a);
                        continue;
                    }
                }
                // Bei Ablehnung (Anlage ohne BLF/Hint für diese Nebenstelle) bleibt der Status grau.
                if res.status >= 300 && me.0.borrow().subs.contains_key(&ext_owned) {
                    me.emit(UaEvent::Presence {
                        ext: ext_owned.clone(),
                        state: "unknown",
                    });
                }
                return;
            }
        });
        // Rechtzeitig erneuern
        let me = self.clone();
        let ext_owned = ext.to_string();
        let secs = ((expires as f64 * 0.9) as u64).max(30);
        let timer = tokio::task::spawn_local(async move {
            tokio::time::sleep(Duration::from_secs(secs)).await;
            if me.0.borrow().subs.contains_key(&ext_owned) {
                me.subscribe(&ext_owned, SUB_EXPIRES);
            }
        });
        if let Some(sub) = self.0.borrow_mut().subs.get_mut(ext) {
            if let Some(old) = sub.timer.replace(timer.abort_handle()) {
                old.abort();
            }
        }
    }

    fn unsubscribe(&self, ext: &str) {
        let Some(mut sub) = self.0.borrow_mut().subs.remove(ext) else {
            return;
        };
        if let Some(t) = sub.timer.take() {
            t.abort();
        }
        self.emit(UaEvent::Presence {
            ext: ext.into(),
            state: "unknown",
        });
        if self.reg_state() != "registered" {
            return;
        }
        sub.cseq += 1;
        let req = self.build_request(
            "SUBSCRIBE",
            &sub.target,
            ReqOpts {
                call_id: sub.call_id,
                from: format!("{};tag={}", self.own_from(), sub.tag),
                to: format!("<{}>", sub.target),
                cseq: sub.cseq,
                contact: true,
                extra: vec![
                    ("Event".into(), "dialog".into()),
                    ("Expires".into(), "0".into()),
                ],
                ..Default::default()
            },
        );
        drop(self.send_request(&req)); // Antwort wird nicht gebraucht
    }

    fn on_notify(&self, req: &Msg, from: SocketAddr) {
        self.respond(req, from, 200, "OK", &[]);
        if !req.header("event").to_lowercase().starts_with("dialog") {
            return; // andere NOTIFYs (z. B. Mailbox) ignorieren
        }
        let id = req.header("call-id");
        let Some(ext) = self
            .0
            .borrow()
            .subs
            .iter()
            .find(|(_, s)| s.call_id == id)
            .map(|(e, _)| e.clone())
        else {
            return;
        };
        let ss = req.header("subscription-state").to_lowercase();
        if ss.starts_with("terminated") {
            self.emit(UaEvent::Presence {
                ext: ext.clone(),
                state: "unknown",
            });
            // Nicht endgültige Absage -> nach kurzer Pause neu abonnieren
            if !(ss.contains("reason=rejected") || ss.contains("reason=noresource")) {
                let me = self.clone();
                let e = ext.clone();
                let timer = tokio::task::spawn_local(async move {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    if me.0.borrow().subs.contains_key(&e) {
                        me.subscribe(&e, SUB_EXPIRES);
                    }
                });
                if let Some(sub) = self.0.borrow_mut().subs.get_mut(&ext) {
                    if let Some(old) = sub.timer.replace(timer.abort_handle()) {
                        old.abort();
                    }
                }
            }
            return;
        }
        self.emit(UaEvent::Presence {
            ext,
            state: msg::parse_dialog_info(&req.body),
        });
    }

    // --- Eingehende Requests ---

    fn respond(
        &self,
        req: &Msg,
        to_addr: SocketAddr,
        status: u16,
        reason: &str,
        extra: &[(&str, &str)],
    ) {
        self.respond_full(
            req,
            to_addr,
            status,
            reason,
            RespOpts {
                extra: extra.to_vec(),
                ..Default::default()
            },
        );
    }

    // Antwort auf eine eingehende Anfrage; wird für Wiederholungen derselben Anfrage gemerkt.
    fn respond_full(
        &self,
        req: &Msg,
        to_addr: SocketAddr,
        status: u16,
        reason: &str,
        o: RespOpts,
    ) -> Vec<u8> {
        let mut to = req.header("to").to_string();
        let tag = o
            .to_tag
            .clone()
            .or_else(|| (status > 100).then(|| rand_hex(8)));
        if let Some(tag) = tag {
            if !msg::parse_addr(&to).params.contains_key("tag") {
                to.push_str(&format!(";tag={tag}"));
            }
        }
        let mut headers: Vec<(String, String)> = req
            .all("via")
            .iter()
            .map(|v| ("Via".to_string(), v.clone()))
            .collect();
        if o.contact {
            headers.extend(
                req.all("record-route")
                    .iter()
                    .map(|r| ("Record-Route".to_string(), r.clone())),
            );
        }
        headers.push(("From".into(), req.header("from").into()));
        headers.push(("To".into(), to));
        headers.push(("Call-ID".into(), req.header("call-id").into()));
        headers.push(("CSeq".into(), req.header("cseq").into()));
        if o.contact {
            headers.push(("Contact".into(), self.contact_header()));
        }
        headers.push(("User-Agent".into(), USER_AGENT.into()));
        headers.extend(o.extra.iter().map(|(n, v)| (n.to_string(), v.to_string())));
        let data = msg::serialize(
            &format!("SIP/2.0 {status} {reason}"),
            &headers,
            &o.body,
            o.content_type,
        );
        self.transmit(&data, Some(to_addr));
        let key = format!(
            "{}:{}",
            msg::via_branch(req.header("via")),
            req.method.as_deref().unwrap_or("")
        );
        if let Some(slot) = self.0.borrow_mut().stx.get_mut(&key) {
            *slot = Some(data.clone());
        }
        data
    }

    fn on_request(&self, req: Msg, from: SocketAddr) {
        // Ohne diese Header lässt sich weder antworten noch zuordnen.
        if ["via", "from", "to", "call-id", "cseq"]
            .iter()
            .any(|h| req.header(h).is_empty())
        {
            return;
        }
        let method = req.method.clone().unwrap_or_default();
        if method != "ACK" {
            let key = format!("{}:{method}", msg::via_branch(req.header("via")));
            let known = self.0.borrow().stx.get(&key).cloned();
            if let Some(answer) = known {
                if let Some(data) = answer {
                    self.transmit(&data, Some(from)); // Wiederholung: letzte Antwort erneut senden
                }
                return;
            }
            self.0.borrow_mut().stx.insert(key.clone(), None);
            let me = self.clone();
            tokio::task::spawn_local(async move {
                tokio::time::sleep(Duration::from_millis(TX_TIMEOUT)).await;
                me.0.borrow_mut().stx.remove(&key);
            });
        }
        match method.as_str() {
            "INVITE" => {
                if msg::parse_addr(req.header("to")).params.contains_key("tag") {
                    self.on_reinvite(&req, from);
                } else {
                    self.on_invite(&req, from);
                }
            }
            "ACK" => self.on_ack(&req),
            "BYE" => self.on_bye(&req, from),
            "CANCEL" => self.on_cancel(&req, from),
            "UPDATE" => self.on_update(&req, from),
            "OPTIONS" => self.respond(
                &req,
                from,
                200,
                "OK",
                &[("Allow", ALLOW), ("Accept", "application/sdp")],
            ),
            "NOTIFY" => self.on_notify(&req, from),
            _ => self.respond(&req, from, 501, "Not Implemented", &[("Allow", ALLOW)]),
        }
    }
}

// --- Gespräche (wie der Anruf-Teil von src/sip.js) ---
// Ausleihen: nie eine Call- oder Ua-Ausleihe halten, während Rtp-Methoden laufen (die melden Ereignisse
// zurück, die wiederum ausleihen) – deshalb erst rtp klonen, dann aufrufen.

// Gegenstelle einer Nachricht: P-Asserted-Identity / Remote-Party-ID (so meldet Asterisk die verbundene
// Gegenstelle nach, z. B. bei Click-to-Dial), sonst der Ersatz-Header.
fn remote_identity(m: &Msg, fallback: Option<&str>) -> Option<(String, String)> {
    let value = [
        m.header("p-asserted-identity"),
        m.header("remote-party-id"),
        fallback.map(|h| m.header(h)).unwrap_or(""),
    ]
    .into_iter()
    .find(|v| !v.is_empty())?;
    let a = msg::parse_addr(value);
    (!a.uri.is_empty()).then_some((a.uri, a.display))
}

// encodeURIComponent
fn uri_encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

impl Ua {
    fn calls(&self) -> Vec<CallRef> {
        let i = self.0.borrow();
        i.call.iter().chain(i.consult.iter()).cloned().collect()
    }

    pub fn has_call(&self) -> bool {
        self.0.borrow().call.is_some()
    }

    pub fn set_hd_voice(&self, on: bool) {
        self.0.borrow_mut().hd_voice = on;
    }

    fn is_main(&self, call: &CallRef) -> bool {
        self.0
            .borrow()
            .call
            .as_ref()
            .is_some_and(|c| Rc::ptr_eq(c, call))
    }

    // Das Gespräch mit der aktiven Sprachverbindung: bei Rückfrage das zweite, sonst das Hauptgespräch.
    fn media_call(&self) -> Option<CallRef> {
        let i = self.0.borrow();
        match &i.consult {
            Some(c) if !c.borrow().ended => Some(c.clone()),
            _ => i.call.clone(),
        }
    }

    fn find_call(&self, req: &Msg) -> Option<CallRef> {
        let id = req.header("call-id");
        self.calls().into_iter().find(|c| c.borrow().call_id == id)
    }

    pub fn call_snapshot(&self) -> Option<Value> {
        let i = self.0.borrow();
        let c = i.call.as_ref()?.borrow();
        let consult = i.consult.as_ref().map(|b| {
            let b = b.borrow();
            serde_json::json!({ "state": b.state, "remoteUri": b.remote_uri, "remoteName": b.remote_name, "startedAt": b.started_at, "earlyMedia": b.early_media })
        });
        Some(serde_json::json!({
            "direction": c.direction, "state": c.state, "remoteUri": c.remote_uri, "remoteName": c.remote_name,
            "startedAt": c.started_at, "earlyMedia": c.early_media, "held": c.held,
            "codec": c.codec.map(|k| k.name()), "consult": consult,
        }))
    }

    fn new_call(
        &self,
        direction: &'static str,
        remote_uri: String,
        remote_name: String,
        consult: bool,
    ) -> CallRef {
        let ua = Rc::downgrade(&self.0);
        let call = Rc::new_cyclic(|weak: &std::rc::Weak<RefCell<Call>>| {
            let weak = weak.clone();
            // Nur das Gespräch mit der aktiven Sprachverbindung wird gehört (das andere ist gehalten).
            let events: Rc<dyn Fn(RtpEvent)> = Rc::new(move |ev| {
                let (Some(inner), Some(call)) = (ua.upgrade(), weak.upgrade()) else {
                    return;
                };
                let ua = Ua(inner);
                if !ua.media_call().is_some_and(|m| Rc::ptr_eq(&m, &call)) {
                    return;
                }
                ua.emit(match ev {
                    RtpEvent::Audio(pcm) => UaEvent::Audio(pcm),
                    RtpEvent::Format(rate) => UaEvent::Format(rate),
                });
            });
            RefCell::new(Call {
                direction,
                state: if direction == "out" {
                    "calling"
                } else {
                    "incoming"
                },
                remote_uri,
                remote_name,
                rtp: Rtp::new(events),
                cseq: 0,
                route_set: Vec::new(),
                created_at: now_ms(),
                started_at: None,
                sdp_id: now_ms(),
                sdp_version: 0,
                call_id: String::new(),
                local: String::new(),
                remote: String::new(),
                remote_target: String::new(),
                local_tag: String::new(),
                invite_in: None,
                provisional: false,
                cancel_pending: false,
                ended: false,
                codec: None,
                dtmf_pt: None,
                local_direction: "sendrecv".into(),
                held: false,
                early_media: false,
                awaiting_offer: false,
                ack_data: None,
                ok_timer: None,
                rejected: false,
                invite_out: None,
            })
        });
        let mut i = self.0.borrow_mut();
        if consult {
            i.consult = Some(call.clone());
        } else {
            i.call = Some(call.clone());
        }
        call
    }

    fn local_sdp(&self, call: &CallRef) -> String {
        let (ip, hd) = {
            let i = self.0.borrow();
            (
                i.local_ip.map(|ip| ip.to_string()).unwrap_or_default(),
                i.hd_voice,
            )
        };
        let (port, id, version, codec, dtmf_pt, direction) = {
            let mut c = call.borrow_mut();
            c.sdp_version += 1;
            (
                c.rtp.port(),
                c.sdp_id,
                c.sdp_version,
                c.codec,
                c.dtmf_pt,
                c.local_direction.clone(),
            )
        };
        sdp::build(&sdp::Local {
            ip: &ip,
            port,
            session_id: id,
            version,
            codec,
            offer: sdp::offer_codecs(hd),
            dtmf_pt,
            direction: &direction,
        })
    }

    // Übernimmt eine nachgemeldete Gegenstelle; true, wenn sich etwas geändert hat.
    fn update_remote_party(&self, call: &CallRef, m: &Msg, fallback: Option<&str>) -> bool {
        let Some((uri, name)) = remote_identity(m, fallback) else {
            return false;
        };
        let mut c = call.borrow_mut();
        if uri == c.remote_uri && (name.is_empty() || name == c.remote_name) {
            return false;
        }
        c.remote_name = if !name.is_empty() {
            name
        } else if uri == c.remote_uri {
            c.remote_name.clone()
        } else {
            String::new()
        };
        c.remote_uri = uri;
        crate::logger::info(&format!(
            "Gegenstelle: {} <{}>",
            c.remote_name, c.remote_uri
        ));
        true
    }

    fn apply_remote_sdp(&self, call: &CallRef, body: &str) {
        let Some(remote) = sdp::parse(body) else {
            return;
        };
        let hd = self.0.borrow().hd_voice;
        let Some(codec) = sdp::choose_codec(&remote, hd) else {
            return;
        };
        let rtp = {
            let mut c = call.borrow_mut();
            c.codec = Some(codec);
            // Bei Opus (48-kHz-Takt) Tastentöne über SIP INFO statt RFC 4733 (telephone-event ist 8-kHz-getaktet).
            c.dtmf_pt = if codec.kind == sdp::Kind::Opus {
                None
            } else {
                remote.dtmf_pt
            };
            c.local_direction = match remote.direction.as_str() {
                "sendonly" => "recvonly",
                "recvonly" => "sendonly",
                "inactive" => "inactive",
                _ => "sendrecv",
            }
            .into();
            c.rtp.clone()
        };
        rtp.set_remote(&remote.ip, remote.port, codec);
    }

    fn open_rtp(&self, call: &CallRef) -> Result<(), String> {
        let local_ip = self.0.borrow().local_ip;
        let rtp = call.borrow().rtp.clone();
        rtp.open(local_ip)
            .map_err(|e| format!("Sprachkanal nicht verfügbar ({e})"))
    }

    pub fn dial(&self, target: &str) -> Result<(), String> {
        if self.0.borrow().call.is_some() {
            return Err("Es läuft bereits ein Gespräch".into());
        }
        let local_ip = self.0.borrow().local_ip;
        if self.0.borrow().proxy_addr.is_none() || local_ip.is_none() {
            return Err("Keine Verbindung zum SIP-Server".into());
        }
        if target.trim().is_empty() {
            return Err("Keine Nummer angegeben".into());
        }
        let uri = self.target_uri(target);
        let call = self.new_call("out", uri.clone(), String::new(), false);
        let from = format!("{};tag={}", self.own_from(), rand_hex(12));
        {
            let mut c = call.borrow_mut();
            c.call_id = format!("{}@{}", rand_hex(24), local_ip.unwrap());
            c.local = from;
            c.remote = format!("<{uri}>");
            c.remote_target = uri;
        }
        self.emit(UaEvent::State);
        if let Err(err) = self.open_rtp(&call) {
            self.end_call(&call, &err, false);
            return Err(err);
        }
        self.send_invite(&call, None);
        Ok(())
    }

    fn send_invite(&self, call: &CallRef, auth: Option<(String, String)>) {
        let body = self.local_sdp(call);
        let (target, call_id, local, remote, cseq) = {
            let mut c = call.borrow_mut();
            c.cseq += 1;
            (
                c.remote_target.clone(),
                c.call_id.clone(),
                c.local.clone(),
                c.remote.clone(),
                c.cseq,
            )
        };
        let authed = auth.is_some();
        let req = self.build_request(
            "INVITE",
            &target,
            ReqOpts {
                call_id,
                from: local,
                to: remote,
                cseq,
                contact: true,
                extra: auth.into_iter().collect(),
                body,
                content_type: Some("application/sdp".into()),
                ..Default::default()
            },
        );
        {
            let mut c = call.borrow_mut();
            c.invite_out = Some(Rc::new(req.clone()));
            c.provisional = false;
        }
        let (me, call2, req2) = (self.clone(), call.clone(), req.clone());
        self.send_request_cb(
            req,
            Rc::new(move |res| me.on_invite_response(&call2, &req2, res, authed)),
        );
    }

    fn on_invite_response(&self, call: &CallRef, req: &Request, res: Msg, authed: bool) {
        let ended = call.borrow().ended;
        if res.status < 200 {
            call.borrow_mut().provisional = true;
            if ended {
                let pending = call.borrow().cancel_pending;
                if pending {
                    self.send_cancel(call);
                }
                return;
            }
            if res.status == 100 {
                return;
            }
            self.update_remote_party(call, &res, None);
            call.borrow_mut().state = "ringing";
            if !res.body.is_empty() {
                self.apply_remote_sdp(call, &res.body);
                let rtp = call.borrow().rtp.clone();
                let early = rtp.has_remote();
                call.borrow_mut().early_media = early;
                if early {
                    rtp.start();
                }
            }
            self.emit(UaEvent::State);
            return;
        }
        if res.status < 300 {
            self.confirm_outgoing(call, &res);
            if ended {
                self.send_bye(call);
                return;
            }
            if !res.body.is_empty() {
                self.apply_remote_sdp(call, &res.body);
            }
            self.update_remote_party(call, &res, None);
            let rtp = call.borrow().rtp.clone();
            rtp.start();
            {
                let mut c = call.borrow_mut();
                c.state = "active";
                c.started_at = Some(now_ms());
            }
            self.emit(UaEvent::State);
            return;
        }
        if ended {
            return;
        }
        if (res.status == 401 || res.status == 407) && !authed {
            if let Some(a) = self.authorize(req, &res) {
                self.send_invite(call, Some(a));
                return;
            }
        }
        let reason = FAILURE_TEXT
            .iter()
            .find(|(s, _)| *s == res.status)
            .map(|(_, t)| t.to_string())
            .unwrap_or_else(|| format!("{} {}", res.status, res.reason));
        self.end_call(call, &reason, false);
    }

    fn confirm_outgoing(&self, call: &CallRef, res: &Msg) {
        let (target, call_id, local, remote, cseq, route) = {
            let mut c = call.borrow_mut();
            c.remote = res.header("to").to_string();
            let contact = res.header("contact");
            if !contact.is_empty() {
                c.remote_target = msg::parse_addr(contact).uri;
            }
            c.route_set = res.all("record-route").iter().rev().cloned().collect();
            (
                c.remote_target.clone(),
                c.call_id.clone(),
                c.local.clone(),
                c.remote.clone(),
                c.cseq,
                c.route_set.clone(),
            )
        };
        let ack = self.build_request(
            "ACK",
            &target,
            ReqOpts {
                call_id,
                from: local,
                to: remote,
                cseq,
                route,
                ..Default::default()
            },
        );
        let data = msg::serialize(&format!("ACK {} SIP/2.0", ack.uri), &ack.headers, "", None);
        call.borrow_mut().ack_data = Some(data.clone());
        self.transmit(&data, None);
    }

    fn send_cancel(&self, call: &CallRef) {
        let (inv, cseq) = {
            let mut c = call.borrow_mut();
            c.cancel_pending = false;
            (c.invite_out.clone(), c.cseq)
        };
        let Some(inv) = inv else { return };
        let mut headers = vec![
            ("Via".to_string(), inv.header("Via")),
            ("Max-Forwards".into(), "70".into()),
        ];
        headers.extend(inv.headers.iter().filter(|(n, _)| n == "Route").cloned());
        headers.extend([
            ("From".to_string(), inv.header("From")),
            ("To".into(), inv.header("To")),
            ("Call-ID".into(), inv.header("Call-ID")),
            ("CSeq".into(), format!("{cseq} CANCEL")),
            ("User-Agent".into(), USER_AGENT.into()),
        ]);
        let req = Request {
            method: "CANCEL".into(),
            uri: inv.uri.clone(),
            branch: inv.branch.clone(),
            headers,
            body: String::new(),
            content_type: None,
        };
        self.send_request_cb(req, Rc::new(|_| {}));
    }

    fn in_dialog(&self, call: &CallRef, method: &str, mut o: ReqOpts) -> Request {
        let target = {
            let mut c = call.borrow_mut();
            c.cseq += 1;
            o.call_id = c.call_id.clone();
            o.from = c.local.clone();
            o.to = c.remote.clone();
            o.cseq = c.cseq;
            o.route = c.route_set.clone();
            c.remote_target.clone()
        };
        self.build_request(method, &target, o)
    }

    fn send_bye(&self, call: &CallRef) {
        let req = self.in_dialog(call, "BYE", ReqOpts::default());
        self.send_request_cb(req, Rc::new(|_| {}));
    }

    pub fn hangup(&self) {
        let Some(call) = self.0.borrow().call.clone() else {
            return;
        };
        let (state, provisional) = {
            let c = call.borrow();
            (c.state, c.provisional)
        };
        if state == "incoming" {
            self.reject();
            return;
        }
        if state == "active" {
            self.send_bye(&call);
        } else if provisional {
            self.send_cancel(&call);
        } else {
            call.borrow_mut().cancel_pending = true;
        }
        self.end_call(&call, "Aufgelegt", false);
    }

    pub fn answer(&self) {
        let Some(call) = self.0.borrow().call.clone() else {
            return;
        };
        if call.borrow().state != "incoming" {
            return;
        }
        let body = self.local_sdp(&call);
        let (invite, addr, tag) = {
            let c = call.borrow();
            let Some((m, a)) = c.invite_in.clone() else {
                return;
            };
            (m, a, c.local_tag.clone())
        };
        let data = self.respond_full(
            &invite,
            addr,
            200,
            "OK",
            RespOpts {
                to_tag: Some(tag),
                contact: true,
                body,
                content_type: Some("application/sdp"),
                extra: vec![("Allow", ALLOW)],
            },
        );
        self.retransmit_until_ack(&call, data, addr);
        let rtp = call.borrow().rtp.clone();
        rtp.start();
        {
            let mut c = call.borrow_mut();
            c.state = "active";
            c.started_at = Some(now_ms());
        }
        self.emit(UaEvent::State);
    }

    pub fn reject(&self) {
        let Some(call) = self.0.borrow().call.clone() else {
            return;
        };
        let (invite, tag) = {
            let c = call.borrow();
            if c.state != "incoming" {
                return;
            }
            (c.invite_in.clone(), c.local_tag.clone())
        };
        if let Some((m, a)) = invite {
            self.respond_full(
                &m,
                a,
                486,
                "Busy Here",
                RespOpts {
                    to_tag: Some(tag),
                    ..Default::default()
                },
            );
        }
        call.borrow_mut().rejected = true;
        self.end_call(&call, "Abgelehnt", false);
    }

    // 200 OK wiederholen, bis das ACK kommt (UDP); ohne ACK nach 32 s auflegen.
    fn retransmit_until_ack(&self, call: &CallRef, data: Vec<u8>, addr: SocketAddr) {
        if let Some(t) = call.borrow_mut().ok_timer.take() {
            t.abort();
        }
        let (me, c) = (self.clone(), call.clone());
        let task = tokio::task::spawn_local(async move {
            let started = Instant::now();
            let mut interval = T1;
            loop {
                tokio::time::sleep(Duration::from_millis(interval)).await;
                if started.elapsed().as_millis() as u64 >= TX_TIMEOUT {
                    if me.is_main(&c) {
                        c.borrow_mut().ok_timer = None;
                        me.send_bye(&c);
                        me.end_call(&c, "Keine Bestätigung (ACK) vom Server", false);
                    }
                    return;
                }
                me.transmit(&data, Some(addr));
                interval = (interval * 2).min(T2);
            }
        });
        call.borrow_mut().ok_timer = Some(task.abort_handle());
    }

    // silent (Rückfragegespräch): kein Verlaufseintrag/Hinweis, nur Zustand aktualisieren.
    fn end_call(&self, call: &CallRef, reason: &str, silent: bool) {
        {
            let mut c = call.borrow_mut();
            if c.ended {
                return;
            }
            c.ended = true;
            if let Some(t) = c.ok_timer.take() {
                t.abort();
            }
        }
        let rtp = call.borrow().rtp.clone();
        rtp.close();
        let was_consult = {
            let mut i = self.0.borrow_mut();
            let was_consult = i.consult.as_ref().is_some_and(|c| Rc::ptr_eq(c, call));
            if was_consult {
                i.consult = None;
            } else if i.call.as_ref().is_some_and(|c| Rc::ptr_eq(c, call)) {
                // Endet das Hauptgespräch während einer Rückfrage, wird die Rückfrage zum Hauptgespräch.
                i.call = i.consult.take();
                if let Some(c) = &i.call {
                    c.borrow_mut().held = false;
                }
            }
            was_consult
        };
        crate::logger::info(&format!("Gespräch beendet: {reason}"));
        if !silent {
            let info = {
                let c = call.borrow();
                serde_json::json!({ "direction": c.direction, "remoteUri": c.remote_uri, "remoteName": c.remote_name, "createdAt": c.created_at, "startedAt": c.started_at, "rejected": c.rejected })
            };
            self.emit(UaEvent::Ended {
                reason: reason.into(),
                call: info,
            });
        }
        // Endet die Rückfrage (ohne dass gerade verbunden wird), zurück zum gehaltenen Hauptgespräch.
        let resume = {
            let i = self.0.borrow();
            was_consult && !i.transferring && i.call.as_ref().is_some_and(|c| c.borrow().held)
        };
        if resume {
            self.hold(false);
        }
        self.emit(UaEvent::State);
    }

    pub fn push_audio(&self, pcm: &[i16]) {
        if let Some(c) = self.media_call() {
            let rtp = c.borrow().rtp.clone();
            rtp.push_mic(pcm);
        }
    }

    // Tastentöne: RFC 4733 im RTP-Strom, wenn telephone-event ausgehandelt ist, sonst SIP INFO.
    pub fn send_dtmf(&self, digit: &str) {
        let Some(call) = self.media_call() else {
            return;
        };
        let mut chars = digit.chars();
        let (Some(d), None) = (chars.next(), chars.next()) else {
            return;
        };
        if call.borrow().state != "active"
            || !(d.is_ascii_digit() || matches!(d, '*' | '#' | 'A'..='D'))
        {
            return;
        }
        let (pt, rtp) = {
            let c = call.borrow();
            (c.dtmf_pt, c.rtp.clone())
        };
        if let Some(pt) = pt {
            rtp.send_dtmf(d, pt);
            return;
        }
        let req = self.in_dialog(
            &call,
            "INFO",
            ReqOpts {
                body: format!("Signal={d}\r\nDuration=100\r\n"),
                content_type: Some("application/dtmf-relay".into()),
                ..Default::default()
            },
        );
        self.send_request_cb(req, Rc::new(|_| {}));
    }

    // --- Halten und Weiterleiten ---

    // Halten (sendonly, die Anlage spielt Wartemusik) / Zurückholen (sendrecv) per Re-INVITE.
    pub fn hold(&self, on: bool) {
        let Some(call) = self.0.borrow().call.clone() else {
            return;
        };
        {
            let mut c = call.borrow_mut();
            if c.state != "active" || c.held == on {
                return;
            }
            c.held = on;
            c.local_direction = if on { "sendonly" } else { "sendrecv" }.into();
        }
        self.emit(UaEvent::State);
        self.send_reinvite(&call, on, None);
    }

    fn send_reinvite(&self, call: &CallRef, desired_held: bool, auth: Option<(String, String)>) {
        let body = self.local_sdp(call);
        let authed = auth.is_some();
        let req = self.in_dialog(
            call,
            "INVITE",
            ReqOpts {
                contact: true,
                extra: auth.into_iter().collect(),
                body,
                content_type: Some("application/sdp".into()),
                ..Default::default()
            },
        );
        let (me, c, req2) = (self.clone(), call.clone(), req.clone());
        self.send_request_cb(
            req,
            Rc::new(move |res| {
                if res.status < 200 || !me.is_main(&c) {
                    return;
                }
                if (res.status == 401 || res.status == 407) && !authed {
                    if let Some(a) = me.authorize(&req2, &res) {
                        me.send_reinvite(&c, desired_held, Some(a));
                        return;
                    }
                }
                if res.status < 300 {
                    me.ack_in_dialog(&c, &res);
                    if !res.body.is_empty() {
                        me.apply_remote_sdp(&c, &res.body);
                        c.borrow_mut().held = desired_held; // der Haltezustand bleibt unsere Vorgabe
                    }
                } else {
                    // Fehlgeschlagen -> Haltezustand zurücknehmen
                    let mut cc = c.borrow_mut();
                    cc.held = !desired_held;
                    cc.local_direction = if cc.held { "sendonly" } else { "sendrecv" }.into();
                }
                me.emit(UaEvent::State);
            }),
        );
    }

    fn ack_in_dialog(&self, call: &CallRef, res: &Msg) {
        let (target, call_id, local, remote, cseq, route) = {
            let c = call.borrow();
            let to = if res.header("to").is_empty() {
                c.remote.clone()
            } else {
                res.header("to").to_string()
            };
            (
                c.remote_target.clone(),
                c.call_id.clone(),
                c.local.clone(),
                to,
                c.cseq,
                c.route_set.clone(),
            )
        };
        let ack = self.build_request(
            "ACK",
            &target,
            ReqOpts {
                call_id,
                from: local,
                to: remote,
                cseq,
                route,
                ..Default::default()
            },
        );
        let data = msg::serialize(&format!("ACK {} SIP/2.0", ack.uri), &ack.headers, "", None);
        call.borrow_mut().ack_data = Some(data.clone());
        self.transmit(&data, None);
    }

    // Blind weiterleiten: REFER an die Anlage; nach 202 legen wir unsere Seite auf.
    pub fn transfer(&self, target: &str, auth: Option<(String, String)>) {
        let Some(call) = self.0.borrow().call.clone() else {
            return;
        };
        if call.borrow().state != "active" || target.trim().is_empty() {
            return;
        }
        let uri = self.target_uri(target);
        let mut extra = vec![
            ("Refer-To".to_string(), format!("<{uri}>")),
            ("Referred-By".into(), format!("<{}>", self.aor())),
        ];
        let authed = auth.is_some();
        extra.extend(auth);
        let req = self.in_dialog(
            &call,
            "REFER",
            ReqOpts {
                extra,
                ..Default::default()
            },
        );
        let (me, c, req2, target) = (self.clone(), call.clone(), req.clone(), target.to_string());
        self.send_request_cb(
            req,
            Rc::new(move |res| {
                if res.status < 200 || !me.is_main(&c) {
                    return;
                }
                if (res.status == 401 || res.status == 407) && !authed {
                    if let Some(a) = me.authorize(&req2, &res) {
                        me.transfer(&target, Some(a));
                        return;
                    }
                }
                if res.status < 300 {
                    me.send_bye(&c);
                    me.end_call(&c, "Weitergeleitet", false);
                } else {
                    me.emit(UaEvent::Info(format!(
                        "Weiterleiten abgelehnt ({})",
                        res.status
                    )));
                }
            }),
        );
    }

    // Weiterleiten mit Rückfrage: Hauptgespräch halten, zweites Gespräch (Rückfrage) zum Ziel aufbauen.
    pub fn attended_transfer(&self, target: &str) {
        let Some(a) = self.0.borrow().call.clone() else {
            return;
        };
        if a.borrow().state != "active"
            || self.0.borrow().consult.is_some()
            || target.trim().is_empty()
        {
            return;
        }
        if !a.borrow().held {
            self.hold(true);
        }
        let uri = self.target_uri(target);
        let local_ip = self
            .0
            .borrow()
            .local_ip
            .map(|ip| ip.to_string())
            .unwrap_or_default();
        let b = self.new_call("out", uri.clone(), String::new(), true);
        let from = format!("{};tag={}", self.own_from(), rand_hex(12));
        {
            let mut c = b.borrow_mut();
            c.call_id = format!("{}@{local_ip}", rand_hex(24));
            c.local = from;
            c.remote = format!("<{uri}>");
            c.remote_target = uri;
        }
        self.emit(UaEvent::State);
        if let Err(err) = self.open_rtp(&b) {
            self.end_call(&b, &err, true);
            return;
        }
        self.send_invite(&b, None);
    }

    // Verbinden: Gegenstelle des Hauptgesprächs mit der Rückfrage zusammenschalten (REFER mit Replaces).
    pub fn complete_transfer(&self) {
        let (a, b) = {
            let i = self.0.borrow();
            (i.call.clone(), i.consult.clone())
        };
        let (Some(a), Some(b)) = (a, b) else { return };
        let refer_to = {
            let bb = b.borrow();
            if bb.state != "active" {
                return;
            }
            let to_tag = msg::parse_addr(&bb.remote)
                .params
                .get("tag")
                .cloned()
                .unwrap_or_default();
            let from_tag = msg::parse_addr(&bb.local)
                .params
                .get("tag")
                .cloned()
                .unwrap_or_default();
            let replaces = format!("{};to-tag={to_tag};from-tag={from_tag}", bb.call_id);
            format!("<{}?Replaces={}>", bb.remote_target, uri_encode(&replaces))
        };
        let extra = vec![
            ("Refer-To".to_string(), refer_to),
            ("Referred-By".into(), format!("<{}>", self.aor())),
        ];
        let req = self.in_dialog(
            &a,
            "REFER",
            ReqOpts {
                extra,
                ..Default::default()
            },
        );
        self.0.borrow_mut().transferring = true;
        let me = self.clone();
        self.send_request_cb(
            req,
            Rc::new(move |res| {
                if res.status < 200 {
                    return;
                }
                if res.status < 300 {
                    me.send_bye(&b);
                    me.end_call(&b, "Verbunden", true);
                    me.send_bye(&a);
                    me.end_call(&a, "Verbunden", false);
                    me.0.borrow_mut().transferring = false;
                } else {
                    me.0.borrow_mut().transferring = false;
                    me.emit(UaEvent::Info(format!(
                        "Verbinden abgelehnt ({})",
                        res.status
                    )));
                }
            }),
        );
    }

    // Rückfrage abbrechen: zweites Gespräch beenden, zurück zum gehaltenen Hauptgespräch (in end_call).
    pub fn cancel_consult(&self) {
        let Some(b) = self.0.borrow().consult.clone() else {
            return;
        };
        let (state, provisional) = {
            let c = b.borrow();
            (c.state, c.provisional)
        };
        if state == "active" {
            self.send_bye(&b);
        } else if provisional {
            self.send_cancel(&b);
        } else {
            b.borrow_mut().cancel_pending = true;
        }
        self.end_call(&b, "Rückfrage beendet", true);
    }

    // --- Eingehende Anfragen zu Gesprächen ---

    fn on_invite(&self, req: &Msg, from: SocketAddr) {
        let busy = self.0.borrow().call.is_some();
        let other = self.0.borrow().is_busy.clone();
        if busy || other() {
            self.respond(req, from, 486, "Busy Here", &[]);
            return;
        }
        let Some((uri, name)) = remote_identity(req, Some("from")) else {
            self.respond(req, from, 400, "Bad Request", &[]);
            return;
        };
        self.respond(req, from, 100, "Trying", &[]);
        let call = self.new_call("in", uri.clone(), name.clone(), false);
        let tag = rand_hex(12);
        {
            let mut c = call.borrow_mut();
            c.call_id = req.header("call-id").into();
            c.local_tag = tag.clone();
            c.local = format!("{};tag={tag}", req.header("to"));
            c.remote = req.header("from").into();
            let contact = msg::parse_addr(req.header("contact")).uri;
            c.remote_target = if contact.is_empty() {
                uri.clone()
            } else {
                contact
            };
            c.route_set = req.all("record-route").to_vec();
            c.cseq = (now_ms() % 1000) as u32;
            c.invite_in = Some((req.clone(), from));
        }
        if let Err(err) = self.open_rtp(&call) {
            self.respond_full(
                req,
                from,
                503,
                "Service Unavailable",
                RespOpts {
                    to_tag: Some(tag),
                    ..Default::default()
                },
            );
            self.end_call(&call, &err, false);
            return;
        }
        if !req.body.is_empty() {
            self.apply_remote_sdp(&call, &req.body);
            if call.borrow().codec.is_none() {
                self.respond_full(
                    req,
                    from,
                    488,
                    "Not Acceptable Here",
                    RespOpts {
                        to_tag: Some(tag),
                        ..Default::default()
                    },
                );
                self.end_call(&call, "Kein gemeinsamer Codec", false);
                return;
            }
        } else {
            call.borrow_mut().awaiting_offer = true; // Angebot kommt erst mit dem ACK
        }
        self.respond_full(
            req,
            from,
            180,
            "Ringing",
            RespOpts {
                to_tag: Some(tag),
                contact: true,
                ..Default::default()
            },
        );
        crate::logger::info(&format!("Eingehender Anruf von {name} <{uri}>"));
        self.emit(UaEvent::State);
    }

    fn on_reinvite(&self, req: &Msg, from: SocketAddr) {
        let Some(call) = self.find_call(req) else {
            self.respond(req, from, 481, "Call/Transaction Does Not Exist", &[]);
            return;
        };
        if !req.body.is_empty() {
            self.apply_remote_sdp(&call, &req.body);
        } else {
            call.borrow_mut().awaiting_offer = true;
        }
        // Nur PAI/RPID: der From-Header bleibt im Dialog unverändert (bei Click-to-Dial = man selbst).
        if self.update_remote_party(&call, req, None) {
            self.emit(UaEvent::State);
        }
        let body = self.local_sdp(&call);
        let data = self.respond_full(
            req,
            from,
            200,
            "OK",
            RespOpts {
                contact: true,
                body,
                content_type: Some("application/sdp"),
                extra: vec![("Allow", ALLOW)],
                ..Default::default()
            },
        );
        self.retransmit_until_ack(&call, data, from);
    }

    fn on_ack(&self, req: &Msg) {
        let Some(call) = self.find_call(req) else {
            return;
        };
        let awaiting = {
            let mut c = call.borrow_mut();
            if let Some(t) = c.ok_timer.take() {
                t.abort();
            }
            std::mem::take(&mut c.awaiting_offer)
        };
        if awaiting && !req.body.is_empty() {
            self.apply_remote_sdp(&call, &req.body);
        }
    }

    fn on_bye(&self, req: &Msg, from: SocketAddr) {
        let Some(call) = self.find_call(req) else {
            self.respond(req, from, 481, "Call/Transaction Does Not Exist", &[]);
            return;
        };
        self.respond(req, from, 200, "OK", &[]);
        let incoming = call.borrow().state == "incoming";
        self.end_call(
            &call,
            if incoming {
                "Anruf verpasst"
            } else {
                "Gegenstelle hat aufgelegt"
            },
            false,
        );
    }

    fn on_cancel(&self, req: &Msg, from: SocketAddr) {
        let Some(call) = self.find_call(req) else {
            self.respond(req, from, 481, "Call/Transaction Does Not Exist", &[]);
            return;
        };
        self.respond(req, from, 200, "OK", &[]);
        let (incoming, invite, tag) = {
            let c = call.borrow();
            (
                c.state == "incoming",
                c.invite_in.clone(),
                c.local_tag.clone(),
            )
        };
        if incoming {
            if let Some((m, a)) = invite {
                self.respond_full(
                    &m,
                    a,
                    487,
                    "Request Terminated",
                    RespOpts {
                        to_tag: Some(tag),
                        ..Default::default()
                    },
                );
            }
            self.end_call(&call, "Anruf verpasst", false);
        }
    }

    fn on_update(&self, req: &Msg, from: SocketAddr) {
        let Some(call) = self.find_call(req) else {
            self.respond(req, from, 481, "Call/Transaction Does Not Exist", &[]);
            return;
        };
        if self.update_remote_party(&call, req, None) {
            self.emit(UaEvent::State);
        }
        if req.body.is_empty() {
            self.respond_full(
                req,
                from,
                200,
                "OK",
                RespOpts {
                    contact: true,
                    ..Default::default()
                },
            );
            return;
        }
        self.apply_remote_sdp(&call, &req.body);
        let body = self.local_sdp(&call);
        self.respond_full(
            req,
            from,
            200,
            "OK",
            RespOpts {
                contact: true,
                body,
                content_type: Some("application/sdp"),
                ..Default::default()
            },
        );
    }
}

fn parse_challenge(s: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut rest = s;
    loop {
        rest = rest.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
        let Some(eq) = rest.find('=') else { break };
        let key = rest[..eq].trim().to_lowercase();
        rest = rest[eq + 1..].trim_start();
        let value;
        if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"').unwrap_or(stripped.len());
            value = stripped[..end].to_string();
            rest = stripped.get(end + 1..).unwrap_or("");
        } else {
            let end = rest
                .find(|c: char| c == ',' || c.is_whitespace())
                .unwrap_or(rest.len());
            value = rest[..end].to_string();
            rest = &rest[end..];
        }
        if !key.is_empty() {
            out.insert(key, value);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_parsing() {
        let p = parse_challenge(
            r#" realm="asterisk", nonce="abc,def", qop="auth,auth-int", algorithm=MD5, opaque="o""#,
        );
        assert_eq!(p["realm"], "asterisk");
        assert_eq!(p["nonce"], "abc,def");
        assert_eq!(p["qop"], "auth,auth-int");
        assert_eq!(p["algorithm"], "MD5");
        assert_eq!(p["opaque"], "o");
    }

    #[test]
    fn md5_known_value() {
        // RFC 2617, Abschnitt 3.5: HA1 für Mufasa
        assert_eq!(
            md5_hex("Mufasa:testrealm@host.com:Circle Of Life"),
            "939e7578ed9e3c518a452acee763bce9"
        );
    }
}
