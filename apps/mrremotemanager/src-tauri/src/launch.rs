//! Opening a connection: Remotedesktop via `mstsc.exe` and an `.rdp` file, SSH via `ssh.exe`
//! in a new console window (Windows Terminal, if it is the default terminal), web addresses
//! in the default browser.
//!
//! Every value comes from the user's own list, but it is still validated: nothing may break
//! out of an `.rdp` line or turn into an ssh option.

use serde::{Deserialize, Serialize};
use std::{fs, io::ErrorKind, path::Path, process::Command};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Rdp,
    Ssh,
    Web,
}

/// The fields of a saved connection (`Connection` in `src/lib/types.ts`) needed to open it.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    id: String,
    kind: Kind,
    host: String,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    user: String,
    #[serde(default)]
    fullscreen: bool,
    #[serde(default)]
    multimon: bool,
    #[serde(default)]
    key_file: String,
}

#[tauri::command]
pub fn connect(app: AppHandle, connection: Connection) -> Result<(), String> {
    match connection.kind {
        Kind::Rdp => {
            let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?.join("rdp");
            let file = write_rdp_file(&dir, &connection)?;
            spawn("mstsc.exe", &[file.into_os_string()], "Remotedesktop (mstsc.exe)")
        }
        Kind::Ssh => {
            let args = ssh_args(&connection)?;
            spawn("ssh.exe", &args, "ssh.exe – OpenSSH-Client unter „Optionale Features“ installieren")
        }
        Kind::Web => app
            .opener()
            .open_url(web_url(&connection.host)?, None::<&str>)
            .map_err(|e| e.to_string()),
    }
}

fn spawn<S: AsRef<std::ffi::OsStr>>(program: &str, args: &[S], what: &str) -> Result<(), String> {
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        command.creation_flags(CREATE_NEW_CONSOLE);
    }
    match command.spawn() {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Err(format!("Nicht gefunden: {what}")),
        Err(e) => Err(format!("{program}: {e}")),
    }
}

/// Host name, IPv4 or IPv6 address (optionally in brackets); never starts with `-`.
fn check_host(host: &str) -> Result<&str, String> {
    let host = host.trim();
    let valid = !host.is_empty()
        && !host.starts_with('-')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-_:[]%".contains(c));
    if valid {
        Ok(host)
    } else {
        Err(format!("Ungültiger Host: „{host}“"))
    }
}

fn check_text<'a>(value: &'a str, what: &str) -> Result<&'a str, String> {
    if value.chars().any(char::is_control) {
        Err(format!("{what} enthält ungültige Zeichen."))
    } else {
        Ok(value.trim())
    }
}

fn strip_brackets(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host)
}

fn rdp_file(c: &Connection) -> Result<String, String> {
    let host = strip_brackets(check_host(&c.host)?);
    let address = match c.port {
        Some(port) if port != 3389 && host.contains(':') => format!("[{host}]:{port}"),
        Some(port) if port != 3389 => format!("{host}:{port}"),
        _ => host.to_string(),
    };
    let mut lines = vec![
        format!("full address:s:{address}"),
        format!("screen mode id:i:{}", if c.fullscreen { 2 } else { 1 }),
        format!("use multimon:i:{}", u8::from(c.multimon)),
    ];
    let user = check_text(&c.user, "Der Benutzername")?;
    if !user.is_empty() {
        lines.push(format!("username:s:{user}"));
    }
    Ok(lines.join("\r\n") + "\r\n")
}

/// Writes `<dir>/<id>.rdp` (UTF-16 with BOM, like the files mstsc saves itself).
fn write_rdp_file(dir: &Path, c: &Connection) -> Result<std::path::PathBuf, String> {
    if c.id.is_empty() || !c.id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        return Err("Ungültige Verbindungs-ID.".into());
    }
    let text = rdp_file(c)?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("{}.rdp", c.id));
    let bytes: Vec<u8> = std::iter::once(0xFEFF)
        .chain(text.encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect();
    fs::write(&file, bytes).map_err(|e| e.to_string())?;
    Ok(file)
}

fn ssh_args(c: &Connection) -> Result<Vec<String>, String> {
    let host = strip_brackets(check_host(&c.host)?);
    let mut args = Vec::new();
    if let Some(port) = c.port.filter(|&p| p != 22) {
        args.extend(["-p".into(), port.to_string()]);
    }
    let key = check_text(&c.key_file, "Die Schlüsseldatei")?;
    if !key.is_empty() {
        args.extend(["-i".into(), key.into()]);
    }
    let user = check_text(&c.user, "Der Benutzername")?;
    if !user.is_empty() {
        args.extend(["-l".into(), user.into()]);
    }
    // The validated host never starts with "-", so it cannot be read as an option.
    args.push(host.into());
    Ok(args)
}

