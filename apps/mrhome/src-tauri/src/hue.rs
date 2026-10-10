//! Philips Hue over the bridge's local API (CLIP v2) – no cloud. The bridge has a self-signed
//! certificate; its fingerprint is remembered when pairing and every later connection must present
//! the same one, so nobody on the network can pose as the bridge and read the app key.

use crate::secrets::{self, Secret};
use reqwest::Method;
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::CryptoProvider,
    pki_types::{CertificateDer, ServerName, UnixTime},
    DigitallySignedStruct, SignatureScheme,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    net::IpAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, State};

// ---------------------------------------------------------------------------
// Certificate pinning

type Fingerprint = [u8; 32];

#[derive(Debug)]
struct PinnedVerifier {
    /// `None` while pairing: any certificate is accepted and remembered in `seen`.
    expected: Option<Fingerprint>,
    seen: Arc<Mutex<Option<Fingerprint>>>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinnedVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let fingerprint: Fingerprint = Sha256::digest(end_entity.as_ref()).into();
        *self.seen.lock().unwrap() = Some(fingerprint);
        match self.expected {
            Some(expected) if expected != fingerprint => Err(rustls::Error::General(
                "Das Zertifikat der Hue Bridge passt nicht zur gekoppelten Bridge".into(),
            )),
            _ => Ok(ServerCertVerified::assertion()),
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

fn client(
    expected: Option<Fingerprint>,
    seen: Arc<Mutex<Option<Fingerprint>>>,
    timeout: Option<Duration>,
) -> Result<reqwest::Client, String> {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let verifier = PinnedVerifier { expected, seen, provider: provider.clone() };
    let tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    let mut builder = reqwest::Client::builder().tls_backend_preconfigured(tls).connect_timeout(Duration::from_secs(5));
    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout);
    }
    builder.build().map_err(|e| e.to_string())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<Fingerprint> {
    let bytes: Vec<u8> = (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect::<Option<_>>()?;
    bytes.try_into().ok()
}

// ---------------------------------------------------------------------------
// Paired bridge

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bridge {
    /// e.g. `001788fffe123456` – stays the same when the IP changes.
    id: String,
    ip: String,
    /// SHA-256 of the bridge certificate, hex.
    fingerprint: String,
}

#[derive(Default)]
pub struct Hue {
    bridge: Mutex<Option<Bridge>>,
    http: Mutex<Option<reqwest::Client>>,
    /// Bumped when pairing changes – a running event stream for an older pairing stops.
    generation: Mutex<u64>,
}

fn bridge_file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("hue-bridge.json"))
}

fn save_bridge(app: &AppHandle, bridge: Option<&Bridge>) -> Result<(), String> {
    let file = bridge_file(app).ok_or("Kein Konfigurationsordner")?;
    match bridge {
        Some(b) => {
            std::fs::create_dir_all(file.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(&file, serde_json::to_string_pretty(b).unwrap()).map_err(|e| e.to_string())
        }
        None => match std::fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        },
    }
}

impl Hue {
    fn connection(&self) -> Result<(Bridge, String, reqwest::Client), String> {
        let bridge = self.bridge.lock().unwrap().clone().ok_or("Keine Hue Bridge gekoppelt")?;
        let key = secrets::get(Secret::HueKey).ok_or("Keine Hue Bridge gekoppelt")?;
        let mut http = self.http.lock().unwrap();
        if http.is_none() {
            let expected = unhex(&bridge.fingerprint).ok_or("Ungültiger Zertifikats-Fingerabdruck")?;
            *http = Some(client(Some(expected), Default::default(), Some(Duration::from_secs(10)))?);
        }
        Ok((bridge, key, http.clone().unwrap()))
    }

