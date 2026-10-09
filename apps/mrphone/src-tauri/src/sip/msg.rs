// SIP-Nachrichten lesen und schreiben – wie parseMessage/serialize in src/sip.js der Electron-Version.
use std::collections::HashMap;

const LIST_HEADERS: [&str; 6] = [
    "via",
    "route",
    "record-route",
    "contact",
    "p-asserted-identity",
    "remote-party-id",
];

fn compact(name: &str) -> &str {
    match name {
        "v" => "via",
        "f" => "from",
        "t" => "to",
        "i" => "call-id",
        "m" => "contact",
        "l" => "content-length",
        "c" => "content-type",
        "k" => "supported",
        other => other,
    }
}

#[derive(Debug, Clone, Default)]
pub struct Msg {
    pub method: Option<String>,
    pub uri: String,
    pub status: u16,
    pub reason: String,
    pub headers: HashMap<String, Vec<String>>,
    pub body: String,
}

impl Msg {
    pub fn header(&self, name: &str) -> &str {
        self.headers
            .get(name)
            .and_then(|v| v.first())
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn all(&self, name: &str) -> &[String] {
        self.headers.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    // Antwort ohne Netz (z. B. Zeitüberschreitung einer Transaktion).
    pub fn synthetic(status: u16, reason: &str) -> Self {
        Self {
            status,
            reason: reason.into(),
            ..Default::default()
        }
    }
}

// SIP ist UTF-8, manche Anlagen schicken Namen aber in Windows-1252 (z. B. „ö“ als Byte 0xF6).
pub fn decode_text(buf: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(buf) {
        return s.to_string();
    }
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    buf.iter()
        .map(|&b| {
            if (0x80..0xa0).contains(&b) {
                HIGH[(b - 0x80) as usize]
            } else {
                b as char
            }
        })
        .collect()
}

// Trennt an Kommas außerhalb von "..." und <...>.
fn split_list(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut angle) = (false, false);
    for ch in value.chars() {
        match ch {
            '"' => quoted = !quoted,
            '<' if !quoted => angle = true,
            '>' if !quoted => angle = false,
            _ => {}
        }
        if ch == ',' && !quoted && !angle {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(ch);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

pub fn parse(buf: &[u8]) -> Option<Msg> {
    let text = decode_text(buf);
    let sep = text.find("\r\n\r\n")?;
    let mut lines: Vec<String> = Vec::new();
    for line in text[..sep].split("\r\n") {
        if (line.starts_with(' ') || line.starts_with('\t')) && !lines.is_empty() {
            let last = lines.last_mut().unwrap();
            last.push(' ');
            last.push_str(line.trim());
        } else {
            lines.push(line.to_string());
        }
    }
    let first = lines.remove(0);
    let mut msg = Msg {
        body: text[sep + 4..].to_string(),
        ..Default::default()
    };
    if let Some(rest) = first.strip_prefix("SIP/2.0 ") {
        let status = rest.get(..3)?;
        if !status.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        msg.status = status.parse().ok()?;
        msg.reason = rest[3..].trim().to_string();
    } else {
        let parts: Vec<&str> = first.split(' ').collect();
        if parts.len() != 3
            || parts[2] != "SIP/2.0"
            || parts[0].is_empty()
            || !parts[0].bytes().all(|b| b.is_ascii_uppercase())
        {
            return None;
        }
        msg.method = Some(parts[0].to_string());
        msg.uri = parts[1].to_string();
    }
    for line in lines {
        let Some(i) = line.find(':') else { continue };
        let name = line[..i].trim().to_lowercase();
        let name = compact(&name).to_string();
        let value = line[i + 1..].trim();
        let values = if LIST_HEADERS.contains(&name.as_str()) {
            split_list(value)
        } else {
            vec![value.to_string()]
        };
        msg.headers.entry(name).or_default().extend(values);
    }
    if let Ok(len) = msg.header("content-length").trim().parse::<usize>() {
        // Content-Length zählt Bytes
        let bytes = msg.body.as_bytes();
        if len < bytes.len() {
            msg.body = String::from_utf8_lossy(&bytes[..len]).to_string();
        }
    }
    Some(msg)
}

pub fn serialize(
    start_line: &str,
    headers: &[(String, String)],
    body: &str,
    content_type: Option<&str>,
) -> Vec<u8> {
    let mut out = String::from(start_line);
    out.push_str("\r\n");
    for (n, v) in headers {
        out.push_str(&format!("{n}: {v}\r\n"));
    }
    if !body.is_empty() {
        out.push_str(&format!(
            "Content-Type: {}\r\n",
            content_type.unwrap_or("application/sdp")
        ));
    }
    out.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    out.push_str(body);
    out.into_bytes()
}

pub fn parse_params(s: &str) -> HashMap<String, String> {
    s.split(';')
        .filter_map(|part| {
            let mut kv = part.splitn(2, '=');
            let k = kv.next()?.trim().to_lowercase();
            (!k.is_empty()).then(|| {
                (
                    k,
                    kv.next().map(|v| v.trim().to_string()).unwrap_or_default(),
                )
            })
        })
        .collect()
}

pub struct Addr {
    pub display: String,
    pub uri: String,
    pub params: HashMap<String, String>,
}

// '"Name" <sip:user@host>;tag=x' -> Anzeigename, URI, Parameter
pub fn parse_addr(value: &str) -> Addr {
    if let (Some(lt), Some(gt)) = (value.find('<'), value.rfind('>')) {
        if lt < gt {
            let before = value[..lt].trim();
            let display = if before.len() >= 2 && before.starts_with('"') && before.ends_with('"') {
                let inner = &before[1..before.len() - 1];
                let mut out = String::new();
                let mut esc = false;
                for c in inner.chars() {
                    if esc || c != '\\' {
                        out.push(c);
                        esc = false;
                    } else {
                        esc = true;
                    }
                }
                out
            } else {
                before.to_string()
            };
            return Addr {
                display: display.trim().to_string(),
                uri: value[lt + 1..gt].to_string(),
                params: parse_params(&value[gt + 1..]),
            };
        }
    }
    let (uri, params) = match value.find(';') {
        Some(i) => (&value[..i], &value[i..]),
        None => (value, ""),
    };
    Addr {
        display: String::new(),
        uri: uri.trim().to_string(),
        params: parse_params(params),
    }
}

pub fn via_branch(via: &str) -> String {
    let params = via.find(';').map(|i| &via[i..]).unwrap_or("");
    parse_params(params)
        .get("branch")
        .cloned()
        .unwrap_or_default()
}

// Status einer Nebenstelle aus dialog-info+xml (RFC 4235): busy / ringing / idle.
pub fn parse_dialog_info(xml: &str) -> &'static str {
    let mut states = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<state") {
        rest = &rest[i + 6..];
        let Some(gt) = rest.find('>') else { break };
        let Some(end) = rest.find("</state>") else {
            break;
        };
        if gt < end {
            states.push(rest[gt + 1..end].trim().to_lowercase());
        }
        rest = &rest[end..];
    }
    if states.iter().any(|s| s == "confirmed") {
        "busy"
    } else if states
        .iter()
        .any(|s| matches!(s.as_str(), "early" | "proceeding" | "trying"))
    {
        "ringing"
    } else {
        "idle"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_response_with_folding_compact_and_lists() {
        let raw = b"SIP/2.0 200 OK\r\nv: SIP/2.0/UDP 10.0.0.1:5060;branch=z9hG4bKa;rport\r\nVia: SIP/2.0/UDP x;branch=b\r\nContact: <sip:a@1;line=x>;expires=600, \"B, C\" <sip:b@2>\r\nFrom: <sip:742@s>;tag=1\r\nX-Long: a\r\n  b\r\nContent-Length: 4\r\n\r\nbodyEXTRA";
        let m = parse(raw).unwrap();
        assert_eq!(m.status, 200);
        assert_eq!(m.all("via").len(), 2);
        assert_eq!(via_branch(m.header("via")), "z9hG4bKa");
        assert_eq!(m.all("contact").len(), 2);
        assert_eq!(m.header("x-long"), "a b");
        assert_eq!(m.body, "body");
    }

    #[test]
    fn parses_request_and_rejects_garbage() {
        let m = parse(b"OPTIONS sip:742@1.2.3.4:5060 SIP/2.0\r\nCall-ID: x\r\n\r\n").unwrap();
        assert_eq!(m.method.as_deref(), Some("OPTIONS"));
        assert_eq!(m.uri, "sip:742@1.2.3.4:5060");
        assert!(parse(b"hello\r\n\r\n").is_none());
        assert!(parse(b"\r\n\r\n").is_none());
        assert!(parse(b"SIP/2.0 2x0 OK\r\n\r\n").is_none());
        assert!(parse(b"no separator").is_none());
    }

    #[test]
    fn windows_1252_names() {
        assert_eq!(decode_text(b"J\xf6rg \x80"), "Jörg €");
        assert_eq!(decode_text("Jörg".as_bytes()), "Jörg");
    }

    #[test]
    fn addresses() {
        let a = parse_addr("\"M\\\"ax\" <sip:742@x>;tag=abc");
        assert_eq!(a.display, "M\"ax");
        assert_eq!(a.uri, "sip:742@x");
        assert_eq!(a.params.get("tag").map(String::as_str), Some("abc"));
        let b = parse_addr("sip:1@y;tag=t");
        assert_eq!((b.uri.as_str(), b.params["tag"].as_str()), ("sip:1@y", "t"));
    }

    #[test]
    fn dialog_info() {
        assert_eq!(
            parse_dialog_info("<dialog><state>confirmed</state></dialog>"),
            "busy"
        );
        assert_eq!(
            parse_dialog_info("<state event=\"x\"> early </state>"),
            "ringing"
        );
        assert_eq!(parse_dialog_info("<dialog-info/>"), "idle");
        assert_eq!(parse_dialog_info(""), "idle");
    }
}
