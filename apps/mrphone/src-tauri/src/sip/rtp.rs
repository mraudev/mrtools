// Sprachstrom (RTP/RTCP) eines Gesprächs – wie src/rtp.js der Electron-Version. Gesendet wird im Takt des
// Mikrofons; liefert es länger als 200 ms nichts, hält ein 20-ms-Takt den Strom mit Stille am Laufen.
// Läuft auf dem SIP-Thread (Rc/RefCell). Senden direkt über das Betriebssystem (std-Socket), Empfang über
// Tokio – Tokios try_send_to verliert unter Windows sonst das erste Paket.
use super::g722;
use super::sdp::{Codec, Kind};
use std::{
    cell::RefCell,
    collections::VecDeque,
    net::{IpAddr, SocketAddr, UdpSocket as StdSocket},
    rc::Rc,
    time::{Duration, Instant},
};
use tokio::{net::UdpSocket, task::AbortHandle};

const FRAME: usize = 160; // Payload-Bytes/Zeitmarke je 20 ms bei G.711
const MIC_STALE: Duration = Duration::from_millis(200);
// Tastentöne nach RFC 4733 (telephone-event)
const DTMF_DURATION: u16 = (FRAME * 5) as u16; // 100 ms
const DTMF_END_PACKETS: u32 = 3; // Endpaket dreifach (UDP kann verlieren)
const DTMF_GAP_FRAMES: u32 = 3; // 60 ms Pause zwischen zwei Tönen
const DTMF_VOLUME: u8 = 10; // -10 dBm0

pub enum RtpEvent {
    Audio(Vec<i16>),
    Format(u32),
}

// --- G.711 (nach der Referenzimplementierung von Sun) ---
const SEG_AEND: [i32; 8] = [0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff];
const SEG_UEND: [i32; 8] = [0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff];
const BIAS: i32 = 0x84;
const CLIP: i32 = 8159;

fn segment(value: i32, table: &[i32; 8]) -> i32 {
    table.iter().position(|&end| value <= end).unwrap_or(8) as i32
}

pub fn linear_to_alaw(pcm: i16) -> u8 {
    let mut value = (pcm as i32) >> 3;
    let mut mask = 0xd5;
    if value < 0 {
        mask = 0x55;
        value = -value - 1;
    }
    let seg = segment(value, &SEG_AEND);
    if seg >= 8 {
        return (0x7f ^ mask) as u8;
    }
    let mut aval = seg << 4;
    aval |= if seg < 2 {
        (value >> 1) & 0x0f
    } else {
        (value >> seg) & 0x0f
    };
    (aval ^ mask) as u8
}

pub fn alaw_to_linear(aval: u8) -> i16 {
    let aval = (aval ^ 0x55) as i32;
    let mut t = (aval & 0x0f) << 4;
    let seg = (aval & 0x70) >> 4;
    match seg {
        0 => t += 8,
        1 => t += 0x108,
        _ => t = (t + 0x108) << (seg - 1),
    }
    (if aval & 0x80 != 0 { t } else { -t }) as i16
}

pub fn linear_to_ulaw(pcm: i16) -> u8 {
    let mut value = (pcm as i32) >> 2;
    let mut mask = 0xff;
    if value < 0 {
        value = -value;
        mask = 0x7f;
    }
    value = value.min(CLIP) + (BIAS >> 2);
    let seg = segment(value, &SEG_UEND);
    if seg >= 8 {
        return (0x7f ^ mask) as u8;
    }
    (((seg << 4) | ((value >> (seg + 1)) & 0x0f)) ^ mask) as u8
}

pub fn ulaw_to_linear(uval: u8) -> i16 {
    let uval = (!uval) as i32;
    let mut t = ((uval & 0x0f) << 3) + BIAS;
    t <<= (uval & 0x70) >> 4;
    (if uval & 0x80 != 0 { BIAS - t } else { t - BIAS }) as i16
}

fn dtmf_event(digit: char) -> Option<u8> {
    match digit {
        '0'..='9' => Some(digit as u8 - b'0'),
        '*' => Some(10),
        '#' => Some(11),
        'A'..='D' => Some(12 + (digit as u8 - b'A')),
        _ => None,
    }
}