    async fn request(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value, String> {
        let (bridge, key, http) = self.connection()?;
        let mut builder = http.request(method, format!("https://{}{path}", bridge.ip)).header("hue-application-key", key);
        if let Some(body) = body {
            builder = builder.json(&body);
        }
        let response = builder.send().await.map_err(|e| format!("Hue Bridge nicht erreichbar ({}): {e}", bridge.ip))?;
        let status = response.status();
        let value: Value = response.json().await.unwrap_or(Value::Null);
        if let Some(error) = value["errors"].as_array().and_then(|e| e.first()) {
            return Err(format!("Hue: {}", error["description"].as_str().unwrap_or("Fehler")));
        }
        if !status.is_success() {
            return Err(format!("Hue Bridge antwortet mit {status}"));
        }
        Ok(value)
    }
}

/// Loads the paired bridge and starts the event stream. Called once at startup.
pub fn init(app: &AppHandle) {
    let bridge = bridge_file(app)
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| serde_json::from_str::<Bridge>(&t).ok());
    if bridge.is_some() {
        *app.state::<Hue>().bridge.lock().unwrap() = bridge;
        start_events(app.clone());
    }
}

// ---------------------------------------------------------------------------
// Discovery and pairing

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FoundBridge {
    id: String,
    ip: String,
}

fn discover_blocking(wait: Duration) -> Vec<FoundBridge> {
    let Ok(mdns) = mdns_sd::ServiceDaemon::new() else { return Vec::new() };
    let Ok(receiver) = mdns.browse("_hue._tcp.local.") else { return Vec::new() };
    let deadline = Instant::now() + wait;
    let mut found: Vec<FoundBridge> = Vec::new();
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        let Ok(event) = receiver.recv_timeout(left) else { break };
        if let mdns_sd::ServiceEvent::ServiceResolved(service) = event {
            let id = service.txt_properties.get_property_val_str("bridgeid").unwrap_or_default().to_lowercase();
            if let Some(ip) = service.get_addresses_v4().into_iter().next() {
                if !found.iter().any(|b| b.ip == ip.to_string()) {
                    found.push(FoundBridge { id, ip: ip.to_string() });
                }
            }
        }
    }
    let _ = mdns.shutdown();
    found
}

/// Finds Hue bridges in the local network (mDNS, like the Hue app).
#[tauri::command]
pub async fn hue_discover() -> Result<Vec<FoundBridge>, String> {
    tauri::async_runtime::spawn_blocking(|| discover_blocking(Duration::from_secs(4)))
        .await
        .map_err(|e| e.to_string())
}

