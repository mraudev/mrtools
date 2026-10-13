//! Govee lights over Govee's LAN API – no cloud. Devices answer a multicast scan on
//! 239.255.255.250:4001, take commands on port 4003 and send their answers to port 4002.
//! "LAN Control" has to be switched on per device in the Govee Home app, and only newer models
//! support it.

use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, State};

const MULTICAST: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SCAN_PORT: u16 = 4001;
const LISTEN_PORT: u16 = 4002;
const COMMAND_PORT: u16 = 4003;

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// MAC-like id, e.g. `1F:80:C5:32:32:36:72:4E`.
    id: String,
    /// Model, e.g. `H6076`.
    sku: String,
    ip: String,
    on: Option<bool>,
    /// 1–100.
    brightness: Option<u8>,
    rgb: Option<[u8; 3]>,
    /// White colour temperature, `None`/0 while in colour mode.
    kelvin: Option<u32>,
}

#[derive(Default)]
pub struct Govee {
    devices: Arc<Mutex<HashMap<String, Device>>>,
    /// Why listening on port 4002 failed, if it did.
    listen_error: Mutex<Option<String>>,
}

fn message(cmd: &str, data: Value) -> Vec<u8> {
    json!({ "msg": { "cmd": cmd, "data": data } }).to_string().into_bytes()
}

/// Applies an answer from a device: a scan result or a status report.
fn apply(devices: &mut HashMap<String, Device>, from: IpAddr, payload: &[u8]) -> Option<Device> {
    let v: Value = serde_json::from_slice(payload).ok()?;
    let data = &v["msg"]["data"];
    match v["msg"]["cmd"].as_str()? {
        "scan" => {
            let id = data["device"].as_str()?.to_owned();
            let device = devices.entry(id.clone()).or_insert_with(|| Device { id, ..Default::default() });
            device.sku = data["sku"].as_str().unwrap_or_default().to_owned();
            device.ip = data["ip"].as_str().map_or_else(|| from.to_string(), str::to_owned);
            Some(device.clone())
        }
        "devStatus" => {
            // Status reports carry no id – the sender's address tells which device it is.
            let device = devices.values_mut().find(|d| d.ip == from.to_string())?;
            device.on = data["onOff"].as_u64().map(|v| v == 1);
            device.brightness = data["brightness"].as_u64().map(|b| b.min(100) as u8);
            let c = &data["color"];
            device.rgb = Some([c["r"].as_u64()? as u8, c["g"].as_u64()? as u8, c["b"].as_u64()? as u8]);
            device.kelvin = data["colorTemInKelvin"].as_u64().filter(|&k| k > 0).map(|k| k as u32);
            Some(device.clone())
        }
        _ => None,
    }
}

/// Listens on port 4002 for the devices' answers and forwards them as `govee-update` events.
pub fn init(app: &AppHandle) {
    let govee = app.state::<Govee>();
    let socket = match UdpSocket::bind((Ipv4Addr::UNSPECIFIED, LISTEN_PORT)) {
        Ok(socket) => socket,
        Err(e) => {
            *govee.listen_error.lock().unwrap() = Some(format!("Port {LISTEN_PORT} ist belegt ({e}) – läuft ein anderes Govee-Programm?"));
            return;
        }
    };
    let devices = govee.devices.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 2048];
        while let Ok((len, from)) = socket.recv_from(&mut buf) {
            let updated = apply(&mut devices.lock().unwrap(), from.ip(), &buf[..len]);
            if let Some(device) = updated {
                let _ = app.emit("govee-update", device);
            }
        }
    });
}

/// IPv4 addresses of the network adapters – the scan goes out on each of them (Hyper-V, VPN, …).
fn local_ipv4() -> Vec<Ipv4Addr> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.ip() {
            IpAddr::V4(ip) if !ip.is_link_local() => Some(ip),
            _ => None,
        })
        .collect()
}

fn send_scan() {
    let payload = message("scan", json!({ "account_topic": "reserve" }));
    for ip in local_ipv4() {
        let Ok(socket) = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, None) else { continue };
        if socket.set_multicast_if_v4(&ip).is_err() || socket.bind(&SocketAddr::from((ip, 0)).into()).is_err() {
            continue;
        }
        let _ = socket.send_to(&payload, &SocketAddr::from((MULTICAST, SCAN_PORT)).into());
    }
}

fn send(ip: &str, payload: &[u8]) -> Result<(), String> {
    let target: IpAddr = ip.parse().map_err(|_| format!("Ungültige Adresse: {ip}"))?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(|e| e.to_string())?;
    socket.send_to(payload, (target, COMMAND_PORT)).map_err(|e| e.to_string())?;
    Ok(())
}

fn request_status(ip: &str) {
    let _ = send(ip, &message("devStatus", json!({})));
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    devices: Vec<Device>,
    /// Set when answers cannot be received at all.
    error: Option<String>,
}