struct Dtmf {
    event: u8,
    pt: u8,
    ts: u32,
    duration: u16,
    ends: u32,
    first: bool,
}

#[derive(Default)]
struct Rx {
    received: u32,
    lost: i64,
    expected: u16,
    last_seq: u16,
    last_arrival: Option<Instant>,
    last_ts: u32,
    jitter: f64,
    max_jitter: f64,
}

#[derive(Default)]
struct Report {
    count: u32,
    lost: i32,
    fraction: f64,
    max_jitter_ms: f64,
}

struct Inner {
    ssrc: u32,
    seq: u16,
    ts: u32,
    mic: Vec<i16>,
    frame_samples: usize,
    ts_inc: u32,
    last_mic_at: Option<Instant>,
    voice: u32,
    silence: u32,
    max_gap_ms: u128,
    rx: Rx,
    report: Report,
    send: Option<StdSocket>,
    rtcp_send: Option<StdSocket>,
    pub port: u16,
    remote_ssrc: Option<u32>,
    sdp_ip: Option<IpAddr>,
    sdp_port: Option<u16>,
    rtcp_seen: bool,
    marker: bool,
    remote: Option<SocketAddr>,
    codec: Option<Codec>,
    started: bool,
    closed: bool,
    dtmf_queue: VecDeque<(u8, u8)>,
    dtmf: Option<Dtmf>,
    dtmf_gap: u32,
    tasks: Vec<AbortHandle>,
    decode_error_logged: bool,
    // Zustand der Codecs mit Gedächtnis – je Gespräch eigene Instanzen
    g722: Option<(g722::Encoder, g722::Decoder)>,
    opus: Option<(opus::Encoder, opus::Decoder)>,
    events: Rc<dyn Fn(RtpEvent)>,
}

#[derive(Clone)]
pub struct Rtp(Rc<RefCell<Inner>>);

fn random_u32() -> u32 {
    u32::from_le_bytes(uuid::Uuid::new_v4().as_bytes()[..4].try_into().unwrap())
}

fn bind(ip: IpAddr, port: u16) -> std::io::Result<StdSocket> {
    let s = StdSocket::bind((ip, port))?;
    s.set_nonblocking(true)?;
    Ok(s)
}

impl Rtp {
    pub fn new(events: Rc<dyn Fn(RtpEvent)>) -> Self {
        Rtp(Rc::new(RefCell::new(Inner {
            ssrc: random_u32(),
            seq: random_u32() as u16,
            ts: random_u32(),
            mic: Vec::with_capacity(960),
            frame_samples: FRAME,
            ts_inc: FRAME as u32,
            last_mic_at: None,
            voice: 0,
            silence: 0,
            max_gap_ms: 0,
            rx: Rx::default(),
            report: Report::default(),
            send: None,
            rtcp_send: None,
            port: 0,
            remote_ssrc: None,
            sdp_ip: None,
            sdp_port: None,
            rtcp_seen: false,
            marker: true,
            remote: None,
            codec: None,
            started: false,
            closed: false,
            dtmf_queue: VecDeque::new(),
            dtmf: None,
            dtmf_gap: 0,
            tasks: Vec::new(),
            decode_error_logged: false,
            g722: None,
            opus: None,
            events,
        })))
    }

    pub fn port(&self) -> u16 {
        self.0.borrow().port
    }

    pub fn has_remote(&self) -> bool {
        self.0.borrow().remote.is_some()
    }