/// Pairs with the bridge at `ip`: waits up to a minute for the button on the bridge to be pressed.
#[tauri::command]
pub async fn hue_pair(app: AppHandle, hue: State<'_, Hue>, ip: String) -> Result<(), String> {
    let ip: IpAddr = ip.trim().parse().map_err(|_| "Ungültige IP-Adresse".to_string())?;
    let seen = Arc::new(Mutex::new(None));
    let http = client(None, seen.clone(), Some(Duration::from_secs(10)))?;
    let config: Value = http
        .get(format!("https://{ip}/api/0/config"))
        .send()
        .await
        .map_err(|e| format!("Unter {ip} antwortet keine Hue Bridge: {e}"))?
        .json()
        .await
        .map_err(|_| format!("Unter {ip} antwortet keine Hue Bridge"))?;
    let id = config["bridgeid"].as_str().ok_or("Das ist keine Hue Bridge")?.to_lowercase();
    let fingerprint = seen.lock().unwrap().ok_or("Kein Zertifikat von der Bridge erhalten")?;

    let deadline = Instant::now() + Duration::from_secs(60);
    let key = loop {
        let answer: Value = http
            .post(format!("https://{ip}/api"))
            .json(&json!({ "devicetype": "mrhome#windows", "generateclientkey": true }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        if let Some(key) = answer[0]["success"]["username"].as_str() {
            break key.to_owned();
        }
        // 101: link button not pressed yet.
        if answer[0]["error"]["type"].as_u64() != Some(101) {
            let reason = answer[0]["error"]["description"].as_str().unwrap_or("unbekannt");
            return Err(format!("Koppeln fehlgeschlagen: {reason}"));
        }
        if Instant::now() > deadline {
            return Err("Der Knopf auf der Bridge wurde nicht gedrückt".into());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    };

    secrets::set(Secret::HueKey, &key)?;
    let bridge = Bridge { id, ip: ip.to_string(), fingerprint: hex(&fingerprint) };
    save_bridge(&app, Some(&bridge))?;
    *hue.bridge.lock().unwrap() = Some(bridge);
    *hue.http.lock().unwrap() = None;
    start_events(app.clone());
    Ok(())
}

#[tauri::command]
pub fn hue_unpair(app: AppHandle, hue: State<'_, Hue>) -> Result<(), String> {
    *hue.bridge.lock().unwrap() = None;
    *hue.http.lock().unwrap() = None;
    *hue.generation.lock().unwrap() += 1;
    secrets::delete(Secret::HueKey)?;
    save_bridge(&app, None)
}

#[tauri::command]
pub fn hue_status(hue: State<'_, Hue>) -> Option<FoundBridge> {
    hue.bridge.lock().unwrap().as_ref().map(|b| FoundBridge { id: b.id.clone(), ip: b.ip.clone() })
}

// ---------------------------------------------------------------------------
// State

#[derive(Serialize, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Light {
    id: String,
    name: String,
    /// Hue's icon hint, e.g. `sultan_bulb`, `hue_lightstrip`.
    archetype: String,
    on: bool,
    /// 0–100, `None` for on/off-only lights.
    brightness: Option<f64>,
    /// CIE xy colour, `None` for lights without colour.
    xy: Option<[f64; 2]>,
    /// Colour temperature in mirek (153–500), `None` if the light is in colour mode or has none.
    mirek: Option<u32>,
    mirek_min: Option<u32>,
    mirek_max: Option<u32>,
    reachable: bool,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    id: String,
    /// `room` or `zone`.
    kind: &'static str,
    name: String,
    archetype: String,
    lights: Vec<String>,
    /// The `grouped_light` that switches all lights of the group.
    grouped_light: Option<String>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    id: String,
    name: String,
    group: String,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HueState {
    lights: Vec<Light>,
    groups: Vec<Group>,
    scenes: Vec<Scene>,
}

fn data(v: &Value) -> &[Value] {
    v["data"].as_array().map(Vec::as_slice).unwrap_or_default()
}

fn xy_of(v: &Value) -> Option<[f64; 2]> {
    v.pointer("/color/xy").and_then(|xy| Some([xy["x"].as_f64()?, xy["y"].as_f64()?]))
}

fn mirek_of(v: &Value) -> Option<u32> {
    v.pointer("/color_temperature/mirek").and_then(Value::as_u64).map(|m| m as u32)
}

fn parse_light(l: &Value, reachable: &HashMap<String, bool>) -> Option<Light> {
    let owner = l.pointer("/owner/rid").and_then(Value::as_str).unwrap_or_default();
    let schema = |key: &str| l.pointer(&format!("/color_temperature/mirek_schema/{key}")).and_then(Value::as_u64).map(|m| m as u32);
    Some(Light {
        id: l["id"].as_str()?.to_owned(),
        name: l.pointer("/metadata/name").and_then(Value::as_str).unwrap_or("Lampe").to_owned(),
        archetype: l.pointer("/metadata/archetype").and_then(Value::as_str).unwrap_or_default().to_owned(),
        on: l.pointer("/on/on").and_then(Value::as_bool).unwrap_or(false),
        brightness: l.pointer("/dimming/brightness").and_then(Value::as_f64),
        xy: xy_of(l),
        mirek: mirek_of(l),
        mirek_min: schema("mirek_minimum"),
        mirek_max: schema("mirek_maximum"),
        reachable: reachable.get(owner).copied().unwrap_or(true),
    })
}

fn parse_state(lights: &Value, rooms: &Value, zones: &Value, scenes: &Value, connectivity: &Value) -> HueState {
    // zigbee_connectivity tells whether a device answers; lights point to their device as owner.
    let reachable: HashMap<String, bool> = data(connectivity)
        .iter()
        .filter_map(|c| Some((c.pointer("/owner/rid")?.as_str()?.to_owned(), c["status"].as_str()? == "connected")))
        .collect();
    // Rooms list devices, zones list lights – both become light ids.
    let mut device_lights: HashMap<String, Vec<String>> = HashMap::new();
    for l in data(lights) {
        if let (Some(owner), Some(id)) = (l.pointer("/owner/rid").and_then(Value::as_str), l["id"].as_str()) {
            device_lights.entry(owner.to_owned()).or_default().push(id.to_owned());
        }
    }
    let group = |g: &Value, kind: &'static str| -> Option<Group> {
        let mut ids = Vec::new();
        for child in g["children"].as_array()? {
            let rid = child["rid"].as_str().unwrap_or_default();
            match child["rtype"].as_str() {
                Some("light") => ids.push(rid.to_owned()),
                Some("device") => ids.extend(device_lights.get(rid).cloned().unwrap_or_default()),
                _ => {}
            }
        }
        let grouped_light = g["services"]
            .as_array()
            .and_then(|s| s.iter().find(|s| s["rtype"] == "grouped_light"))
            .and_then(|s| s["rid"].as_str())
            .map(str::to_owned);
        Some(Group {
            id: g["id"].as_str()?.to_owned(),
            kind,
            name: g.pointer("/metadata/name").and_then(Value::as_str).unwrap_or("Raum").to_owned(),
            archetype: g.pointer("/metadata/archetype").and_then(Value::as_str).unwrap_or_default().to_owned(),
            lights: ids,
            grouped_light,
        })
    };
    let mut groups: Vec<Group> = data(rooms).iter().filter_map(|g| group(g, "room")).collect();
    groups.extend(data(zones).iter().filter_map(|g| group(g, "zone")));
    groups.sort_by(|a, b| (a.kind, a.name.to_lowercase()).cmp(&(b.kind, b.name.to_lowercase())));

    let mut scenes: Vec<Scene> = data(scenes)
        .iter()
        .filter_map(|s| {
            Some(Scene {
                id: s["id"].as_str()?.to_owned(),
                name: s.pointer("/metadata/name")?.as_str()?.to_owned(),
                group: s.pointer("/group/rid")?.as_str()?.to_owned(),
            })
        })
        .collect();
    scenes.sort_by_key(|s| s.name.to_lowercase());

    let mut lights: Vec<Light> = data(lights).iter().filter_map(|l| parse_light(l, &reachable)).collect();
    lights.sort_by_key(|l| l.name.to_lowercase());
    HueState { lights, groups, scenes }
}

/// Everything the light view shows, in one go (five requests, all local).
#[tauri::command]
pub async fn hue_state(hue: State<'_, Hue>) -> Result<HueState, String> {
    let paths = ["light", "room", "zone", "scene", "zigbee_connectivity"].map(|kind| format!("/clip/v2/resource/{kind}"));
    let (lights, rooms, zones, scenes, connectivity) = tokio::try_join!(
        hue.request(Method::GET, &paths[0], None),
        hue.request(Method::GET, &paths[1], None),
        hue.request(Method::GET, &paths[2], None),
        hue.request(Method::GET, &paths[3], None),
        hue.request(Method::GET, &paths[4], None),
    )?;
    Ok(parse_state(&lights, &rooms, &zones, &scenes, &connectivity))
}

/// What to change; missing fields stay as they are.
#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    on: Option<bool>,
    brightness: Option<f64>,
    xy: Option<[f64; 2]>,
    mirek: Option<u32>,
}

fn change_body(change: &Change) -> Value {
    let mut body = json!({});
    if let Some(on) = change.on {
        body["on"] = json!({ "on": on });
    }
    if let Some(b) = change.brightness {
        body["dimming"] = json!({ "brightness": b.clamp(1.0, 100.0) });
    }
    if let Some([x, y]) = change.xy {
        body["color"] = json!({ "xy": { "x": x.clamp(0.0, 1.0), "y": y.clamp(0.0, 1.0) } });
    }
    if let Some(m) = change.mirek {
        body["color_temperature"] = json!({ "mirek": m.clamp(153, 500) });
    }
    body
}

#[tauri::command]
pub async fn hue_set_light(hue: State<'_, Hue>, id: String, change: Change) -> Result<(), String> {
    hue.request(Method::PUT, &format!("/clip/v2/resource/light/{id}"), Some(change_body(&change))).await.map(drop)
}

/// Switches or dims all lights of a room or zone at once.
#[tauri::command]
pub async fn hue_set_group(hue: State<'_, Hue>, id: String, change: Change) -> Result<(), String> {
    let path = format!("/clip/v2/resource/grouped_light/{id}");
    hue.request(Method::PUT, &path, Some(change_body(&change))).await.map(drop)
}

#[tauri::command]
pub async fn hue_scene(hue: State<'_, Hue>, id: String) -> Result<(), String> {
    let body = json!({ "recall": { "action": "active" } });
    hue.request(Method::PUT, &format!("/clip/v2/resource/scene/{id}"), Some(body)).await.map(drop)
}

// ---------------------------------------------------------------------------
// Live events

/// A change reported by the bridge – also when switched in the Hue app or by voice assistant.
#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LightUpdate {
    id: String,
    on: Option<bool>,
    brightness: Option<f64>,
    xy: Option<[f64; 2]>,
    mirek: Option<u32>,
}

