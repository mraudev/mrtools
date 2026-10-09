//! Releases of the apps on GitHub (public, no token) and installing them.
//!
//! All apps share the repository mraudev/mrtools; an app's releases are
//! tagged `<folder>-v<version>`. Installers are only downloaded from that
//! repository and only run if their SHA-256 matches the digest GitHub reports.

use crate::catalog;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, time::Duration};

const REPO: &str = "mraudev/mrtools";

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
    draft: bool,
    prerelease: bool,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize, Clone)]
struct GhAsset {
    name: String,
    size: u64,
    browser_download_url: String,
    digest: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    version: String,
    url: String,
    published_at: Option<String>,
    /// Size of the installer in bytes, `None` if the release has none.
    installer_size: Option<u64>,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("mrtools")
        .timeout(Duration::from_secs(120))
        .https_only(true)
        .build()
        .map_err(|e| e.to_string())
}

async fn fetch_releases() -> Result<Vec<GhRelease>, String> {
    let response = client()?
        .get(format!("https://api.github.com/repos/{REPO}/releases?per_page=100"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    match response.status().as_u16() {
        403 | 429 => Err("GitHub-Abfragelimit erreicht – später erneut versuchen.".into()),
        code if !(200..300).contains(&code) => Err(format!("GitHub: HTTP {code}")),
        _ => response.json().await.map_err(|e| e.to_string()),
    }
}

/// The newest published release of `folder` (GitHub lists newest first) and its version.
fn latest<'a>(releases: &'a [GhRelease], folder: &str) -> Option<(&'a GhRelease, String)> {
    let prefix = format!("{folder}-v");
    releases
        .iter()
        .filter(|r| !r.draft && !r.prerelease)
        .find_map(|r| r.tag_name.strip_prefix(&prefix).map(|v| (r, v.to_string())))
}

/// The NSIS installer `<name>_<version>_x64-setup.exe` of a release.
fn installer<'a>(release: &'a GhRelease, name: &str, version: &str) -> Option<&'a GhAsset> {
    let file = format!("{name}_{version}_x64-setup.exe");
    release.assets.iter().find(|a| a.name == file)
}

/// Latest release per app folder; apps without a release are missing.
#[tauri::command]
pub async fn releases() -> Result<HashMap<String, Release>, String> {
    let releases = fetch_releases().await?;
    Ok(catalog::entries()
        .into_iter()
        .filter_map(|entry| {
            let (release, version) = latest(&releases, &entry.folder)?;
            let release = Release {
                installer_size: installer(release, &entry.name, &version).map(|a| a.size),
                url: release.html_url.clone(),
                published_at: release.published_at.clone(),
                version,
            };
            Some((entry.folder, release))
        })
        .collect())
}

fn verify(bytes: &[u8], digest: Option<&str>) -> Result<(), String> {
    let expected = digest
        .and_then(|d| d.strip_prefix("sha256:"))
        .ok_or("GitHub liefert keine Prüfsumme für den Installer.")?;
    let actual: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err("Prüfsumme des Installers stimmt nicht – Download verworfen.".into())
    }
}

/// Downloads the installer of the latest release of `folder` and runs it in
/// passive mode (progress bar only). A running instance of the app is closed
/// by the installer. Returns when the installer has finished.
#[tauri::command]
pub async fn install(folder: String) -> Result<(), String> {
    let entry = catalog::entries()
        .into_iter()
        .find(|e| e.folder == folder)
        .ok_or("Unbekannte App.")?;
    let releases = fetch_releases().await?;
    let (release, version) = latest(&releases, &entry.folder).ok_or("Noch kein Release vorhanden.")?;
    let asset = installer(release, &entry.name, &version)
        .ok_or("Das Release enthält keinen Installer.")?
        .clone();
    if !asset
        .browser_download_url
        .starts_with(&format!("https://github.com/{REPO}/releases/download/"))
    {
        return Err("Unerwartete Download-Adresse.".into());
    }

    let bytes = client()?
        .get(&asset.browser_download_url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    verify(&bytes, asset.digest.as_deref())?;

    let dir = std::env::temp_dir().join("mrtools");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(&asset.name);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

    let status = tauri::async_runtime::spawn_blocking(move || {
        let status = std::process::Command::new(&path).arg("/P").status();
        let _ = std::fs::remove_file(&path);
        status
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("Installer ließ sich nicht starten: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Installer beendet mit Code {}.", status.code().unwrap_or(-1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, prerelease: bool, assets: &[&str]) -> GhRelease {
        GhRelease {
            tag_name: tag.into(),
            html_url: String::new(),
            published_at: None,
            draft: false,
            prerelease,
            assets: assets
                .iter()
                .map(|name| GhAsset {
                    name: (*name).into(),
                    size: 1,
                    browser_download_url: String::new(),
                    digest: None,
                })
                .collect(),
        }
    }

    #[test]
    fn finds_latest_release_of_an_app() {
        let releases = [
            release("mrstart-latest", true, &["latest.json"]),
            release("mrstart-v1.8.0", false, &["mrstart_1.8.0_x64-setup.exe"]),
            release("mrprocs-v1.0.0", false, &[]),
            release("mrstart-v1.7.0", false, &[]),
        ];
        let (r, version) = latest(&releases, "mrstart").unwrap();
        assert_eq!(version, "1.8.0");
        assert!(installer(r, "mrstart", &version).is_some());
        assert!(latest(&releases, "mrdiskspace").is_none());
    }

    #[test]
    fn verifies_sha256() {
        let digest = "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify(b"hello", Some(digest)).is_ok());
        assert!(verify(b"hellO", Some(digest)).is_err());
        assert!(verify(b"hello", None).is_err());
    }
}
