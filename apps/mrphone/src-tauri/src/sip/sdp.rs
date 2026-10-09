// SDP-Aushandlung – wie src/sdp.js der Electron-Version. rate = Audioabtastrate, frame = PCM-Samples je
// 20 ms, clock = RTP-Zeittakt. HD-Reihenfolge: Opus vor G.722 vor G.711.
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pcma,
    Pcmu,
    G722,
    Opus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Codec {
    pub kind: Kind,
    pub pt: u8,
    pub rate: u32,
    pub frame: usize,
    pub clock: u32,
}

impl Codec {
    pub fn name(&self) -> &'static str {
        match self.kind {
            Kind::Pcma => "PCMA",
            Kind::Pcmu => "PCMU",
            Kind::G722 => "G722",
            Kind::Opus => "OPUS",
        }
    }

    fn rtp_name(&self) -> &'static str {
        if self.kind == Kind::Opus {
            "opus"
        } else {
            self.name()
        }
    }
}

const PCMA: Codec = Codec {
    kind: Kind::Pcma,
    pt: 8,
    rate: 8000,
    frame: 160,
    clock: 8000,
};
const PCMU: Codec = Codec {
    kind: Kind::Pcmu,
    pt: 0,
    rate: 8000,
    frame: 160,
    clock: 8000,
};
pub const DTMF_PT: u8 = 101;

const G722: Codec = Codec {
    kind: Kind::G722,
    pt: 9,
    rate: 16000,
    frame: 320,
    clock: 8000,
};
const OPUS: Codec = Codec {
    kind: Kind::Opus,
    pt: 111,
    rate: 48000,
    frame: 960,
    clock: 48000,
};

// Angebots-/Akzeptanzliste: mit HD Opus bevorzugt, dann G.722, sonst nur G.711.
pub fn offer_codecs(hd: bool) -> Vec<Codec> {
    if hd {
        vec![OPUS, G722, PCMA, PCMU]
    } else {
        vec![PCMA, PCMU]
    }
}

#[derive(Debug, Clone)]
pub struct Remote {
    pub ip: String,
    pub port: u16,
    pub pts: Vec<u8>,
    pub dtmf_pt: Option<u8>,
    pub direction: String,
    pub rtpmap: HashMap<u8, String>,
}

pub fn parse(text: &str) -> Option<Remote> {
    let mut section = "session";
    let (mut session_ip, mut audio_ip) = (None::<String>, None::<String>);
    let mut audio: Option<(u16, Vec<u8>)> = None;
    let mut direction = "sendrecv".to_string();
    let mut rtpmap = HashMap::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(m) = line.strip_prefix("m=") {
            let parts: Vec<&str> = m.split_whitespace().collect();
            if audio.is_none() && parts.len() >= 3 && parts[0] == "audio" {
                if let Ok(port) = parts[1].parse() {
                    audio = Some((
                        port,
                        parts[3..].iter().filter_map(|p| p.parse().ok()).collect(),
                    ));
                    section = "audio";
                    continue;
                }
            }
            section = "other";
            continue;
        }
        if section == "other" {
            continue;
        }
        if let Some(ip) = line.strip_prefix("c=IN IP4 ") {
            let ip = ip.split_whitespace().next().unwrap_or("").to_string();
            if section == "audio" {
                audio_ip = Some(ip);
            } else {
                session_ip = Some(ip);
            }
        } else if let Some(map) = line.strip_prefix("a=rtpmap:") {
            let mut it = map.splitn(2, ' ');
            if let (Some(pt), Some(rest)) = (it.next().and_then(|p| p.parse().ok()), it.next()) {
                rtpmap.insert(
                    pt,
                    rest.split('/').next().unwrap_or("").trim().to_uppercase(),
                );
            }
        } else if let Some(d) = line.strip_prefix("a=") {
            if matches!(d.trim(), "sendrecv" | "sendonly" | "recvonly" | "inactive") {
                direction = d.trim().to_string();
            }
        }
    }
    let (port, pts) = audio?;
    let dtmf_pt = pts
        .iter()
        .copied()
        .find(|pt| rtpmap.get(pt).map(String::as_str) == Some("TELEPHONE-EVENT"));
    Some(Remote {
        ip: audio_ip.or(session_ip).unwrap_or_default(),
        port,
        pts,
        dtmf_pt,
        direction,
        rtpmap,
    })
}

fn static_name(pt: u8) -> Option<&'static str> {
    match pt {
        0 => Some("PCMU"),
        8 => Some("PCMA"),
        9 => Some("G722"),
        _ => None,
    }
}