/// Light updates from one server-sent-events message (`data: [...]`).
fn parse_events(message: &str) -> Vec<LightUpdate> {
    let json: String = message.lines().filter_map(|l| l.strip_prefix("data:")).map(str::trim).collect();
    let Ok(events) = serde_json::from_str::<Value>(&json) else { return Vec::new() };
    events
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["type"] == "update")
        .flat_map(|e| e["data"].as_array().cloned().unwrap_or_default())
        .filter(|d| d["type"] == "light")
        .filter_map(|d| {
            Some(LightUpdate {
                id: d["id"].as_str()?.to_owned(),
                on: d.pointer("/on/on").and_then(Value::as_bool),
                brightness: d.pointer("/dimming/brightness").and_then(Value::as_f64),
                xy: xy_of(&d),
                mirek: mirek_of(&d),
            })
        })
        .collect()
}

/// Listens to the bridge's event stream and forwards light changes as `hue-update` events.
/// Reconnects after errors; ends when the pairing changes.
fn start_events(app: AppHandle) {
    let generation = {
        let hue = app.state::<Hue>();
        let mut g = hue.generation.lock().unwrap();
        *g += 1;
        *g
    };
    tauri::async_runtime::spawn(async move {
        let current = || *app.state::<Hue>().generation.lock().unwrap() == generation;
        while current() {
            let result = async {
                let (bridge, key, _) = app.state::<Hue>().connection()?;
                let expected = unhex(&bridge.fingerprint).ok_or("Ungültiger Fingerabdruck")?;
                // No overall timeout: the stream stays open.
                let http = client(Some(expected), Default::default(), None)?;
                let mut response = http
                    .get(format!("https://{}/eventstream/clip/v2", bridge.ip))
                    .header("hue-application-key", key)
                    .header("Accept", "text/event-stream")
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                let _ = app.emit("hue-connected", true);
                let mut buffer = String::new();
                while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                    if !current() {
                        return Ok(());
                    }
                    buffer.push_str(&String::from_utf8_lossy(&chunk));
                    while let Some(end) = buffer.find("\n\n") {
                        let message: String = buffer.drain(..end + 2).collect();
                        let updates = parse_events(&message);
                        if !updates.is_empty() {
                            let _ = app.emit("hue-update", updates);
                        }
                    }
                }
                Ok::<(), String>(())
            }
            .await;
            if !current() {
                break;
            }
            if result.is_err() {
                let _ = app.emit("hue-connected", false);
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_round_trip() {
        let fp: Fingerprint = Sha256::digest(b"cert").into();
        assert_eq!(unhex(&hex(&fp)), Some(fp));
        assert_eq!(unhex("zz"), None);
        assert_eq!(unhex("00"), None);
    }

    #[test]
    fn builds_change_bodies() {
        let body = change_body(&Change { on: Some(true), brightness: Some(150.0), xy: Some([0.3, 0.4]), mirek: Some(100) });
        assert_eq!(
            body,
            json!({
                "on": { "on": true },
                "dimming": { "brightness": 100.0 },
                "color": { "xy": { "x": 0.3, "y": 0.4 } },
                "color_temperature": { "mirek": 153 }
            })
        );
        assert_eq!(change_body(&Change::default()), json!({}));
    }

    fn sample() -> HueState {
        let lights = json!({ "data": [
            { "id": "l1", "owner": { "rid": "d1" }, "metadata": { "name": "Decke", "archetype": "sultan_bulb" },
              "on": { "on": true }, "dimming": { "brightness": 80.0 },
              "color": { "xy": { "x": 0.45, "y": 0.41 } },
              "color_temperature": { "mirek": 366, "mirek_schema": { "mirek_minimum": 153, "mirek_maximum": 500 } } },
            { "id": "l2", "owner": { "rid": "d2" }, "metadata": { "name": "Ambiente" }, "on": { "on": false } }
        ]});
        let rooms = json!({ "data": [{ "id": "r1", "metadata": { "name": "Wohnzimmer", "archetype": "living_room" },
            "children": [{ "rid": "d1", "rtype": "device" }, { "rid": "d2", "rtype": "device" }],
            "services": [{ "rid": "g1", "rtype": "grouped_light" }] }] });
        let zones = json!({ "data": [{ "id": "z1", "metadata": { "name": "Fernsehen" },
            "children": [{ "rid": "l2", "rtype": "light" }], "services": [{ "rid": "g2", "rtype": "grouped_light" }] }] });
        let scenes = json!({ "data": [{ "id": "s1", "metadata": { "name": "Entspannen" }, "group": { "rid": "r1", "rtype": "room" } }] });
        let connectivity = json!({ "data": [{ "owner": { "rid": "d2" }, "status": "connectivity_issue" }] });
        parse_state(&lights, &rooms, &zones, &scenes, &connectivity)
    }

    #[test]
    fn maps_lights_to_rooms_and_zones() {
        let state = sample();
        assert_eq!(state.lights.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Ambiente", "Decke"]);
        let decke = state.lights.iter().find(|l| l.id == "l1").unwrap();
        assert_eq!((decke.on, decke.brightness, decke.mirek, decke.mirek_min), (true, Some(80.0), Some(366), Some(153)));
        assert!(decke.reachable);
        assert!(!state.lights.iter().find(|l| l.id == "l2").unwrap().reachable);
        assert_eq!(
            state.groups[0],
            Group {
                id: "r1".into(),
                kind: "room",
                name: "Wohnzimmer".into(),
                archetype: "living_room".into(),
                lights: vec!["l1".into(), "l2".into()],
                grouped_light: Some("g1".into()),
            }
        );
        assert_eq!((state.groups[1].kind, state.groups[1].lights.clone()), ("zone", vec!["l2".to_string()]));
        assert_eq!(state.scenes, vec![Scene { id: "s1".into(), name: "Entspannen".into(), group: "r1".into() }]);
    }

    #[test]
    fn reads_event_stream_messages() {
        let message = "id: 1:0\ndata: [{\"type\":\"update\",\"data\":[{\"id\":\"l1\",\"type\":\"light\",\"on\":{\"on\":false}},{\"id\":\"x\",\"type\":\"button\"}]}]\n\n";
        assert_eq!(parse_events(message), vec![LightUpdate { id: "l1".into(), on: Some(false), ..Default::default() }]);
        assert!(parse_events(": hi\n\n").is_empty());
    }
}

#[cfg(test)]
mod network {
    /// `cargo test -p mrhome --lib hue::network -- --ignored --nocapture` – read-only mDNS search.
    #[test]
    #[ignore]
    fn finds_bridges() {
        println!("Hue Bridges: {:?}", super::discover_blocking(std::time::Duration::from_secs(5)));
    }
}
