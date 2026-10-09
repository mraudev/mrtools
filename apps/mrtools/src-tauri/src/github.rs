//! Latest release of a public GitHub repository (no token).

use crate::apps::is_name;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct Release {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
    #[serde(default, skip_serializing)]
    draft: bool,
    #[serde(default, skip_serializing)]
    prerelease: bool,
}

/// Tag prefixes such as `mrprocs-v`.
fn is_tag_prefix(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// `None` if the repository has no release (or is not public). With
/// `tag_prefix` (apps in a monorepo), the newest release whose tag starts
/// with it.
#[tauri::command]
pub async fn latest_release(repo: String, tag_prefix: Option<String>) -> Result<Option<Release>, String> {
    let valid = repo
        .split_once('/')
        .is_some_and(|(owner, name)| is_name(owner) && is_name(name));
    if !valid || tag_prefix.as_deref().is_some_and(|p| !is_tag_prefix(p)) {
        return Err("Ungültiges Repository.".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("mrtools")
        .timeout(Duration::from_secs(15))
        .https_only(true)
        .build()
        .map_err(|e| e.to_string())?;
    let url = match tag_prefix {
        Some(_) => format!("https://api.github.com/repos/{repo}/releases?per_page=100"),
        None => format!("https://api.github.com/repos/{repo}/releases/latest"),
    };
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    match response.status().as_u16() {
        404 => return Ok(None),
        403 | 429 => return Err("GitHub-Abfragelimit erreicht – später erneut versuchen.".into()),
        code if !(200..300).contains(&code) => return Err(format!("HTTP {code}")),
        _ => {}
    }
    match tag_prefix {
        // GitHub lists releases newest first.
        Some(prefix) => {
            let releases: Vec<Release> = response.json().await.map_err(|e| e.to_string())?;
            Ok(releases
                .into_iter()
                .find(|r| !r.draft && !r.prerelease && r.tag_name.starts_with(&prefix)))
        }
        None => response.json().await.map(Some).map_err(|e| e.to_string()),
    }
}