fn snapshot(govee: &Govee) -> Scan {
    let mut devices: Vec<Device> = govee.devices.lock().unwrap().values().cloned().collect();
    devices.sort_by(|a, b| (&a.sku, &a.id).cmp(&(&b.sku, &b.id)));
    Scan { devices, error: govee.listen_error.lock().unwrap().clone() }
}

/// Looks for devices (about two seconds) and asks each for its state.
#[tauri::command]
pub async fn govee_scan(govee: State<'_, Govee>) -> Result<Scan, String> {
    tauri::async_runtime::spawn_blocking(send_scan).await.map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let ips: Vec<String> = govee.devices.lock().unwrap().values().map(|d| d.ip.clone()).collect();
    ips.iter().for_each(|ip| request_status(ip));
    tokio::time::sleep(Duration::from_millis(600)).await;
    Ok(snapshot(&govee))
}

/// What to change; missing fields stay as they are.
#[derive(serde::Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    on: Option<bool>,
    brightness: Option<u8>,
    rgb: Option<[u8; 3]>,
    kelvin: Option<u32>,
}

fn commands(change: &Change) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    if let Some(on) = change.on {
        out.push(message("turn", json!({ "value": u8::from(on) })));
    }
    if let Some(b) = change.brightness {
        out.push(message("brightness", json!({ "value": b.clamp(1, 100) })));
    }
    if let Some([r, g, b]) = change.rgb {
        out.push(message("colorwc", json!({ "color": { "r": r, "g": g, "b": b }, "colorTemInKelvin": 0 })));
    } else if let Some(k) = change.kelvin {
        out.push(message("colorwc", json!({ "color": { "r": 0, "g": 0, "b": 0 }, "colorTemInKelvin": k.clamp(2000, 9000) })));
    }
    out
}

/// Sends the change to the device at `ip`; its new state arrives as a `govee-update` event.
#[tauri::command]
pub async fn govee_set(ip: String, change: Change) -> Result<(), String> {
    for (i, payload) in commands(&change).iter().enumerate() {
        if i > 0 {
            // Some devices drop commands that arrive in the same instant.
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
        send(&ip, payload)?;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    request_status(&ip);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learns_devices_from_scan_answers_and_status() {
        let mut devices = HashMap::new();
        let ip: IpAddr = "192.168.1.50".parse().unwrap();
        let scan = br#"{"msg":{"cmd":"scan","data":{"ip":"192.168.1.50","device":"1F:80:C5:32","sku":"H6076","bleVersionHard":"3.01.01"}}}"#;
        let device = apply(&mut devices, ip, scan).unwrap();
        assert_eq!((device.id.as_str(), device.sku.as_str(), device.ip.as_str()), ("1F:80:C5:32", "H6076", "192.168.1.50"));

        let status = br#"{"msg":{"cmd":"devStatus","data":{"onOff":1,"brightness":80,"color":{"r":255,"g":120,"b":0},"colorTemInKelvin":0}}}"#;
        let device = apply(&mut devices, ip, status).unwrap();
        assert_eq!((device.on, device.brightness, device.rgb, device.kelvin), (Some(true), Some(80), Some([255, 120, 0]), None));

        // Status from an unknown address is ignored.
        assert!(apply(&mut devices, "10.0.0.9".parse().unwrap(), status).is_none());
        assert!(apply(&mut devices, ip, b"kaputt").is_none());
    }

    #[test]
    fn builds_commands() {
        let all = commands(&Change { on: Some(true), brightness: Some(0), rgb: None, kelvin: Some(12000) });
        let text: Vec<String> = all.iter().map(|c| String::from_utf8(c.clone()).unwrap()).collect();
        assert_eq!(text.len(), 3);
        assert!(text[0].contains(r#""cmd":"turn""#) && text[0].contains(r#""value":1"#));
        assert!(text[1].contains(r#""value":1"#), "brightness is at least 1");
        assert!(text[2].contains(r#""colorTemInKelvin":9000"#));
        assert!(commands(&Change::default()).is_empty());
    }
}

#[cfg(test)]
mod network {
    use super::*;

    /// `cargo test -p mrhome --lib govee::network -- --ignored --nocapture` – read-only scan.
    #[test]
    #[ignore]
    fn finds_devices() {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, LISTEN_PORT)).expect("Port 4002 frei");
        socket.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
        println!("Netzwerkkarten: {:?}", local_ipv4());
        send_scan();
        let mut devices = HashMap::new();
        let mut buf = [0u8; 2048];
        let until = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < until {
            if let Ok((len, from)) = socket.recv_from(&mut buf) {
                println!("Antwort von {from}: {}", String::from_utf8_lossy(&buf[..len]));
                apply(&mut devices, from.ip(), &buf[..len]);
            }
        }
        println!("Govee-Geräte: {:?}", devices.values().collect::<Vec<_>>());
    }
}
