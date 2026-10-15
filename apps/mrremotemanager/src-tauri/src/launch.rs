//! Opening a connection:
//! - Remotedesktop via `mstsc.exe` and an `.rdp` file
//! - SSH via `ssh.exe` in a new console window (Windows Terminal, if it is the default terminal)
//! - web addresses in the default browser
//! - VNC in an installed viewer (TigerVNC, RealVNC, UltraVNC, TightVNC)
//! - Telnet and serial lines in PuTTY
//! - Hyper-V machines in `vmconnect.exe` (via the shell, so it can ask for admin rights)
//!
//! Every value comes from the user's own list, but it is still validated: nothing may break
//! out of an `.rdp` line, turn into a command line option or escape a quoted argument.

use crate::win;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Rdp,
    Ssh,
    Web,
    Vnc,
    Telnet,
    Serial,
    Hyperv,
}

fn yes() -> bool {
    true
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
    // Remotedesktop
    #[serde(default)]
    fullscreen: bool,
    #[serde(default)]
    multimon: bool,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default = "yes")]
    clipboard: bool,
    #[serde(default)]
    drives: bool,
    #[serde(default)]
    gateway: String,
    #[serde(default)]
    admin: bool,
    // SSH
    #[serde(default)]
    key_file: String,
    #[serde(default)]
    jump_host: String,
    /// One forwarding per line: "L 8080:localhost:80", "R …" or "D 1080".
    #[serde(default)]
    forwards: String,
    // Serial line
    #[serde(default)]
    baud: Option<u32>,
    // Hyper-V
    #[serde(default)]
    vm_name: String,
}

#[tauri::command]
pub fn connect(app: AppHandle, connection: Connection) -> Result<(), String> {
    let c = &connection;
    match c.kind {
        Kind::Rdp => {
            let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?.join("rdp");
            let mut args = vec![write_rdp_file(&dir, c)?.into_os_string()];
            if c.admin {
                args.push("/admin".into());
            }
            spawn(Path::new("mstsc.exe"), &args, "Remotedesktop (mstsc.exe)")
        }
        Kind::Ssh => spawn(
            Path::new("ssh.exe"),
            &ssh_args(c)?,
            "ssh.exe – OpenSSH-Client unter „Optionale Features“ installieren",
        ),
        Kind::Web => app.opener().open_url(web_url(&c.host)?, None::<&str>).map_err(|e| e.to_string()),
        Kind::Vnc => {
            let viewer = find_vnc().ok_or("Kein VNC-Viewer gefunden (TigerVNC, RealVNC, UltraVNC oder TightVNC).")?;
            spawn(&viewer, &[vnc_address(c)?], "VNC-Viewer")
        }
        Kind::Telnet | Kind::Serial => {
            let putty = find_putty().ok_or("PuTTY nicht gefunden – bitte installieren.")?;
            spawn(&putty, &putty_args(c)?, "PuTTY")
        }
        Kind::Hyperv => win::shell_open("vmconnect.exe", &vmconnect_args(c)?),
    }
}

/// Opens the SSH server in WinSCP (SFTP).
#[tauri::command]
pub fn open_sftp(connection: Connection) -> Result<(), String> {
    let winscp = find_winscp().ok_or("WinSCP nicht gefunden – bitte installieren.")?;
    spawn(&winscp, &[sftp_url(&connection)?], "WinSCP")
}

/// Which optional programs are installed.
#[derive(Serialize)]
pub struct Tools {
    vnc: bool,
    putty: bool,
    winscp: bool,
}

#[tauri::command]
pub fn tools() -> Tools {
    Tools {
        vnc: find_vnc().is_some(),
        putty: find_putty().is_some(),
        winscp: find_winscp().is_some(),
    }
}

fn spawn<S: AsRef<std::ffi::OsStr>>(program: &Path, args: &[S], what: &str) -> Result<(), String> {
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        // Console programs (ssh) get their own window; GUI programs ignore the flag.
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        command.creation_flags(CREATE_NEW_CONSOLE);
    }
    match command.spawn() {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Err(format!("Nicht gefunden: {what}")),
        Err(e) => Err(format!("{}: {e}", program.display())),
    }
}

