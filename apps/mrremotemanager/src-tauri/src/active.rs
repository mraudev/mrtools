//! Remote connections open right now: Remotedesktop (`mstsc.exe`) and `ssh.exe` with the host
//! they were started with, plus any other program connected to port 3389 or 22 (PuTTY, WinSCP, …)
//! with the remote IP address.

use crate::launch::Kind;
use crate::win::{self, Tcp};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Active {
    kind: Kind,
    /// Host as the client was given it (name or IP); otherwise the remote IP.
    host: String,
    port: u16,
    user: String,
    key_file: String,
    /// Remote IP address of the connection.
    address: String,
    pid: u32,
    process: String,
}

/// Keeps sysinfo's process list between refreshes (cheaper than starting over).
#[derive(Default)]
pub struct Monitor {
    system: Mutex<Option<System>>,
}

#[tauri::command]
pub async fn active_connections(monitor: tauri::State<'_, Monitor>) -> Result<Vec<Active>, String> {
    let tcp = win::established();
    let mut pids: Vec<u32> = tcp.iter().map(|t| t.pid).collect();
    pids.sort_unstable();
    pids.dedup();

    let processes: HashMap<u32, (String, Vec<String>)> = {
        let mut guard = monitor.system.lock().unwrap();
        let system = guard.get_or_insert_with(System::new);
        let wanted: Vec<Pid> = pids.iter().map(|&p| Pid::from_u32(p)).collect();
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&wanted),
            true,
            ProcessRefreshKind::nothing().with_cmd(UpdateKind::OnlyIfNotSet),
        );
        pids.iter()
            .filter_map(|&pid| {
                let p = system.process(Pid::from_u32(pid))?;
                let args = p.cmd().iter().map(|a| a.to_string_lossy().into_owned()).collect();
                Some((pid, (p.name().to_string_lossy().into_owned(), args)))
            })
            .collect()
    };
    let titles = win::window_titles();
    Ok(detect(&tcp, &processes, &titles, win::rdp_username_hint))
}

fn detect(
    tcp: &[Tcp],
    processes: &HashMap<u32, (String, Vec<String>)>,
    titles: &HashMap<u32, Vec<String>>,
    user_hint: impl Fn(&str) -> Option<String>,
) -> Vec<Active> {
    let mut out = Vec::new();
    let mut done_pids = HashSet::new();
    let mut done_connections = HashSet::new();
    for t in tcp {
        let Some((name, args)) = processes.get(&t.pid) else { continue };
        let lower = name.to_ascii_lowercase();
        let base = |kind, s: &Tcp| Active {
            kind,
            host: s.remote_address.clone(),
            port: s.remote_port,
            user: String::new(),
            key_file: String::new(),
            address: s.remote_address.clone(),
            pid: s.pid,
            process: name.clone(),
        };
        if lower == "mstsc.exe" || lower == "ssh.exe" {
            if !done_pids.insert(t.pid) {
                continue;
            }
            let own: Vec<&Tcp> = tcp.iter().filter(|s| s.pid == t.pid).collect();
            if lower == "mstsc.exe" {
                // A gateway may add other connections; the session itself usually uses 3389.
                let s = own.iter().find(|s| s.remote_port == 3389).unwrap_or(&own[0]);
                let mut a = base(Kind::Rdp, s);
                if let Some(target) = mstsc_target(args) {
                    a.host = target.host;
                    a.port = target.port.unwrap_or(a.port);
                    a.user = target.user;
                } else if let Some(host) = titles.get(&t.pid).and_then(|ts| ts.iter().find_map(|t| rdp_title_host(t))) {
                    a.host = host;
                }
                if a.user.is_empty() {
                    a.user = user_hint(&a.host).unwrap_or_default();
                }
                out.push(a);
            } else {
                let mut a = base(Kind::Ssh, own[0]);
                if let Some(target) = ssh_target(args) {
                    a.host = target.host;
                    a.port = target.port.unwrap_or(a.port);
                    a.user = target.user;
                    a.key_file = target.key_file;
                }
                out.push(a);
            }
        } else if matches!(t.remote_port, 3389 | 22) && !matches!(t.local_port, 3389 | 22) {
            // Outgoing only: a remote session *into* this PC has the local port 3389/22.
            if done_connections.insert((t.pid, &t.remote_address, t.remote_port)) {
                out.push(base(if t.remote_port == 22 { Kind::Ssh } else { Kind::Rdp }, t));
            }
        }
    }
    out
}

