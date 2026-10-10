//! tado° X over tado's cloud API: `hops.tado.com` for rooms (tado X), `my.tado.com` for homes and
//! presence, `login.tado.com` for OAuth (device code flow). The API is unofficial; tado limits it to
//! 100 calls a day without an Auto-Assist subscription and reports the rest in the `ratelimit` header.

use crate::secrets::{self, Secret};
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, State};

/// tado's own web app client – the one every third-party integration uses for the device code flow.
const CLIENT_ID: &str = "1bb50063-6b0c-4d11-bd99-387f4a91cc46";
const LOGIN: &str = "https://login.tado.com/oauth2";
const MY: &str = "https://my.tado.com/api/v2";
const HOPS: &str = "https://hops.tado.com";

/// Returned as error when there is no valid login – the frontend shows the login view.
pub const LOGGED_OUT: &str = "LOGGED_OUT";

// ---------------------------------------------------------------------------
// State

struct Session {
    access_token: String,
    valid_until: Instant,
}

pub struct Tado {
    http: reqwest::Client,
    session: tauri::async_runtime::Mutex<Option<Session>>,
    quota: Mutex<Option<Quota>>,
}

impl Default for Tado {
    fn default() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("mrhome/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(20))
                .build()
                .expect("HTTP client"),
            session: Default::default(),
            quota: Default::default(),
        }
    }
}

/// tado's daily request budget as last reported.
#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Quota {
    pub remaining: Option<u64>,
    pub limit: Option<u64>,
    /// ms since 1970 when the budget is renewed.
    pub reset_at: Option<u64>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// `"perday";r=87;t=3600` → value of `key` (`r`, `q`, `t`, …).
fn header_param(value: &str, key: &str) -> Option<u64> {
    value
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find(|(k, _)| k.trim() == key)
        .and_then(|(_, v)| v.trim().trim_matches('"').parse().ok())
}

/// Reads `ratelimit: "perday";r=…;t=…` and `ratelimit-policy: "perday";q=…;w=…`.
fn parse_quota(ratelimit: Option<&str>, policy: Option<&str>, now: u64) -> Option<Quota> {
    let remaining = ratelimit.and_then(|v| header_param(v, "r"));
    let limit = policy.and_then(|v| header_param(v, "q"));
    if remaining.is_none() && limit.is_none() {
        return None;
    }
    let reset_at = ratelimit.and_then(|v| header_param(v, "t")).map(|t| now + t * 1000);
    Some(Quota { remaining, limit, reset_at })
}

// ---------------------------------------------------------------------------
// Login

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
}