/// The first program that exists: below Program Files or `%LOCALAPPDATA%\Programs`, or on PATH.
fn find_program(relative: &[&str], names: &[&str]) -> Option<PathBuf> {
    let roots = ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"]
        .iter()
        .filter_map(|v| env::var_os(v).map(PathBuf::from))
        .chain(env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Programs")));
    let installed = roots.flat_map(|root| relative.iter().map(move |r| root.join(r)));
    let on_path = env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .flat_map(|dir| names.iter().map(move |n| dir.join(n)));
    installed.chain(on_path).find(|p| p.is_file())
}

fn find_vnc() -> Option<PathBuf> {
    find_program(
        &[
            "TigerVNC\\vncviewer.exe",
            "RealVNC\\VNC Viewer\\vncviewer.exe",
            "uvnc bvba\\UltraVNC\\vncviewer.exe",
            "UltraVNC\\vncviewer.exe",
            "TightVNC\\tvnviewer.exe",
        ],
        &["vncviewer.exe", "tvnviewer.exe"],
    )
}

fn find_putty() -> Option<PathBuf> {
    find_program(&["PuTTY\\putty.exe"], &["putty.exe"])
}

fn find_winscp() -> Option<PathBuf> {
    find_program(&["WinSCP\\WinSCP.exe"], &["WinSCP.exe"])
}

/// Host name, IPv4 or IPv6 address (optionally in brackets); never starts with `-`.
pub fn check_host(host: &str) -> Result<&str, String> {
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

/// A value that becomes a separate argument: must not look like an option.
fn check_argument<'a>(value: &'a str, what: &str, allowed: &str) -> Result<&'a str, String> {
    let value = value.trim();
    let valid = !value.starts_with('-')
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || allowed.contains(c));
    if valid {
        Ok(value)
    } else {
        Err(format!("{what} enthält ungültige Zeichen: „{value}“"))
    }
}

pub fn strip_brackets(host: &str) -> &str {
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
        format!("redirectclipboard:i:{}", u8::from(c.clipboard)),
    ];
    if let (false, Some(width), Some(height)) = (c.fullscreen, c.width, c.height) {
        if !(200..=8192).contains(&width) || !(200..=8192).contains(&height) {
            return Err("Die Fenstergröße muss zwischen 200 und 8192 Pixeln liegen.".into());
        }
        lines.push(format!("desktopwidth:i:{width}"));
        lines.push(format!("desktopheight:i:{height}"));
    }
    if c.drives {
        lines.push("drivestoredirect:s:*".into());
    }
    let gateway = c.gateway.trim();
    if !gateway.is_empty() {
        let gateway = check_host(gateway)?;
        lines.push(format!("gatewayhostname:s:{gateway}"));
        lines.push("gatewayusagemethod:i:1".into());
        lines.push("gatewayprofileusagemethod:i:1".into());
    }
    let user = check_text(&c.user, "Der Benutzername")?;
    if !user.is_empty() {
        lines.push(format!("username:s:{user}"));
    }
    Ok(lines.join("\r\n") + "\r\n")
}

