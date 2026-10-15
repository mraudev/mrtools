//! Connections to import from what is already on this PC: the servers Remotedesktop remembers,
//! `~/.ssh/config`, PuTTY's saved sessions and `.rdp` files. Nothing is changed there.

use crate::active::{rdp_file_target, split_host_port};
use crate::launch::Kind;
use crate::win;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    kind: Kind,
    name: String,
    host: String,
    port: Option<u16>,
    user: String,
    baud: Option<u32>,
    /// Where it was found.
    source: String,
    /// Extra information for the import list (e.g. the real host behind an ssh alias).
    detail: String,
}

impl Candidate {
    fn new(kind: Kind, host: &str, source: &str) -> Self {
        Self {
            kind,
            name: host.into(),
            host: host.into(),
            port: None,
            user: String::new(),
            baud: None,
            source: source.into(),
            detail: String::new(),
        }
    }
}

const RDP_CLIENT: &str = "Software\\Microsoft\\Terminal Server Client";
const PUTTY_SESSIONS: &str = "Software\\SimonTatham\\PuTTY\\Sessions";

/// Everything found in the Remotedesktop history, `~/.ssh/config` and PuTTY.
#[tauri::command]
pub fn import_sources() -> Vec<Candidate> {
    let mut out = rdp_history();
    if let Some(home) = std::env::var_os("USERPROFILE") {
        if let Ok(text) = std::fs::read_to_string(Path::new(&home).join(".ssh").join("config")) {
            out.extend(ssh_config(&text));
        }
    }
    out.extend(putty_sessions());
    out
}

/// Reads `.rdp` files chosen by the user.
#[tauri::command]
pub fn import_rdp_files(paths: Vec<String>) -> Vec<Candidate> {
    paths
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rdp"))
        .filter_map(|p| {
            let target = rdp_file_target(&std::fs::read(p).ok()?)?;
            let name = Path::new(p).file_stem()?.to_string_lossy().into_owned();
            let mut c = Candidate::new(Kind::Rdp, &target.host, "RDP-Datei");
            c.name = name;
            c.port = target.port.filter(|&port| port != 3389);
            c.user = target.user;
            c.detail = p.clone();
            Some(c)
        })
        .collect()
}

fn rdp_history() -> Vec<Candidate> {
    let servers = win::reg_subkeys(&format!("{RDP_CLIENT}\\Servers"));
    let recent = (0..10).filter_map(|i| win::reg_string(&format!("{RDP_CLIENT}\\Default"), &format!("MRU{i}")));
    let mut seen = HashSet::new();
    servers
        .into_iter()
        .chain(recent)
        .filter_map(|entry| {
            let (host, port) = split_host_port(&entry);
            if host.is_empty() || !seen.insert((host.to_ascii_lowercase(), port)) {
                return None;
            }
            let mut c = Candidate::new(Kind::Rdp, &host, "Remotedesktop-Verlauf");
            c.port = port.filter(|&p| p != 3389);
            c.user = win::rdp_username_hint(&entry).unwrap_or_default();
            Some(c)
        })
        .collect()
}

/// Concrete `Host` entries (no patterns). The alias becomes the host, so ssh keeps using the
/// settings from the config (HostName, ProxyJump, …); those are only shown as detail.
fn ssh_config(text: &str) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    // Indices of the entries the current `Host` block applies to.
    let mut block: Vec<usize> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once(|c: char| c.is_whitespace() || c == '=')
            .map(|(k, v)| (k.to_ascii_lowercase(), v.trim_start_matches([' ', '\t', '=']).trim()))
            .unwrap_or((line.to_ascii_lowercase(), ""));
        match key.as_str() {
            "host" => {
                block.clear();
                for alias in value.split_whitespace().filter(|a| !a.contains(['*', '?', '!'])) {
                    block.push(out.len());
                    out.push(Candidate::new(Kind::Ssh, alias, "~/.ssh/config"));
                }
            }
            "match" => block.clear(),
            "hostname" | "user" | "port" => {
                for &i in &block {
                    let detail = &mut out[i].detail;
                    if !detail.is_empty() {
                        detail.push_str(", ");
                    }
                    detail.push_str(&format!("{key} {value}"));
                }
            }
            _ => {}
        }
    }
    out
}

/// PuTTY stores session names URL-encoded ("My%20Server").
fn decode(name: &str) -> String {
    let bytes = name.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = (bytes[i] == b'%')
            .then(|| name.get(i + 1..i + 3))
            .flatten()
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match hex {
            Some(b) => {
                out.push(b);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn putty_sessions() -> Vec<Candidate> {
    win::reg_subkeys(PUTTY_SESSIONS)
        .into_iter()
        .filter(|key| key != "Default%20Settings")
        .filter_map(|key| {
            let path = format!("{PUTTY_SESSIONS}\\{key}");
            let text = |value: &str| win::reg_string(&path, value).unwrap_or_default();
            let number = |value: &str| win::reg_dword(&path, value);
            let (kind, host) = match text("Protocol").as_str() {
                "ssh" => (Kind::Ssh, text("HostName")),
                "telnet" => (Kind::Telnet, text("HostName")),
                "serial" => (Kind::Serial, text("SerialLine")),
                _ => return None,
            };
            if host.is_empty() {
                return None;
            }
            let mut c = Candidate::new(kind, &host, "PuTTY");
            c.name = decode(&key);
            c.user = text("UserName");
            let default = if kind == Kind::Ssh { 22 } else { 23 };
            c.port = number("PortNumber").and_then(|p| u16::try_from(p).ok()).filter(|&p| p != default);
            if kind == Kind::Serial {
                c.port = None;
                c.baud = number("SerialSpeed");
            }
            Some(c)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_config_hosts() {
        let text = "# Kommentar\nHost pi raspi\n  HostName 192.168.1.9\n  User pi\n\nHost *.firma.de\n  User me\n\
                    Host nas\n  Port=2222\nMatch host x\n  User y\n";
        let found = ssh_config(text);
        let summary: Vec<_> = found.iter().map(|c| (c.host.as_str(), c.detail.as_str())).collect();
        assert_eq!(
            summary,
            [
                ("pi", "hostname 192.168.1.9, user pi"),
                ("raspi", "hostname 192.168.1.9, user pi"),
                ("nas", "port 2222"),
            ]
        );
        assert!(found.iter().all(|c| c.kind == Kind::Ssh && c.port.is_none() && c.user.is_empty()));
    }

    #[test]
    fn putty_names() {
        assert_eq!(decode("My%20Server%2Fx"), "My Server/x");
        assert_eq!(decode("100%"), "100%");
    }

    #[test]
    fn rdp_files() {
        let dir = std::env::temp_dir().join("mrremotemanager-import-test");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Server Büro.rdp");
        std::fs::write(&file, "full address:s:srv:3390\r\nusername:s:me\r\n").unwrap();
        let found = import_rdp_files(vec![file.to_string_lossy().into_owned(), "C:\\x.txt".into()]);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].name.as_str(), found[0].host.as_str(), found[0].port), ("Server Büro", "srv", Some(3390)));
        assert_eq!(found[0].user, "me");
        let _ = std::fs::remove_dir_all(dir);
    }
}