    // RTP auf einem geraden Port, RTCP auf Port+1 (so erwartet es Asterisk); nur auf der Adresse zur Anlage
    // (sonst fragt die Windows-Firewall nach dem Heimnetz). Gibt es die Adresse nicht mehr, alle Netze.
    pub fn open(&self, local_ip: Option<IpAddr>) -> std::io::Result<()> {
        let mut ip = local_ip.unwrap_or(IpAddr::from([0, 0, 0, 0]));
        for _ in 0..8 {
            let rtp = match bind(ip, 0) {
                Ok(s) => s,
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::AddrNotAvailable {
                        ip = IpAddr::from([0, 0, 0, 0]);
                    }
                    continue;
                }
            };
            let port = rtp.local_addr()?.port();
            if port % 2 != 0 {
                continue;
            }
            let Ok(rtcp) = bind(ip, port + 1) else {
                continue;
            };
            crate::logger::info(&format!("RTP-Ports: {port} (RTP) / {} (RTCP)", port + 1));
            return self.listen(rtp, Some(rtcp), port);
        }
        // Kein Paar frei bekommen: RTP allein, ohne RTCP-Auswertung.
        let rtp = bind(ip, 0)?;
        let port = rtp.local_addr()?.port();
        crate::logger::info(&format!("RTP-Port: {port} (kein RTCP-Port verfügbar)"));
        self.listen(rtp, None, port)
    }

    fn listen(&self, rtp: StdSocket, rtcp: Option<StdSocket>, port: u16) -> std::io::Result<()> {
        let send = rtp.try_clone()?;
        let recv = UdpSocket::from_std(rtp)?;
        let me = self.clone();
        let task = tokio::task::spawn_local(async move {
            let mut buf = vec![0u8; 2048];
            loop {
                if let Ok((n, from)) = recv.recv_from(&mut buf).await {
                    me.on_packet(&buf[..n], from);
                }
            }
        });
        let mut tasks = vec![task.abort_handle()];
        let mut rtcp_send = None;
        if let Some(rtcp) = rtcp {
            rtcp_send = Some(rtcp.try_clone()?);
            let recv = UdpSocket::from_std(rtcp)?;
            let me = self.clone();
            let task = tokio::task::spawn_local(async move {
                let mut buf = vec![0u8; 2048];
                loop {
                    if let Ok((n, from)) = recv.recv_from(&mut buf).await {
                        me.on_rtcp(&buf[..n], from);
                    }
                }
            });
            tasks.push(task.abort_handle());
        }
        let mut i = self.0.borrow_mut();
        i.send = Some(send);
        i.rtcp_send = rtcp_send;
        i.port = port;
        i.tasks.extend(tasks);
        Ok(())
    }

    pub fn set_remote(&self, ip: &str, port: u16, codec: Codec) {
        let events = {
            let mut i = self.0.borrow_mut();
            i.codec = Some(codec);
            i.frame_samples = codec.frame;
            i.ts_inc = codec.clock / 50; // Zeitmarke je 20 ms
            if codec.kind == Kind::G722 && i.g722.is_none() {
                i.g722 = Some((g722::Encoder::default(), g722::Decoder::default()));
            }
            if codec.kind == Kind::Opus && i.opus.is_none() {
                let enc = opus::Encoder::new(48000, opus::Channels::Mono, opus::Application::Voip);
                let dec = opus::Decoder::new(48000, opus::Channels::Mono);
                match (enc, dec) {
                    (Ok(e), Ok(d)) => i.opus = Some((e, d)),
                    (Err(err), _) | (_, Err(err)) => {
                        crate::logger::warn(&format!("Opus nicht verfügbar: {err}"))
                    }
                }
            }
            i.sdp_ip = ip.parse().ok(); // nur von dieser Adresse (laut SDP) werden Sprachpakete angenommen
            i.sdp_port = (port > 0).then_some(port);
            i.remote = match (ip.parse::<IpAddr>(), port) {
                (Ok(addr), p) if p > 0 && !addr.is_unspecified() => Some(SocketAddr::new(addr, p)),
                _ => None,
            };
            i.events.clone()
        };
        // Oberfläche über die Abtastrate informieren (8 kHz G.711, 16 kHz G.722, 48 kHz Opus).
        events(RtpEvent::Format(codec.rate));
    }

    pub fn start(&self) {
        {
            let mut i = self.0.borrow_mut();
            if i.started || i.closed {
                return;
            }
            i.started = true;
        }
        let me = self.clone();
        let tick = tokio::task::spawn_local(async move {
            let mut next = Instant::now();
            loop {
                let now = Instant::now();
                let fresh =
                    me.0.borrow()
                        .last_mic_at
                        .is_some_and(|t| now - t < MIC_STALE);
                if fresh {
                    next = now + Duration::from_millis(20);
                } else {
                    if now > next + Duration::from_millis(200) {
                        next = now;
                    }
                    while next <= now {
                        let n = me.0.borrow().frame_samples;
                        me.send_frame(&vec![0; n]);
                        me.0.borrow_mut().silence += 1;
                        next += Duration::from_millis(20);
                    }
                }
                tokio::time::sleep_until(tokio::time::Instant::from_std(
                    next.max(Instant::now() + Duration::from_millis(1)),
                ))
                .await;
            }
        });
        let mut tasks = vec![tick.abort_handle()];
        if self.0.borrow().rtcp_send.is_some() {
            let me = self.clone();
            let rr = tokio::task::spawn_local(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    me.send_receiver_report();
                }
            });
            tasks.push(rr.abort_handle());
        }
        self.0.borrow_mut().tasks.extend(tasks);
    }

    pub fn push_mic(&self, samples: &[i16]) {
        let frame = {
            let mut i = self.0.borrow_mut();
            if !i.started {
                return; // vor Gesprächsbeginn nichts puffern (käme sonst als Verzögerung dazu)
            }
            let now = Instant::now();
            if let Some(last) = i.last_mic_at {
                i.max_gap_ms = i.max_gap_ms.max((now - last).as_millis());
            }
            i.last_mic_at = Some(now);
            i.frame_samples
        };
        let mut offset = 0;
        while offset < samples.len() {
            let full = {
                let mut i = self.0.borrow_mut();
                let n = (frame - i.mic.len()).min(samples.len() - offset);
                i.mic.extend_from_slice(&samples[offset..offset + n]);
                offset += n;
                (i.mic.len() == frame).then(|| std::mem::take(&mut i.mic))
            };
            if let Some(block) = full {
                self.send_frame(&block);
                self.0.borrow_mut().voice += 1;
            }
        }
    }

    pub fn send_dtmf(&self, digit: char, pt: u8) {
        if let Some(event) = dtmf_event(digit) {
            self.0.borrow_mut().dtmf_queue.push_back((event, pt));
        }
    }

    fn send_frame(&self, frame: &[i16]) {
        let mut i = self.0.borrow_mut();
        if i.dtmf_gap > 0 {
            i.dtmf_gap -= 1;
        } else if i.dtmf.is_none() {
            if let Some((event, pt)) = i.dtmf_queue.pop_front() {
                let ts = i.ts;
                i.dtmf = Some(Dtmf {
                    event,
                    pt,
                    ts,
                    duration: 0,
                    ends: 0,
                    first: true,
                });
            }
        }
        // Während eines Tastentons ersetzt das telephone-event-Paket das Sprachpaket.
        if i.dtmf.is_some() {
            send_dtmf_packet(&mut i);
        } else if let (Some(remote), Some(codec)) = (i.remote, i.codec) {
            let payload = encode(&mut i, codec, frame);
            let mut packet = Vec::with_capacity(12 + payload.len());
            packet.push(0x80);
            packet.push((if i.marker { 0x80 } else { 0 }) | codec.pt);
            packet.extend_from_slice(&i.seq.to_be_bytes());
            packet.extend_from_slice(&i.ts.to_be_bytes());
            packet.extend_from_slice(&i.ssrc.to_be_bytes());
            packet.extend_from_slice(&payload);
            if let Some(s) = &i.send {
                let _ = s.send_to(&packet, remote);
            }
            i.marker = false;
        }
        i.seq = i.seq.wrapping_add(1);
        i.ts = i.ts.wrapping_add(i.ts_inc);
    }

    fn on_packet(&self, buf: &[u8], from: SocketAddr) {
        let (codec, events) = {
            let i = self.0.borrow();
            // Nur die Adresse aus der SDP (Anlage oder bei Direktverbindung das andere Telefon).
            if i.sdp_ip != Some(from.ip()) {
                return;
            }
            let Some(codec) = i.codec else { return };
            (codec, i.events.clone())
        };
        if buf.len() < 12 || buf[0] >> 6 != 2 || buf[1] & 0x7f != codec.pt {
            return;
        }
        let mut offset = 12 + (buf[0] & 0x0f) as usize * 4;
        if buf[0] & 0x10 != 0 {
            if buf.len() < offset + 4 {
                return;
            }
            offset += 4 + u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize * 4;
        }
        let mut end = buf.len();
        if buf[0] & 0x20 != 0 {
            end = end.saturating_sub(buf[end - 1] as usize);
        }
        if end <= offset {
            return;
        }
        {
            let mut i = self.0.borrow_mut();
            // Symmetrisches RTP: dorthin zurücksenden, woher die Gegenstelle sendet (hilft bei NAT).
            i.remote = Some(from);
            i.remote_ssrc = Some(u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]));
            count_received(
                &mut i,
                u16::from_be_bytes([buf[2], buf[3]]),
                u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]),
                buf[1] & 0x80 != 0,
            );
        }
        let payload = &buf[offset..end];
        let pcm: Vec<i16> = match codec.kind {
            Kind::Pcma => payload.iter().map(|&b| alaw_to_linear(b)).collect(),
            Kind::Pcmu => payload.iter().map(|&b| ulaw_to_linear(b)).collect(),
            Kind::G722 => match self.0.borrow_mut().g722.as_mut() {
                Some((_, dec)) => dec.decode(payload), // 160 Byte -> 320 Samples
                None => return,
            },
            Kind::Opus => {
                let mut i = self.0.borrow_mut();
                let Some((_, dec)) = i.opus.as_mut() else {
                    return;
                };
                let mut out = vec![0i16; 960 * 6]; // bis 120 ms je Paket
                match dec.decode(payload, &mut out, false) {
                    Ok(n) => {
                        out.truncate(n);
                        out
                    }
                    Err(err) => {
                        // Ein kaputtes Paket verwerfen, statt die Audioverarbeitung abzubrechen.
                        if !i.decode_error_logged {
                            crate::logger::warn(&format!(
                                "RTP-Paket nicht dekodierbar, verworfen: {err}"
                            ));
                            i.decode_error_logged = true;
                        }
                        return;
                    }
                }
            }
        };
        events(RtpEvent::Audio(pcm));
    }

    // Kleiner eigener Empfangsbericht (RR): bringt Asterisk dazu, seinerseits Berichte zu schicken.
    fn send_receiver_report(&self) {
        let i = self.0.borrow();
        let (Some(sock), Some(remote), Some(port), Some(remote_ssrc)) =
            (&i.rtcp_send, i.remote, i.sdp_port, i.remote_ssrc)
        else {
            return;
        };
        let mut p = vec![0x81, 201];
        p.extend_from_slice(&7u16.to_be_bytes()); // Länge 32/4-1
        p.extend_from_slice(&i.ssrc.to_be_bytes());
        p.extend_from_slice(&remote_ssrc.to_be_bytes());
        let lost = i.rx.lost.clamp(0, 0xffffff) as u32;
        p.push(0); // Verlustanteil vereinfachend 0
        p.extend_from_slice(&lost.to_be_bytes()[1..]);
        p.extend_from_slice(&(i.rx.last_seq as u32).to_be_bytes());
        p.extend_from_slice(&(i.rx.jitter.round() as u32).to_be_bytes());
        p.extend_from_slice(&[0; 8]); // LSR, DLSR
        let _ = sock.send_to(&p, SocketAddr::new(remote.ip(), port + 1));
    }

    // RTCP der Anlage: was sie über UNSEREN Sendestrom meldet (Verlust, Jitter).
    fn on_rtcp(&self, buf: &[u8], from: SocketAddr) {
        let mut i = self.0.borrow_mut();
        if !i.rtcp_seen {
            i.rtcp_seen = true;
            let note = if i.sdp_ip == Some(from.ip()) {
                String::new()
            } else {
                " (fremde Adresse, wird verworfen)".into()
            };
            crate::logger::info(&format!("Erstes RTCP von {from}{note}"));
        }
        if i.sdp_ip != Some(from.ip()) {
            return;
        }
        let mut off = 0;
        while off + 4 <= buf.len() {
            if buf[off] >> 6 != 2 {
                break;
            }
            let pt = buf[off + 1];
            let len = (u16::from_be_bytes([buf[off + 2], buf[off + 3]]) as usize + 1) * 4;
            if off + len > buf.len() {
                break;
            }
            if pt == 200 || pt == 201 {
                let count = (buf[off] & 0x1f) as usize;
                let mut rb = off + if pt == 200 { 28 } else { 8 };
                for _ in 0..count {
                    if rb + 24 > off + len {
                        break;
                    }
                    let ssrc = u32::from_be_bytes([buf[rb], buf[rb + 1], buf[rb + 2], buf[rb + 3]]);
                    if ssrc == i.ssrc {
                        let lost = ((u32::from_be_bytes([
                            buf[rb + 4],
                            buf[rb + 5],
                            buf[rb + 6],
                            buf[rb + 7],
                        ]) << 8) as i32)
                            >> 8;
                        let jitter_ms = u32::from_be_bytes([
                            buf[rb + 12],
                            buf[rb + 13],
                            buf[rb + 14],
                            buf[rb + 15],
                        ]) as f64
                            / 8.0;
                        i.report.count += 1;
                        i.report.fraction = buf[rb + 4] as f64 / 256.0;
                        i.report.lost = lost;
                        i.report.max_jitter_ms = i.report.max_jitter_ms.max(jitter_ms);
                    }
                    rb += 24;
                }
            }
            off += len;
        }
    }

    pub fn close(&self) {
        let mut i = self.0.borrow_mut();
        if i.started && !i.closed {
            let codec = i.codec.map(|c| c.name()).unwrap_or("?");
            let r = &i.report;
            let report = if r.count > 0 {
                format!(
                    "{} Berichte, {} Pakete verloren (zuletzt {:.1} %), Jitter max {:.1} ms",
                    r.count,
                    r.lost,
                    r.fraction * 100.0,
                    r.max_jitter_ms
                )
            } else {
                "keine RTCP-Berichte empfangen".into()
            };
            crate::logger::info(&format!(
                "RTP-Statistik (Codec {codec}):\n  gesendet: {} Sprach-, {} Stillepakete, längste Mikrofonpause {} ms\n  empfangen: {} Pakete, {} Lücken (geschätzt), Jitter max {:.1} ms\n  Anlage meldet über unseren Sendestrom: {report}",
                i.voice,
                i.silence,
                i.max_gap_ms,
                i.rx.received,
                i.rx.lost,
                i.rx.max_jitter / 8.0
            ));
        }
        i.closed = true;
        for t in i.tasks.drain(..) {
            t.abort();
        }
        i.send = None;
        i.rtcp_send = None;
    }
}