#[derive(Debug, PartialEq, Default)]
struct Target {
    host: String,
    port: Option<u16>,
    user: String,
    key_file: String,
}

/// "host", "host:port", "[v6]:port" or a bare IPv6 address.
fn split_host_port(text: &str) -> (String, Option<u16>) {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix('[') {
        if let Some((host, after)) = rest.split_once(']') {
            return (host.into(), after.strip_prefix(':').and_then(|p| p.parse().ok()));
        }
    }
    match text.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => match port.parse() {
            Ok(port) => (host.into(), Some(port)),
            Err(_) => (text.into(), None),
        },
        _ => (text.into(), None),
    }
}

/// Host and user from `mstsc /v:host[:port]` or from the `.rdp` file it was started with.
fn mstsc_target(args: &[String]) -> Option<Target> {
    for arg in args.iter().skip(1) {
        if arg.len() > 3 && arg[..3].eq_ignore_ascii_case("/v:") {
            let (host, port) = split_host_port(&arg[3..]);
            return Some(Target { host, port, ..Default::default() });
        }
    }
    let file = args.iter().skip(1).find(|a| a.to_ascii_lowercase().ends_with(".rdp"))?;
    rdp_file_target(&std::fs::read(file).ok()?)
}

fn rdp_file_target(bytes: &[u8]) -> Option<Target> {
    let text = match bytes {
        [0xFF, 0xFE, rest @ ..] => {
            String::from_utf16_lossy(&rest.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect::<Vec<_>>())
        }
        _ => String::from_utf8_lossy(bytes).trim_start_matches('\u{feff}').to_string(),
    };
    let field = |name: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(name))
            .map(|v| v.trim().to_string())
    };
    let (host, port) = split_host_port(&field("full address:s:")?);
    let user = field("username:s:").unwrap_or_default();
    (!host.is_empty()).then_some(Target { host, port, user, ..Default::default() })
}

/// "server01 - Remotedesktopverbindung" → "server01".
fn rdp_title_host(title: &str) -> Option<String> {
    let (host, app) = title.rsplit_once(" - ")?;
    let host = host.trim();
    (app.contains("Remote") && !host.is_empty() && !host.contains(' ')).then(|| host.to_string())
}

