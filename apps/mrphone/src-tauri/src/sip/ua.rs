// SIP-Benutzeragent eines Kontos: Anmeldung (REGISTER mit Digest), Übernahme-Erkennung, Besetztlampenfeld.
// Entspricht der Klasse SipUA in src/sip.js der Electron-Version. Läuft auf dem SIP-Thread (eine
// Ereignisschleife wie Node.js) – daher Rc/RefCell und spawn_local; Ausleihen nie über ein await halten.
// Gespräche (INVITE annehmen, RTP) folgen in Stufe 3; bis dahin werden Anrufe mit „besetzt“ abgewiesen.
use super::msg::{self, Msg};
use md5::{Digest, Md5};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    rc::Rc,
    time::{Duration, Instant},
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

pub struct Request {
    method: String,
    uri: String,
    branch: String,
    headers: Vec<(String, String)>,
}

pub struct ReqOpts {
    call_id: String,
    from: String,
    to: String,
    cseq: u32,
    contact: bool,
    extra: Vec<(String, String)>,
}

struct Tx {
    waiter: Option<oneshot::Sender<Msg>>,
    provisional: bool,
    task: AbortHandle,
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
}

#[derive(Clone)]
pub struct Ua(Rc<RefCell<Inner>>);

impl Ua {
    pub fn new(cfg: Value, bind_ip: IpAddr, events: Rc<dyn Fn(UaEvent)>) -> Self {
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
            ("From".into(), o.from),
            ("To".into(), o.to),
            ("Call-ID".into(), o.call_id),
            ("CSeq".into(), format!("{} {method}", o.cseq)),
        ];
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
        }
    }

    // Client-Transaktion mit UDP-Wiederholungen (RFC 3261 Timer E/F); liefert die endgültige Antwort
    // oder nach 32 s eine 408.
    fn send_request(&self, req: &Request) -> oneshot::Receiver<Msg> {
        let data = msg::serialize(
            &format!("{} {} SIP/2.0", req.method, req.uri),
            &req.headers,
            "",
            None,
        );
        let key = format!("{}:{}", req.branch, req.method);
        let (tx, rx) = oneshot::channel();
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
                if !provisional {
                    me.transmit(&data, None);
                }
                let left = TX_TIMEOUT.saturating_sub(start.elapsed().as_millis() as u64);
                if left == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(interval.min(left))).await;
                interval = (interval * 2).min(T2);
            }
            let waiter = me.0.borrow_mut().tx.remove(&k).and_then(|t| t.waiter);
            if let Some(w) = waiter {
                let _ = w.send(Msg::synthetic(408, "Request Timeout"));
            }
        });
        self.0.borrow_mut().tx.insert(
            key,
            Tx {
                waiter: Some(tx),
                provisional: false,
                task: task.abort_handle(),
            },
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
        let mut i = self.0.borrow_mut();
        let Some(tx) = i.tx.get_mut(&key) else { return };
        if res.status < 200 {
            tx.provisional = true; // keine Wiederholungen mehr, aber weiter auf die Antwort warten
            return;
        }
        let tx = i.tx.remove(&key).unwrap();
        drop(i);
        tx.task.abort();
        if let Some(w) = tx.waiter {
            let _ = w.send(res);
        }
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
        let mut to = req.header("to").to_string();
        if status > 100 && !msg::parse_addr(&to).params.contains_key("tag") {
            to.push_str(&format!(";tag={}", rand_hex(8)));
        }
        let mut headers: Vec<(String, String)> = req
            .all("via")
            .iter()
            .map(|v| ("Via".to_string(), v.clone()))
            .collect();
        headers.push(("From".into(), req.header("from").into()));
        headers.push(("To".into(), to));
        headers.push(("Call-ID".into(), req.header("call-id").into()));
        headers.push(("CSeq".into(), req.header("cseq").into()));
        headers.push(("User-Agent".into(), USER_AGENT.into()));
        headers.extend(extra.iter().map(|(n, v)| (n.to_string(), v.to_string())));
        let data = msg::serialize(&format!("SIP/2.0 {status} {reason}"), &headers, "", None);
        self.transmit(&data, Some(to_addr));
        let key = format!(
            "{}:{}",
            msg::via_branch(req.header("via")),
            req.method.as_deref().unwrap_or("")
        );
        if let Some(slot) = self.0.borrow_mut().stx.get_mut(&key) {
            *slot = Some(data);
        }
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
            "INVITE" => self.on_invite(&req, from),
            "ACK" => {}
            "OPTIONS" => self.respond(
                &req,
                from,
                200,
                "OK",
                &[("Allow", ALLOW), ("Accept", "application/sdp")],
            ),
            "NOTIFY" => self.on_notify(&req, from),
            "BYE" | "CANCEL" | "UPDATE" => {
                self.respond(&req, from, 481, "Call/Transaction Does Not Exist", &[])
            }
            _ => self.respond(&req, from, 501, "Not Implemented", &[("Allow", ALLOW)]),
        }
    }

    // Stufe 2: noch keine Gespräche – Anrufe bekommen „besetzt“, die Oberfläche einen Hinweis.
    fn on_invite(&self, req: &Msg, from: SocketAddr) {
        if msg::parse_addr(req.header("to")).params.contains_key("tag") {
            self.respond(req, from, 481, "Call/Transaction Does Not Exist", &[]);
            return;
        }
        self.respond(req, from, 486, "Busy Here", &[]);
        let caller = {
            let pai = req.header("p-asserted-identity");
            let value = if pai.is_empty() {
                req.header("from")
            } else {
                pai
            };
            let a = msg::parse_addr(value);
            if a.display.is_empty() {
                a.uri
            } else {
                format!("{} <{}>", a.display, a.uri)
            }
        };
        crate::logger::info(&format!(
            "Anruf von {caller} abgewiesen (Gespräche folgen in Stufe 3)"
        ));
        self.emit(UaEvent::Info(format!(
            "Anruf von {caller} abgewiesen – Gespräche kommen in Stufe 3 der Tauri-Version."
        )));
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