// frame: PCM eines 20-ms-Blocks in der Rate des Codecs -> RTP-Payload.
fn encode(i: &mut Inner, codec: Codec, frame: &[i16]) -> Vec<u8> {
    match codec.kind {
        Kind::Pcmu => frame
            .iter()
            .take(FRAME)
            .map(|&s| linear_to_ulaw(s))
            .collect(),
        Kind::Pcma => frame
            .iter()
            .take(FRAME)
            .map(|&s| linear_to_alaw(s))
            .collect(),
        Kind::G722 => i
            .g722
            .as_mut()
            .map(|(enc, _)| enc.encode(frame))
            .unwrap_or_default(), // 320 Samples -> 160 Byte
        Kind::Opus => {
            let Some((enc, _)) = i.opus.as_mut() else {
                return Vec::new();
            };
            let mut out = vec![0u8; 1500];
            match enc.encode(frame, &mut out) {
                Ok(n) => {
                    out.truncate(n);
                    out
                }
                Err(_) => Vec::new(),
            }
        }
    }
}

// RFC 4733: ein Ereignis behält seine Zeitmarke, die Dauer wächst je Paket, am Ende E-Bit.
fn send_dtmf_packet(i: &mut Inner) {
    let Inner {
        dtmf,
        dtmf_gap,
        send,
        remote,
        ssrc,
        seq,
        ..
    } = i;
    let Some(d) = dtmf.as_mut() else { return };
    let end = d.duration >= DTMF_DURATION;
    if end {
        d.ends += 1;
    } else {
        d.duration += FRAME as u16;
    }
    if let (Some(remote), Some(sock)) = (remote, send) {
        let mut p = vec![0x80, (if d.first { 0x80 } else { 0 }) | d.pt];
        p.extend_from_slice(&seq.to_be_bytes());
        p.extend_from_slice(&d.ts.to_be_bytes());
        p.extend_from_slice(&ssrc.to_be_bytes());
        p.push(d.event);
        p.push((if end { 0x80 } else { 0 }) | DTMF_VOLUME);
        p.extend_from_slice(&d.duration.to_be_bytes());
        let _ = sock.send_to(&p, *remote);
    }
    d.first = false;
    if end && d.ends >= DTMF_END_PACKETS {
        *dtmf = None;
        *dtmf_gap = DTMF_GAP_FRAMES;
    }
}