/// Destination, port, user and key file from an ssh command line.
fn ssh_target(args: &[String]) -> Option<Target> {
    // Options of OpenSSH's ssh that take a value.
    const WITH_VALUE: &str = "BbcDEeFIiJLlmOoPpQRSWw";
    let mut target = Target::default();
    let mut user_option = String::new();
    let mut args = args.iter().skip(1);
    while let Some(arg) = args.next() {
        let Some(option) = arg.strip_prefix('-').filter(|o| !o.is_empty()) else {
            let destination = arg.strip_prefix("ssh://").unwrap_or(arg);
            let (user, host) = destination.rsplit_once('@').unwrap_or(("", destination));
            let (host, port) = split_host_port(host);
            target.host = host;
            target.port = target.port.or(port);
            target.user = if user.is_empty() { user_option } else { user.into() };
            return Some(target);
        };
        // Flags may be combined ("-vA"); the first option that takes a value ends the group.
        for (i, letter) in option.char_indices() {
            if WITH_VALUE.contains(letter) {
                let rest = &option[i + letter.len_utf8()..];
                let value = if rest.is_empty() { args.next().cloned().unwrap_or_default() } else { rest.to_string() };
                match letter {
                    'p' => target.port = value.parse().ok(),
                    'l' => user_option = value,
                    'i' => target.key_file = value,
                    _ => {}
                }
                break;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split(' ').map(String::from).collect()
    }

    fn tcp(pid: u32, remote: &str, remote_port: u16, local_port: u16) -> Tcp {
        Tcp { local_port, remote_address: remote.into(), remote_port, pid }
    }

    #[test]
    fn host_and_port() {
        assert_eq!(split_host_port("srv"), ("srv".into(), None));
        assert_eq!(split_host_port("srv:3390"), ("srv".into(), Some(3390)));
        assert_eq!(split_host_port("[fe80::1]:22"), ("fe80::1".into(), Some(22)));
        assert_eq!(split_host_port("fe80::1"), ("fe80::1".into(), None));
    }

    #[test]
    fn ssh_command_lines() {
        let t = ssh_target(&args("ssh.exe -p 2222 -i C:\\k\\id -l pi pi.local")).unwrap();
        assert_eq!(t, Target { host: "pi.local".into(), port: Some(2222), user: "pi".into(), key_file: "C:\\k\\id".into() });
        let t = ssh_target(&args("ssh -vA -p2200 admin@10.0.0.1 uptime")).unwrap();
        assert_eq!((t.host.as_str(), t.port, t.user.as_str()), ("10.0.0.1", Some(2200), "admin"));
        let t = ssh_target(&args("ssh -o ServerAliveInterval=30 ssh://me@[::1]:2022")).unwrap();
        assert_eq!((t.host.as_str(), t.port, t.user.as_str()), ("::1", Some(2022), "me"));
        assert_eq!(ssh_target(&args("ssh -V")), None);
    }

    #[test]
    fn mstsc_command_line_and_rdp_file() {
        let t = mstsc_target(&args("mstsc.exe /f /v:server01:3390")).unwrap();
        assert_eq!((t.host.as_str(), t.port), ("server01", Some(3390)));
        assert_eq!(mstsc_target(&args("mstsc.exe")), None);

        let text = "screen mode id:i:1\r\nfull address:s:srv.firma.de\r\nusername:s:FIRMA\\me\r\n";
        let utf16: Vec<u8> = [0xFFu8, 0xFE].into_iter().chain(text.encode_utf16().flat_map(u16::to_le_bytes)).collect();
        let t = rdp_file_target(&utf16).unwrap();
        assert_eq!((t.host.as_str(), t.port, t.user.as_str()), ("srv.firma.de", None, "FIRMA\\me"));
        assert_eq!(rdp_file_target(text.as_bytes()).unwrap().host, "srv.firma.de");
    }

    #[test]
    fn rdp_window_titles() {
        assert_eq!(rdp_title_host("server01 - Remotedesktopverbindung"), Some("server01".into()));
        assert_eq!(rdp_title_host("10.0.0.5 - Remote Desktop Connection"), Some("10.0.0.5".into()));
        assert_eq!(rdp_title_host("Remotedesktopverbindung"), None);
        assert_eq!(rdp_title_host("Bericht - Word"), None);
    }

    #[test]
    fn detects_clients_and_skips_incoming_sessions() {
        let processes = HashMap::from([
            (10, ("mstsc.exe".to_string(), args("mstsc.exe"))),
            (20, ("ssh.exe".to_string(), args("ssh.exe pi@pi.local"))),
            (30, ("putty.exe".to_string(), args("putty.exe"))),
            (40, ("svchost.exe".to_string(), args("svchost.exe"))),
            (50, ("chrome.exe".to_string(), args("chrome.exe"))),
        ]);
        let tcp = [
            tcp(10, "10.0.0.5", 443, 50000),
            tcp(10, "10.0.0.5", 3389, 50001),
            tcp(20, "192.168.1.9", 22, 50002),
            tcp(30, "192.168.1.7", 22, 50003),
            tcp(30, "192.168.1.7", 22, 50004),
            tcp(40, "192.168.1.50", 50100, 3389),
            tcp(50, "1.2.3.4", 443, 50005),
        ];
        let titles = HashMap::from([(10, vec!["server01 - Remotedesktopverbindung".to_string()])]);
        let found = detect(&tcp, &processes, &titles, |host| (host == "server01").then(|| "me".into()));
        let summary: Vec<_> = found
            .iter()
            .map(|a| (a.kind, a.host.as_str(), a.port, a.user.as_str(), a.process.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                (Kind::Rdp, "server01", 3389, "me", "mstsc.exe"),
                (Kind::Ssh, "pi.local", 22, "pi", "ssh.exe"),
                (Kind::Ssh, "192.168.1.7", 22, "", "putty.exe"),
            ]
        );
        assert_eq!(found[0].address, "10.0.0.5");
    }
}
