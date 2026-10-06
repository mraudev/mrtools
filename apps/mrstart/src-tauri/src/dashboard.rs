//! Pull request dashboard across Gitea and GitHub: the user's own open pull
//! requests (with "behind base" status) and the reviews requested from them,
//! plus updating a pull request branch with its base.
//!
//! Tokens come from the credential store; everything that ends up in a URL is
//! validated before use.

use crate::pulls::{gitea_host, http_client, is_name};
use crate::secrets::{self, Secret};
use crate::{gitea, github};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// The base branch has commits the pull request branch does not have yet.
    Behind,
    /// Cannot be merged automatically.
    Conflict,
    Clean,
    /// Blocked by branch protection (missing reviews or checks).
    Blocked,
    /// Mergeable, but checks are not green.
    Unstable,
    Draft,
    Unknown,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardPull {
    pub provider: &'static str,
    /// GitHub node ID (needed to update the branch); empty for Gitea.
    pub node_id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub is_draft: bool,
    pub updated_at: String,
    pub created_at: String,
    pub head: String,
    pub base: String,
    pub head_sha: String,
    /// Current tip of the base branch.
    pub base_sha: String,
    /// Commit date of the merge base: the newest state of the base branch the
    /// pull request branch contains (refreshed by merging/rebasing the base in).
    pub base_date: Option<String>,
    pub status: Status,
    /// The branch is behind its base and can be updated.
    pub can_update: bool,
    /// Head and base conflict – updating the branch would not work without
    /// resolving them, so the dashboard offers no update button then.
    pub has_conflicts: bool,
    /// Combined CI state of the head commit: `success`, `failure` or
    /// `pending`; `None` if no checks ran.
    pub ci: Option<&'static str>,
    /// Page with the check results (http/https only).
    pub ci_url: Option<String>,
    /// GitHub only: APPROVED, CHANGES_REQUESTED, REVIEW_REQUIRED.
    pub review_decision: Option<String>,
    pub author: String,
}

/// An open issue assigned to the user.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardIssue {
    pub provider: &'static str,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
    pub author: String,
    pub labels: Vec<String>,
}

/// Own pull requests, review requests and assigned issues of one provider.
pub struct Lists {
    pub authored: Vec<DashboardPull>,
    pub reviews: Vec<DashboardPull>,
    pub issues: Vec<DashboardIssue>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    authored: Vec<DashboardPull>,
    review_requests: Vec<DashboardPull>,
    issues: Vec<DashboardIssue>,
    errors: Vec<String>,
}

/// Maps the CI states of GitHub and Gitea to `success`/`failure`/`pending`.
pub fn ci_state(state: &str) -> Option<&'static str> {
    match state.to_ascii_lowercase().as_str() {
        "success" => Some("success"),
        "failure" | "error" | "warning" => Some("failure"),
        "pending" | "expected" => Some("pending"),
        _ => None,
    }
}

/// Only web links are passed on to be opened in the browser.
pub fn web_link(url: &str) -> Option<String> {
    (url.starts_with("https://") || url.starts_with("http://")).then(|| url.to_string())
}

/// A full commit ID: SHA-1 (40) or SHA-256 (64) hex digits.
pub fn is_sha(s: &str) -> bool {
    matches!(s.len(), 40 | 64) && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[tauri::command]
pub async fn dashboard(gitea_host: String) -> Dashboard {
    let mut result = Dashboard::default();
    let (github_token, gitea_token) = tauri::async_runtime::spawn_blocking(|| {
        (secrets::get(Secret::GitHub), secrets::get(Secret::Gitea))
    })
    .await
    .unwrap_or_default();
    let client = match http_client() {
        Ok(client) => client,
        Err(e) => {
            result.errors.push(e);
            return result;
        }
    };

    let gitea = match (self::gitea_host(&gitea_host), gitea_token) {
        (Some(host), Some(token)) => {
            let client = client.clone();
            Some(tauri::async_runtime::spawn(async move {
                gitea::fetch(&client, &host, &token).await
            }))
        }
        _ => None,
    };
    let github = github_token.map(|token| {
        tauri::async_runtime::spawn(async move { github::fetch(&client, &token).await })
    });

    for (name, task) in [("Gitea", gitea), ("GitHub", github)] {
        let Some(task) = task else { continue };
        match task.await.map_err(|e| e.to_string()).and_then(|r| r) {
            Ok(lists) => {
                result.authored.extend(lists.authored);
                result.review_requests.extend(lists.reviews);
                result.issues.extend(lists.issues);
            }
            Err(e) => result.errors.push(format!("{name}: {e}")),
        }
    }
    // ISO 8601 timestamps sort chronologically as strings; newest first.
    result
        .authored
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    result
        .review_requests
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    result
        .issues
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    result
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequest {
    provider: String,
    node_id: String,
    owner: String,
    repo: String,
    number: u64,
    head_sha: String,
    rebase: bool,
    gitea_host: String,
}

/// Brings a pull request branch up to date with its base (merge or rebase),
/// like the "Update branch" button on Gitea/GitHub.
#[tauri::command]
pub async fn update_pull_branch(request: UpdateRequest) -> Result<(), String> {
    if !is_name(&request.owner) || !is_name(&request.repo) || !is_sha(&request.head_sha) {
        return Err("Ungültige Pull-Request-Angaben.".into());
    }
    let secret = match request.provider.as_str() {
        "github" => Secret::GitHub,
        "gitea" => Secret::Gitea,
        other => return Err(format!("Unbekannter Anbieter: {other}")),
    };
    let token = tauri::async_runtime::spawn_blocking(move || secrets::get(secret))
        .await
        .ok()
        .flatten()
        .ok_or("Kein Token für diesen Anbieter hinterlegt.")?;
    let client = http_client()?;

    match secret {
        Secret::GitHub => {
            github::update(
                &client,
                &token,
                &request.node_id,
                &request.head_sha,
                request.rebase,
            )
            .await
        }
        Secret::Gitea => {
            let host =
                gitea_host(&request.gitea_host).ok_or("Kein gültiger Gitea-Host eingetragen.")?;
            let style = if request.rebase { "rebase" } else { "merge" };
            gitea::update(
                &client,
                &host,
                &token,
                &request.owner,
                &request.repo,
                request.number,
                style,
            )
            .await
        }
    }
}