// Erster Codec aus der Liste der Gegenstelle, den wir können.
pub fn choose_codec(remote: &Remote, hd: bool) -> Option<Codec> {
    let supported = offer_codecs(hd);
    for &pt in &remote.pts {
        let Some(name) = remote
            .rtpmap
            .get(&pt)
            .cloned()
            .or_else(|| static_name(pt).map(String::from))
        else {
            continue; // unbekannter Payload-Typ ohne rtpmap
        };
        if let Some(c) = supported.iter().find(|c| c.name() == name) {
            return Some(Codec { pt, ..*c });
        }
    }
    None
}

pub struct Local<'a> {
    pub ip: &'a str,
    pub port: u16,
    pub session_id: u64,
    pub version: u32,
    pub codec: Option<Codec>,
    pub offer: Vec<Codec>,
    pub dtmf_pt: Option<u8>,
    pub direction: &'a str,
}

// Ohne ausgehandelten Codec ein Angebot mit allen Codecs, sonst eine Antwort.
pub fn build(l: &Local) -> String {
    let codecs = match l.codec {
        Some(c) => vec![c],
        None => l.offer.clone(),
    };
    let dtmf = if l.codec.is_some() {
        l.dtmf_pt
    } else {
        Some(DTMF_PT)
    };
    let mut pts: Vec<String> = codecs.iter().map(|c| c.pt.to_string()).collect();
    if let Some(d) = dtmf {
        pts.push(d.to_string());
    }
    let mut lines = vec![
        "v=0".to_string(),
        format!("o=- {} {} IN IP4 {}", l.session_id, l.version, l.ip),
        "s=mrphone".into(),
        format!("c=IN IP4 {}", l.ip),
        "t=0 0".into(),
        format!("m=audio {} RTP/AVP {}", l.port, pts.join(" ")),
    ];
    for c in &codecs {
        let channels = if c.kind == Kind::Opus { "/2" } else { "" };
        lines.push(format!(
            "a=rtpmap:{} {}/{}{channels}",
            c.pt,
            c.rtp_name(),
            c.clock
        ));
        if c.kind == Kind::Opus {
            lines.push(format!("a=fmtp:{} useinbandfec=1", c.pt));
        }
    }
    if let Some(d) = dtmf {
        lines.push(format!("a=rtpmap:{d} telephone-event/8000"));
        lines.push(format!("a=fmtp:{d} 0-16"));
    }
    lines.push("a=ptime:20".into());
    lines.push(format!("a={}", l.direction));
    lines.join("\r\n") + "\r\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASTERISK: &str = "v=0\r\no=root 1 1 IN IP4 10.49.200.6\r\ns=Asterisk\r\nc=IN IP4 10.49.200.6\r\nt=0 0\r\nm=audio 12000 RTP/AVP 0 8 101\r\na=rtpmap:0 PCMU/8000\r\na=rtpmap:8 PCMA/8000\r\na=rtpmap:101 telephone-event/8000\r\na=fmtp:101 0-16\r\na=sendonly\r\nm=video 0 RTP/AVP 34\r\nc=IN IP4 9.9.9.9\r\n";

    #[test]
    fn parses_asterisk_offer() {
        let r = parse(ASTERISK).unwrap();
        assert_eq!((r.ip.as_str(), r.port), ("10.49.200.6", 12000));
        assert_eq!(r.pts, vec![0, 8, 101]);
        assert_eq!(r.dtmf_pt, Some(101));
        assert_eq!(r.direction, "sendonly");
        assert_eq!(choose_codec(&r, true).unwrap().name(), "PCMU"); // Reihenfolge der Gegenstelle
    }

    #[test]
    fn static_payload_without_rtpmap_and_no_match() {
        let r = parse("v=0\r\nc=IN IP4 1.2.3.4\r\nm=audio 4000 RTP/AVP 8\r\n").unwrap();
        assert_eq!(choose_codec(&r, false).unwrap().pt, 8);
        let none = parse(
            "v=0\r\nc=IN IP4 1.2.3.4\r\nm=audio 4000 RTP/AVP 18\r\na=rtpmap:18 G729/8000\r\n",
        )
        .unwrap();
        assert!(choose_codec(&none, false).is_none());
        assert!(parse("v=0\r\n").is_none());
    }

    #[test]
    fn builds_offer_and_answer() {
        let offer = build(&Local {
            ip: "10.0.0.5",
            port: 40000,
            session_id: 7,
            version: 1,
            codec: None,
            offer: offer_codecs(false),
            dtmf_pt: None,
            direction: "sendrecv",
        });
        assert!(offer.contains("m=audio 40000 RTP/AVP 8 0 101\r\n"));
        assert!(offer.contains("a=rtpmap:101 telephone-event/8000\r\n"));
        let answer = build(&Local {
            ip: "10.0.0.5",
            port: 40000,
            session_id: 7,
            version: 2,
            codec: Some(PCMA),
            offer: vec![],
            dtmf_pt: None,
            direction: "recvonly",
        });
        assert!(
            answer.contains("m=audio 40000 RTP/AVP 8\r\n") && answer.ends_with("a=recvonly\r\n")
        );
    }
}