impl Tado {
    async fn token_request(&self, form: &[(&str, &str)]) -> Result<Result<TokenResponse, String>, String> {
        let response = self
            .http
            .post(format!("{LOGIN}/token"))
            .form(form)
            .send()
            .await
            .map_err(|e| format!("tado nicht erreichbar: {e}"))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| e.to_string())?;
        if status.is_success() {
            return serde_json::from_str(&text).map(Ok).map_err(|e| format!("Unerwartete Antwort von tado: {e}"));
        }
        // OAuth errors like "authorization_pending" or "invalid_grant".
        Ok(Err(serde_json::from_str::<OAuthError>(&text).map_or_else(|_| format!("HTTP {status}"), |e| e.error)))
    }

    async fn store(&self, token: TokenResponse) -> Result<String, String> {
        // tado rotates refresh tokens: the old one is invalid from now on.
        secrets::set(Secret::TadoRefreshToken, &token.refresh_token)?;
        let access = token.access_token.clone();
        *self.session.lock().await = Some(Session {
            access_token: token.access_token,
            // A minute early, so a request never starts with a token about to expire.
            valid_until: Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(60)),
        });
        Ok(access)
    }

    /// A valid access token, refreshed with the stored refresh token when needed.
    async fn access_token(&self, force_refresh: bool) -> Result<String, String> {
        {
            let session = self.session.lock().await;
            if let Some(s) = session.as_ref().filter(|s| !force_refresh && s.valid_until > Instant::now()) {
                return Ok(s.access_token.clone());
            }
        }
        let refresh = tauri::async_runtime::spawn_blocking(|| secrets::get(Secret::TadoRefreshToken))
            .await
            .map_err(|e| e.to_string())?
            .ok_or(LOGGED_OUT)?;
        match self
            .token_request(&[("client_id", CLIENT_ID), ("grant_type", "refresh_token"), ("refresh_token", &refresh)])
            .await?
        {
            Ok(token) => self.store(token).await,
            Err(error) if error == "invalid_grant" => {
                // Expired (after 30 days unused) or revoked – a new login is needed.
                let _ = secrets::delete(Secret::TadoRefreshToken);
                Err(LOGGED_OUT.into())
            }
            Err(error) => Err(format!("Anmeldung bei tado fehlgeschlagen: {error}")),
        }
    }

    /// Sends a request to tado. Bodiless answers (204) become `Value::Null`.
    async fn request(&self, app: &AppHandle, method: Method, url: &str, body: Option<Value>) -> Result<Value, String> {
        for attempt in 0..2 {
            let token = self.access_token(attempt > 0).await?;
            let mut builder = self.http.request(method.clone(), url).bearer_auth(&token);
            if url.starts_with(HOPS) {
                builder = builder.query(&[("ngsw-bypass", "true")]);
            }
            if let Some(body) = &body {
                builder = builder.json(body);
            }
            let response = builder.send().await.map_err(|e| format!("tado nicht erreichbar: {e}"))?;

            let header = |name: &str| response.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_owned);
            if let Some(quota) = parse_quota(header("ratelimit").as_deref(), header("ratelimit-policy").as_deref(), now_ms()) {
                *self.quota.lock().unwrap() = Some(quota.clone());
                let _ = app.emit("quota", quota);
            }

            let status = response.status();
            if status == StatusCode::UNAUTHORIZED && attempt == 0 {
                continue;
            }
            let text = response.text().await.map_err(|e| e.to_string())?;
            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err("Das Tageslimit von tado ist erreicht – die Steuerung geht erst nach dem Zurücksetzen wieder".into());
            }
            if !status.is_success() {
                let detail: String = text.chars().take(200).collect();
                return Err(format!("tado antwortet mit {status}{}", if detail.is_empty() { String::new() } else { format!(": {detail}") }));
            }
            return Ok(if text.trim().is_empty() { Value::Null } else { serde_json::from_str(&text).map_err(|e| e.to_string())? });
        }
        Err(LOGGED_OUT.into())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLogin {
    device_code: String,
    user_code: String,
    /// Opens tado's login page with the code filled in.
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

/// Starts the login: the user confirms the code on tado's website.
#[tauri::command]
pub async fn login_start(tado: State<'_, Tado>) -> Result<DeviceLogin, String> {
    #[derive(Deserialize)]
    struct Answer {
        device_code: String,
        user_code: String,
        verification_uri_complete: String,
        expires_in: u64,
        interval: Option<u64>,
    }
    let answer: Answer = tado
        .http
        .post(format!("{LOGIN}/device_authorize"))
        .form(&[("client_id", CLIENT_ID), ("scope", "offline_access")])
        .send()
        .await
        .map_err(|e| format!("tado nicht erreichbar: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("Unerwartete Antwort von tado: {e}"))?;
    Ok(DeviceLogin {
        device_code: answer.device_code,
        user_code: answer.user_code,
        verification_uri: answer.verification_uri_complete,
        expires_in: answer.expires_in,
        interval: answer.interval.unwrap_or(5),
    })
}

/// Waits until the user has confirmed the code (or it expires).
#[tauri::command]
pub async fn login_finish(tado: State<'_, Tado>, login: DeviceLogin) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(login.expires_in);
    let mut interval = login.interval.max(1);
    while Instant::now() < deadline {
        tokio::time::sleep(Duration::from_secs(interval)).await;
        let result = tado
            .token_request(&[
                ("client_id", CLIENT_ID),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &login.device_code),
            ])
            .await?;
        match result {
            Ok(token) => return tado.store(token).await.map(drop),
            Err(e) if e == "authorization_pending" => {}
            Err(e) if e == "slow_down" => interval += 5,
            Err(e) if e == "access_denied" => return Err("Die Anmeldung wurde abgelehnt".into()),
            Err(e) if e == "expired_token" => break,
            Err(e) => return Err(format!("Anmeldung fehlgeschlagen: {e}")),
        }
    }
    Err("Der Code ist abgelaufen – bitte erneut anmelden".into())
}

#[tauri::command]
pub async fn logout(tado: State<'_, Tado>) -> Result<(), String> {
    *tado.session.lock().await = None;
    secrets::delete(Secret::TadoRefreshToken)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    logged_in: bool,
    quota: Option<Quota>,
}

#[tauri::command]
pub fn auth_status(tado: State<'_, Tado>) -> AuthStatus {
    AuthStatus { logged_in: secrets::get(Secret::TadoRefreshToken).is_some(), quota: tado.quota.lock().unwrap().clone() }
}

// ---------------------------------------------------------------------------
// Homes and rooms

#[derive(Serialize, Debug, PartialEq)]
pub struct Home {
    id: u64,
    name: String,
}

#[tauri::command]
pub async fn homes(app: AppHandle, tado: State<'_, Tado>) -> Result<Vec<Home>, String> {
    let me = tado.request(&app, Method::GET, &format!("{MY}/me"), None).await?;
    Ok(parse_homes(&me))
}

fn parse_homes(me: &Value) -> Vec<Home> {
    me["homes"]
        .as_array()
        .map(|homes| {
            homes
                .iter()
                .filter_map(|h| Some(Home { id: h["id"].as_u64()?, name: h["name"].as_str().unwrap_or("Zuhause").to_owned() }))
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NextChange {
    /// ISO time.
    start: String,
    power: bool,
    temperature: Option<f64>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    id: u64,
    name: String,
    /// Measured °C.
    temperature: Option<f64>,
    humidity: Option<f64>,
    /// Heating on (false: off / frost protection).
    power: bool,
    /// Target °C.
    target: Option<f64>,
    /// Valve opening in percent.
    heating_power: Option<f64>,
    connected: bool,
    /// Seconds until open-window mode ends, if it is active.
    open_window: Option<u64>,
    /// `schedule`, `boost`, `nextBlock` (manual until the next schedule change), `timer` or `manual`.
    mode: &'static str,
    /// ISO time when manual control or boost ends.
    until: Option<String>,
    next_change: Option<NextChange>,
}

fn parse_room(v: &Value) -> Option<Room> {
    let f = |path: &str| v.pointer(path).and_then(Value::as_f64);
    let s = |path: &str| v.pointer(path).and_then(Value::as_str);
    let termination = |key: &str| v.get(key).filter(|t| t.is_object());
    let (mode, until) = if let Some(boost) = termination("boostMode") {
        ("boost", boost["projectedExpiry"].as_str())
    } else if let Some(manual) = termination("manualControlTermination") {
        let mode = match manual["type"].as_str() {
            Some("TIMER") => "timer",
            Some("NEXT_TIME_BLOCK") => "nextBlock",
            _ => "manual",
        };
        (mode, manual["projectedExpiry"].as_str())
    } else {
        ("schedule", None)
    };
    let open_window = v
        .get("openWindow")
        .filter(|w| w["activated"].as_bool() == Some(true))
        .map(|w| w["expiryInSeconds"].as_u64().unwrap_or(0));
    let next_change = v.get("nextScheduleChange").filter(|n| n.is_object()).and_then(|n| {
        Some(NextChange {
            start: n["start"].as_str()?.to_owned(),
            power: n.pointer("/setting/power").and_then(Value::as_str) != Some("OFF"),
            temperature: n.pointer("/setting/temperature/value").and_then(Value::as_f64),
        })
    });
    Some(Room {
        id: v["id"].as_u64()?,
        name: v["name"].as_str().unwrap_or("Raum").to_owned(),
        temperature: f("/sensorDataPoints/insideTemperature/value"),
        humidity: f("/sensorDataPoints/humidity/percentage"),
        power: s("/setting/power") != Some("OFF"),
        target: f("/setting/temperature/value"),
        heating_power: f("/heatingPower/percentage"),
        connected: s("/connection/state").is_none_or(|state| state == "CONNECTED"),
        open_window,
        mode,
        until: until.map(str::to_owned),
        next_change,
    })
}

#[tauri::command]
pub async fn rooms(app: AppHandle, tado: State<'_, Tado>, home: u64) -> Result<Vec<Room>, String> {
    let rooms = tado.request(&app, Method::GET, &format!("{HOPS}/homes/{home}/rooms"), None).await?;
    Ok(rooms.as_array().map(|r| r.iter().filter_map(parse_room).collect()).unwrap_or_default())
}

#[tauri::command]
pub async fn room(app: AppHandle, tado: State<'_, Tado>, home: u64, room: u64) -> Result<Room, String> {
    let value = tado.request(&app, Method::GET, &format!("{HOPS}/homes/{home}/rooms/{room}"), None).await?;
    parse_room(&value).ok_or_else(|| "Unerwartete Antwort von tado".into())
}

/// How long a manual setting lasts.
#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Termination {
    /// Until the schedule changes next.
    NextBlock,
    Timer { seconds: u64 },
    /// Until changed again.
    Manual,
}

fn manual_body(temperature: Option<f64>, termination: Termination) -> Value {
    let setting = match temperature {
        Some(t) => json!({ "power": "ON", "isBoost": false, "temperature": { "value": (t * 10.0).round() / 10.0 } }),
        None => json!({ "power": "OFF", "isBoost": false, "temperature": null }),
    };
    let termination = match termination {
        Termination::NextBlock => json!({ "type": "NEXT_TIME_BLOCK" }),
        Termination::Timer { seconds } => json!({ "type": "TIMER", "durationInSeconds": seconds }),
        Termination::Manual => json!({ "type": "MANUAL" }),
    };
    json!({ "setting": setting, "termination": termination })
}

/// Sets a room to `temperature` (`None`: heating off).
#[tauri::command]
pub async fn set_room(
    app: AppHandle,
    tado: State<'_, Tado>,
    home: u64,
    room: u64,
    temperature: Option<f64>,
    termination: Termination,
) -> Result<(), String> {
    let url = format!("{HOPS}/homes/{home}/rooms/{room}/manualControl");
    tado.request(&app, Method::POST, &url, Some(manual_body(temperature, termination))).await.map(drop)
}

/// Ends manual control: the room follows its schedule again.
#[tauri::command]
pub async fn resume_room(app: AppHandle, tado: State<'_, Tado>, home: u64, room: u64) -> Result<(), String> {
    let url = format!("{HOPS}/homes/{home}/rooms/{room}/manualControl");
    tado.request(&app, Method::DELETE, &url, None).await.map(drop)
}

#[tauri::command]
pub async fn end_open_window(app: AppHandle, tado: State<'_, Tado>, home: u64, room: u64) -> Result<(), String> {
    let url = format!("{HOPS}/homes/{home}/rooms/{room}/openWindow");
    tado.request(&app, Method::DELETE, &url, None).await.map(drop)
}

/// `boost` (all rooms, 30 min), `allOff` or `resumeSchedule` (all rooms).
#[tauri::command]
pub async fn quick_action(app: AppHandle, tado: State<'_, Tado>, home: u64, action: String) -> Result<(), String> {
    if !["boost", "allOff", "resumeSchedule"].contains(&action.as_str()) {
        return Err(format!("Unbekannte Aktion: {action}"));
    }
    let url = format!("{HOPS}/homes/{home}/quickActions/{action}");
    tado.request(&app, Method::POST, &url, None).await.map(drop)
}

// ---------------------------------------------------------------------------
// Presence

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HomeState {
    /// `HOME` or `AWAY`.
    presence: String,
    /// Set by hand instead of by geofencing.
    locked: bool,
}

#[tauri::command]
pub async fn home_state(app: AppHandle, tado: State<'_, Tado>, home: u64) -> Result<HomeState, String> {
    let state = tado.request(&app, Method::GET, &format!("{MY}/homes/{home}/state"), None).await?;
    Ok(HomeState {
        presence: state["presence"].as_str().unwrap_or("HOME").to_owned(),
        locked: state["presenceLocked"].as_bool().unwrap_or(false),
    })
}

/// `HOME` or `AWAY` by hand; `None` hands control back to geofencing.
#[tauri::command]
pub async fn set_presence(app: AppHandle, tado: State<'_, Tado>, home: u64, presence: Option<String>) -> Result<(), String> {
    let url = format!("{MY}/homes/{home}/presenceLock");
    match presence.as_deref() {
        Some(p @ ("HOME" | "AWAY")) => {
            tado.request(&app, Method::PUT, &url, Some(json!({ "homePresence": p }))).await.map(drop)
        }
        Some(other) => Err(format!("Unbekannter Zustand: {other}")),
        None => tado.request(&app, Method::DELETE, &url, None).await.map(drop),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_quota_headers() {
        let q = parse_quota(Some(r#""perday";r=87;t=3600"#), Some(r#""perday";q=100;w=86400"#), 1_000).unwrap();
        assert_eq!(q, Quota { remaining: Some(87), limit: Some(100), reset_at: Some(1_000 + 3_600_000) });
        assert_eq!(parse_quota(None, None, 0), None);
        assert_eq!(parse_quota(Some(r#""perday";r=0"#), None, 0).unwrap().remaining, Some(0));
    }

    #[test]
    fn parses_a_room_in_manual_mode() {
        let v = json!({
            "id": 3, "name": "Wohnzimmer",
            "sensorDataPoints": { "insideTemperature": { "value": 20.4 }, "humidity": { "percentage": 48 } },
            "setting": { "power": "ON", "temperature": { "value": 22.0 } },
            "heatingPower": { "percentage": 35 },
            "connection": { "state": "CONNECTED" },
            "openWindow": null,
            "manualControlTermination": { "type": "TIMER", "remainingTimeInSeconds": 1800, "projectedExpiry": "2026-10-10T12:00:00Z" },
            "boostMode": null,
            "nextScheduleChange": { "start": "2026-10-10T21:00:00Z", "setting": { "power": "ON", "temperature": { "value": 18.0 } } }
        });
        let room = parse_room(&v).unwrap();
        assert_eq!(room.name, "Wohnzimmer");
        assert_eq!((room.temperature, room.humidity, room.target, room.heating_power), (Some(20.4), Some(48.0), Some(22.0), Some(35.0)));
        assert_eq!((room.mode, room.until.as_deref()), ("timer", Some("2026-10-10T12:00:00Z")));
        assert!(room.power && room.connected && room.open_window.is_none());
        assert_eq!(room.next_change.unwrap().temperature, Some(18.0));
    }

    #[test]
    fn parses_schedule_boost_off_and_open_window() {
        let base = json!({ "id": 1, "name": "Bad", "setting": { "power": "OFF", "temperature": null } });
        let room = parse_room(&base).unwrap();
        assert_eq!((room.mode, room.power, room.target), ("schedule", false, None));

        let mut boost = base.clone();
        boost["boostMode"] = json!({ "type": "TIMER", "projectedExpiry": "2026-10-10T10:30:00Z" });
        boost["openWindow"] = json!({ "activated": true, "expiryInSeconds": 600 });
        boost["connection"] = json!({ "state": "DISCONNECTED" });
        let room = parse_room(&boost).unwrap();
        assert_eq!((room.mode, room.open_window, room.connected), ("boost", Some(600), false));

        assert!(parse_room(&json!({ "name": "ohne id" })).is_none());
    }

    #[test]
    fn builds_manual_control_bodies() {
        assert_eq!(
            manual_body(Some(21.04), Termination::NextBlock),
            json!({ "setting": { "power": "ON", "isBoost": false, "temperature": { "value": 21.0 } }, "termination": { "type": "NEXT_TIME_BLOCK" } })
        );
        assert_eq!(
            manual_body(None, Termination::Timer { seconds: 3600 })["termination"],
            json!({ "type": "TIMER", "durationInSeconds": 3600 })
        );
        assert_eq!(manual_body(None, Termination::Manual)["setting"]["power"], "OFF");
    }

    #[test]
    fn reads_terminations_from_the_frontend() {
        let t: Termination = serde_json::from_value(json!({ "kind": "timer", "seconds": 7200 })).unwrap();
        assert!(matches!(t, Termination::Timer { seconds: 7200 }));
        let t: Termination = serde_json::from_value(json!({ "kind": "nextBlock" })).unwrap();
        assert!(matches!(t, Termination::NextBlock));
    }

    #[test]
    fn lists_homes() {
        let me = json!({ "name": "Michael", "homes": [{ "id": 123, "name": "Zuhause" }, { "id": 7 }] });
        assert_eq!(parse_homes(&me), vec![Home { id: 123, name: "Zuhause".into() }, Home { id: 7, name: "Zuhause".into() }]);
    }
}