/// Only http(s); an address without scheme gets `https://`.
fn web_url(address: &str) -> Result<String, String> {
    let address = address.trim();
    if address.is_empty() || address.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(format!("Ungültige Adresse: „{address}“"));
    }
    let lower = address.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        Ok(address.into())
    } else if lower.contains("://") {
        Err("Nur http:// und https:// werden geöffnet.".into())
    } else {
        Ok(format!("https://{address}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection(kind: Kind, host: &str) -> Connection {
        Connection {
            id: "abc-1".into(),
            kind,
            host: host.into(),
            port: None,
            user: String::new(),
            fullscreen: false,
            multimon: false,
            key_file: String::new(),
        }
    }

    #[test]
    fn rdp_file_has_address_user_and_screen() {
        let mut c = connection(Kind::Rdp, " server.local ");
        c.user = "FIRMA\\michael".into();
        c.fullscreen = true;
        assert_eq!(
            rdp_file(&c).unwrap(),
            "full address:s:server.local\r\nscreen mode id:i:2\r\nuse multimon:i:0\r\nusername:s:FIRMA\\michael\r\n"
        );
    }

    #[test]
    fn rdp_port_only_when_not_default() {
        let mut c = connection(Kind::Rdp, "10.0.0.5");
        c.port = Some(3389);
        assert!(rdp_file(&c).unwrap().starts_with("full address:s:10.0.0.5\r\n"));
        c.port = Some(3390);
        assert!(rdp_file(&c).unwrap().starts_with("full address:s:10.0.0.5:3390\r\n"));
        c.host = "[fe80::1]".into();
        assert!(rdp_file(&c).unwrap().starts_with("full address:s:[fe80::1]:3390\r\n"));
    }

    #[test]
    fn rdp_rejects_line_breaks() {
        let mut c = connection(Kind::Rdp, "server");
        c.user = "a\r\nalternate shell:s:cmd".into();
        assert!(rdp_file(&c).is_err());
        assert!(rdp_file(&connection(Kind::Rdp, "server\nx")).is_err());
    }

    #[test]
    fn rdp_file_is_utf16_with_bom() {
        let dir = std::env::temp_dir().join("mrremotemanager-test");
        let file = write_rdp_file(&dir, &connection(Kind::Rdp, "server")).unwrap();
        let bytes = fs::read(&file).unwrap();
        assert_eq!(&bytes[..4], &[0xFF, 0xFE, b'f', 0]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rdp_file_name_must_be_safe() {
        let mut c = connection(Kind::Rdp, "server");
        c.id = "..\\evil".into();
        assert!(write_rdp_file(&std::env::temp_dir(), &c).is_err());
    }

    #[test]
    fn ssh_arguments() {
        let mut c = connection(Kind::Ssh, "pi.local");
        assert_eq!(ssh_args(&c).unwrap(), ["pi.local"]);
        c.port = Some(2222);
        c.user = "pi".into();
        c.key_file = "C:\\Users\\me\\.ssh\\id ed25519".into();
        assert_eq!(
            ssh_args(&c).unwrap(),
            ["-p", "2222", "-i", "C:\\Users\\me\\.ssh\\id ed25519", "-l", "pi", "pi.local"]
        );
    }

    #[test]
    fn ssh_host_cannot_be_an_option() {
        assert!(ssh_args(&connection(Kind::Ssh, "-oProxyCommand=calc")).is_err());
        assert!(ssh_args(&connection(Kind::Ssh, "host name")).is_err());
        assert_eq!(ssh_args(&connection(Kind::Ssh, "[::1]")).unwrap(), ["::1"]);
    }

    #[test]
    fn web_urls() {
        assert_eq!(web_url("nas.local:5001").unwrap(), "https://nas.local:5001");
        assert_eq!(web_url(" http://router/ ").unwrap(), "http://router/");
        assert!(web_url("file:///C:/Windows").is_err());
        assert!(web_url("a b").is_err());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let c: Connection = serde_json::from_str(
            r#"{"id":"x","kind":"ssh","host":"h","name":"Name","notes":"","port":null,"lastUsed":1}"#,
        )
        .unwrap();
        assert_eq!(c.kind, Kind::Ssh);
        assert_eq!(c.port, None);
    }
}