// Empfangene Pakete zählen und den Jitter der Gegenrichtung schätzen (RFC 3550).
fn count_received(i: &mut Inner, seq: u16, ts: u32, marker: bool) {
    let rx = &mut i.rx;
    rx.received += 1;
    rx.last_seq = seq;
    if rx.received == 1 {
        rx.expected = seq.wrapping_add(1);
    } else {
        let gap = seq.wrapping_sub(rx.expected);
        if gap < 0x8000 {
            rx.lost += gap as i64;
            rx.expected = seq.wrapping_add(1);
        }
    }
    let now = Instant::now();
    if let (Some(last), false) = (rx.last_arrival, marker) {
        let dts = ts.wrapping_sub(rx.last_ts) as i32 as f64;
        let d = ((now - last).as_secs_f64() * 8000.0 - dts).abs();
        if d < 8000.0 {
            rx.jitter += (d - rx.jitter) / 16.0;
            rx.max_jitter = rx.max_jitter.max(rx.jitter);
        }
    }
    rx.last_arrival = Some(now);
    rx.last_ts = ts;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g711_matches_reference_values() {
        // Werte der Sun-Referenz (wie die Electron-Version)
        assert_eq!(linear_to_alaw(0), 0xd5);
        assert_eq!(linear_to_alaw(-1), 0x55);
        assert_eq!(linear_to_alaw(32767), 0xaa);
        assert_eq!(linear_to_ulaw(0), 0xff);
        assert_eq!(linear_to_ulaw(-32768), 0x00);
        assert_eq!(alaw_to_linear(0xd5), 8);
        assert_eq!(ulaw_to_linear(0xff), 0);
        // Rundreise: Fehler bleibt im Rahmen der Quantisierung
        for s in [-30000i16, -1000, -10, 0, 10, 1000, 30000] {
            assert!(
                (alaw_to_linear(linear_to_alaw(s)) as i32 - s as i32).abs()
                    <= (s as i32).abs() / 16 + 16,
                "A-law {s}"
            );
            assert!(
                (ulaw_to_linear(linear_to_ulaw(s)) as i32 - s as i32).abs()
                    <= (s as i32).abs() / 16 + 16,
                "u-law {s}"
            );
        }
    }

    // Prüfsummen aus der JS-Umsetzung der Electron-Version (src/rtp.js): gleiche Tabellen, gleiche Kodierung.
    #[test]
    fn g711_tables_identical_to_electron() {
        use md5::{Digest, Md5};
        let hash = |v: Vec<String>| format!("{:x}", Md5::digest(v.join(",").as_bytes()));
        assert_eq!(
            hash((0..=255u8).map(|b| alaw_to_linear(b).to_string()).collect()),
            "5a429dc69e01360be265667574209d9a"
        );
        assert_eq!(
            hash((0..=255u8).map(|b| ulaw_to_linear(b).to_string()).collect()),
            "771fdb1dc7a559d5ab5eeb5b7d2766cb"
        );
        let mut enc = Vec::new();
        let mut s: i32 = -32768;
        while s < 32768 {
            enc.push(linear_to_alaw(s as i16).to_string());
            enc.push(linear_to_ulaw(s as i16).to_string());
            s += 7;
        }
        assert_eq!(hash(enc), "38df09bfeb14d3d9e4b905b7794bf37a");
    }

    #[test]
    fn dtmf_events() {
        assert_eq!(dtmf_event('5'), Some(5));
        assert_eq!(dtmf_event('*'), Some(10));
        assert_eq!(dtmf_event('#'), Some(11));
        assert_eq!(dtmf_event('D'), Some(15));
        assert_eq!(dtmf_event('x'), None);
    }
}