/// Writes `<dir>/<id>.rdp` (UTF-16 with BOM, like the files mstsc saves itself).
fn write_rdp_file(dir: &Path, c: &Connection) -> Result<PathBuf, String> {
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
    let jump = check_argument(&c.jump_host, "Der Jump-Host", ".-_:@[],%")?;
    if !jump.is_empty() {
        args.extend(["-J".into(), jump.into()]);
    }
    for line in c.forwards.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let line = line.strip_prefix('-').unwrap_or(line);
        let mut chars = line.chars();
        let flag = chars.next().map(|f| f.to_ascii_uppercase());
        let spec = check_argument(chars.as_str(), "Die Weiterleitung", ".-_:[]/*")?;
        match flag {
            Some(flag @ ('L' | 'R' | 'D')) if !spec.is_empty() => args.extend([format!("-{flag}"), spec.into()]),
            _ => return Err(format!("Ungültige Weiterleitung: „{line}“ (Beispiel: L 8080:localhost:80)")),
        }
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

/// "host::port" – understood by all common viewers.
fn vnc_address(c: &Connection) -> Result<String, String> {
    let host = strip_brackets(check_host(&c.host)?);
    Ok(format!("{host}::{}", c.port.unwrap_or(5900)))
}

fn putty_args(c: &Connection) -> Result<Vec<String>, String> {
    if c.kind == Kind::Serial {
        let line = c.host.trim().to_ascii_uppercase();
        if !(line.len() > 3 && line.starts_with("COM") && line[3..].chars().all(|ch| ch.is_ascii_digit())) {
            return Err(format!("Ungültige Schnittstelle: „{line}“ (Beispiel: COM3)"));
        }
        let baud = c.baud.unwrap_or(9600);
        if !(50..=4_000_000).contains(&baud) {
            return Err("Ungültige Baudrate.".into());
        }
        return Ok(vec!["-serial".into(), line, "-sercfg".into(), format!("{baud},8,n,1,N")]);
    }
    let host = strip_brackets(check_host(&c.host)?);
    let mut args = vec!["-telnet".into()];
    let user = check_argument(&c.user, "Der Benutzername", ".-_@\\")?;
    if !user.is_empty() {
        args.extend(["-l".into(), user.into()]);
    }
    args.extend(["-P".into(), c.port.unwrap_or(23).to_string(), host.into()]);
    Ok(args)
}

/// `"server" "vm"` for vmconnect; the server defaults to this PC.
fn vmconnect_args(c: &Connection) -> Result<String, String> {
    let server = if c.host.trim().is_empty() { "localhost" } else { check_host(&c.host)? };
    let vm = check_text(&c.vm_name, "Der VM-Name")?;
    // A quote or backslash could end the quoted argument early.
    if vm.is_empty() || vm.contains(['"', '\\']) {
        return Err("Bitte einen gültigen VM-Namen eingeben (ohne \" und \\).".into());
    }
    Ok(format!("\"{server}\" \"{vm}\""))
}

/// `sftp://user@host:port/` for WinSCP.
fn sftp_url(c: &Connection) -> Result<String, String> {
    let host = check_host(&c.host)?;
    let host = if strip_brackets(host).contains(':') { format!("[{}]", strip_brackets(host)) } else { host.into() };
    let user = check_argument(&c.user, "Der Benutzername", ".-_")?;
    let user = if user.is_empty() { String::new() } else { format!("{user}@") };
    Ok(format!("sftp://{user}{host}:{}/", c.port.unwrap_or(22)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn connection(kind: &str, host: &str) -> Connection {
        serde_json::from_value(json!({ "id": "abc-1", "kind": kind, "host": host })).unwrap()
    }

    #[test]
    fn rdp_file_has_address_user_and_screen() {
        let mut c = connection("rdp", " server.local ");
        c.user = "FIRMA\\michael".into();
        c.fullscreen = true;
        assert_eq!(
            rdp_file(&c).unwrap(),
            "full address:s:server.local\r\nscreen mode id:i:2\r\nuse multimon:i:0\r\nredirectclipboard:i:1\r\nusername:s:FIRMA\\michael\r\n"
        );
    }

    #[test]
    fn rdp_options() {
        let mut c = connection("rdp", "srv");
        c.width = Some(1600);
        c.height = Some(900);
        c.clipboard = false;
        c.drives = true;
        c.gateway = "gw.firma.de".into();
        let text = rdp_file(&c).unwrap();
        for line in [
            "redirectclipboard:i:0",
            "desktopwidth:i:1600",
            "desktopheight:i:900",
            "drivestoredirect:s:*",
            "gatewayhostname:s:gw.firma.de",
        ] {
            assert!(text.contains(line), "{line} fehlt in {text}");
        }
        c.fullscreen = true;
        assert!(!rdp_file(&c).unwrap().contains("desktopwidth"));
        c.gateway = "gw\r\nalternate shell:s:cmd".into();
        assert!(rdp_file(&c).is_err());
    }

    #[test]
    fn rdp_port_only_when_not_default() {
        let mut c = connection("rdp", "10.0.0.5");
        c.port = Some(3389);
        assert!(rdp_file(&c).unwrap().starts_with("full address:s:10.0.0.5\r\n"));
        c.port = Some(3390);
        assert!(rdp_file(&c).unwrap().starts_with("full address:s:10.0.0.5:3390\r\n"));
        c.host = "[fe80::1]".into();
        assert!(rdp_file(&c).unwrap().starts_with("full address:s:[fe80::1]:3390\r\n"));
    }

    #[test]
    fn rdp_rejects_line_breaks() {
        let mut c = connection("rdp", "server");
        c.user = "a\r\nalternate shell:s:cmd".into();
        assert!(rdp_file(&c).is_err());
        assert!(rdp_file(&connection("rdp", "server\nx")).is_err());
    }

    #[test]
    fn rdp_file_is_utf16_with_bom() {
        let dir = env::temp_dir().join("mrremotemanager-test");
        let file = write_rdp_file(&dir, &connection("rdp", "server")).unwrap();
        let bytes = fs::read(&file).unwrap();
        assert_eq!(&bytes[..4], &[0xFF, 0xFE, b'f', 0]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rdp_file_name_must_be_safe() {
        let mut c = connection("rdp", "server");
        c.id = "..\\evil".into();
        assert!(write_rdp_file(&env::temp_dir(), &c).is_err());
    }

    #[test]
    fn ssh_arguments() {
        let mut c = connection("ssh", "pi.local");
        assert_eq!(ssh_args(&c).unwrap(), ["pi.local"]);
        c.port = Some(2222);
        c.user = "pi".into();
        c.key_file = "C:\\Users\\me\\.ssh\\id ed25519".into();
        c.jump_host = "admin@bastion:2200".into();
        c.forwards = "L 8080:localhost:80\n\n-D 1080\nr 9000:127.0.0.1:9000".into();
        assert_eq!(
            ssh_args(&c).unwrap(),
            [
                "-p", "2222", "-i", "C:\\Users\\me\\.ssh\\id ed25519", "-l", "pi", "-J", "admin@bastion:2200", "-L",
                "8080:localhost:80", "-D", "1080", "-R", "9000:127.0.0.1:9000", "pi.local"
            ]
        );
    }

    #[test]
    fn ssh_values_cannot_be_options() {
        assert!(ssh_args(&connection("ssh", "-oProxyCommand=calc")).is_err());
        assert!(ssh_args(&connection("ssh", "host name")).is_err());
        assert_eq!(ssh_args(&connection("ssh", "[::1]")).unwrap(), ["::1"]);
        let mut c = connection("ssh", "h");
        c.jump_host = "-oProxyCommand=calc".into();
        assert!(ssh_args(&c).is_err());
        c.jump_host = String::new();
        c.forwards = "L -oProxyCommand=calc".into();
        assert!(ssh_args(&c).is_err());
        c.forwards = "X 1:2:3".into();
        assert!(ssh_args(&c).is_err());
    }

    #[test]
    fn web_urls() {
        assert_eq!(web_url("nas.local:5001").unwrap(), "https://nas.local:5001");
        assert_eq!(web_url(" http://router/ ").unwrap(), "http://router/");
        assert!(web_url("file:///C:/Windows").is_err());
        assert!(web_url("a b").is_err());
    }

    #[test]
    fn vnc_and_putty() {
        let mut c = connection("vnc", "nas");
        assert_eq!(vnc_address(&c).unwrap(), "nas::5900");
        c.port = Some(5901);
        assert_eq!(vnc_address(&c).unwrap(), "nas::5901");

        let mut c = connection("telnet", "switch");
        c.user = "admin".into();
        assert_eq!(putty_args(&c).unwrap(), ["-telnet", "-l", "admin", "-P", "23", "switch"]);
        c.user = "-sercfg".into();
        assert!(putty_args(&c).is_err());

        let mut c = connection("serial", "com3");
        assert_eq!(putty_args(&c).unwrap(), ["-serial", "COM3", "-sercfg", "9600,8,n,1,N"]);
        c.baud = Some(115200);
        assert_eq!(putty_args(&c).unwrap()[3], "115200,8,n,1,N");
        c.host = "COM3 -load x".into();
        assert!(putty_args(&c).is_err());
    }

    #[test]
    fn vmconnect_quotes_safely() {
        let mut c = connection("hyperv", "");
        c.vm_name = "Windows 11 Dev".into();
        assert_eq!(vmconnect_args(&c).unwrap(), "\"localhost\" \"Windows 11 Dev\"");
        c.vm_name = "x\" /evil".into();
        assert!(vmconnect_args(&c).is_err());
        c.vm_name = "x\\".into();
        assert!(vmconnect_args(&c).is_err());
    }

    #[test]
    fn sftp_urls() {
        let mut c = connection("ssh", "pi.local");
        assert_eq!(sftp_url(&c).unwrap(), "sftp://pi.local:22/");
        c.user = "pi".into();
        c.port = Some(2222);
        assert_eq!(sftp_url(&c).unwrap(), "sftp://pi@pi.local:2222/");
        c.host = "fe80::1".into();
        assert_eq!(sftp_url(&c).unwrap(), "sftp://pi@[fe80::1]:2222/");
        c.user = "a@b".into();
        assert!(sftp_url(&c).is_err());
    }

    #[test]
    fn unknown_fields_are_ignored_and_defaults_apply() {
        let c: Connection = serde_json::from_str(
            r#"{"id":"x","kind":"ssh","host":"h","name":"Name","notes":"","port":null,"lastUsed":1}"#,
        )
        .unwrap();
        assert_eq!(c.kind, Kind::Ssh);
        assert_eq!(c.port, None);
        assert!(c.clipboard);
    }
}
