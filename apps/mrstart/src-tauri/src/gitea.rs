//! Gitea part of the dashboard (REST API v1). `host` is always the validated
//! host from the settings; owner/repo are validated before use in a URL.

use crate::dashboard::{is_sha, DashboardPull, Lists, Status};
use crate::pulls::is_name;
use serde::Deserialize;

#[derive(Deserialize)]
struct Issue {
    number: u64,
    title: String,
    html_url: String,
    updated_at: String,
    created_at: String,
    user: Option<User>,
    repository: Option<RepositoryMeta>,
}

#[derive(Deserialize)]
struct User {
    login: String,
}

#[derive(Deserialize)]
struct RepositoryMeta {
    name: String,
    owner: String,
}

#[derive(Deserialize)]
struct Pull {
    mergeable: bool,
    draft: Option<bool>,
    merge_base: Option<String>,
    head: Branch,
    base: Branch,
}

#[derive(Deserialize)]
struct Branch {
    #[serde(rename = "ref")]
    name: String,
    sha: String,
}

fn status(pull: &Pull) -> Status {
    // Behind: the merge base is no longer the tip of the base branch, i.e.
    // the base has commits the pull request branch does not contain yet.
    let behind = !pull.base.sha.is_empty()
        && pull
            .merge_base
            .as_deref()
            .is_some_and(|base| !base.is_empty() && base != pull.base.sha);
    if behind {
        Status::Behind
    } else if !pull.mergeable {
        Status::Conflict
    } else if pull.draft == Some(true) {
        Status::Draft
    } else {
        Status::Clean
    }
}

async fn get<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    token: &str,
    url: String,
) -> Result<T, String> {
    let response = client
        .get(url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 => "Token ungültig oder abgelaufen".into(),
            403 => "keine Berechtigung (Token-Rechte prüfen)".into(),
            _ => format!("HTTP {status}"),
        });
    }
    response.json().await.map_err(|e| e.to_string())
}

/// Open pull requests across all repositories, filtered by `filter`
/// (`created` or `review_requested`).
async fn search(
    client: &reqwest::Client,
    host: &str,
    token: &str,
    filter: &str,
) -> Result<Vec<Issue>, String> {
    let url = format!(
        "https://{host}/api/v1/repos/issues/search?type=pulls&state=open&{filter}=true&limit=50"
    );
    get(client, token, url).await
}

/// Builds a dashboard entry from a search hit; `None` if its data is unsafe.
fn entry(host: &str, issue: Issue, pull: Option<&Pull>) -> Option<DashboardPull> {
    let repository = issue.repository?;
    let link_ok = issue.html_url.starts_with(&format!("https://{host}/"));
    if !link_ok || !is_name(&repository.owner) || !is_name(&repository.name) {
        return None;
    }
    let status = pull.map(status).unwrap_or(Status::Unknown);
    Some(DashboardPull {
        provider: "gitea",
        node_id: String::new(),
        owner: repository.owner,
        repo: repository.name,
        number: issue.number,
        title: issue.title,
        url: issue.html_url,
        is_draft: pull.and_then(|p| p.draft).unwrap_or(false),
        updated_at: issue.updated_at,
        created_at: issue.created_at,
        head: pull.map(|p| p.head.name.clone()).unwrap_or_default(),
        base: pull.map(|p| p.base.name.clone()).unwrap_or_default(),
        head_sha: pull
            .map(|p| p.head.sha.clone())
            .filter(|sha| is_sha(sha))
            .unwrap_or_default(),
        status,
        can_update: status == Status::Behind,
        review_decision: None,
        author: issue.user.map(|u| u.login).unwrap_or_default(),
    })
}

pub async fn fetch(client: &reqwest::Client, host: &str, token: &str) -> Result<Lists, String> {
    let authored = search(client, host, token, "created").await?;
    let reviews = search(client, host, token, "review_requested").await?;

    // The search result has no merge information; load each own PR in parallel.
    let details: Vec<_> = authored
        .iter()
        .map(|issue| {
            let repo = issue
                .repository
                .as_ref()
                .filter(|r| is_name(&r.owner) && is_name(&r.name))
                .map(|r| (r.owner.clone(), r.name.clone()));
            let (client, host, token, number) = (
                client.clone(),
                host.to_string(),
                token.to_string(),
                issue.number,
            );
            tauri::async_runtime::spawn(async move {
                let (owner, name) = repo?;
                let url = format!("https://{host}/api/v1/repos/{owner}/{name}/pulls/{number}");
                get::<Pull>(&client, &token, url).await.ok()
            })
        })
        .collect();

    let mut authored_entries = Vec::new();
    for (issue, detail) in authored.into_iter().zip(details) {
        let pull = detail.await.ok().flatten();
        authored_entries.extend(entry(host, issue, pull.as_ref()));
    }
    let review_entries = reviews
        .into_iter()
        .filter_map(|issue| entry(host, issue, None))
        .collect();
    Ok((authored_entries, review_entries))
}

/// Gitea's "Update branch": merges (or rebases onto) the base branch.
pub async fn update(
    client: &reqwest::Client,
    host: &str,
    token: &str,
    owner: &str,
    repo: &str,
    number: u64,
    style: &str,
) -> Result<(), String> {
    let response = client
        .post(format!(
            "https://{host}/api/v1/repos/{owner}/{repo}/pulls/{number}/update?style={style}"
        ))
        .header("Authorization", format!("token {token}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    Err(match status.as_u16() {
        403 => "Keine Berechtigung, diesen Branch zu aktualisieren.".into(),
        409 => "Konflikt – der Branch lässt sich nicht automatisch aktualisieren.".into(),
        422 => {
            "Gitea lehnt die Aktualisierung ab (z. B. Rebase im Repository nicht erlaubt).".into()
        }
        _ => format!("Gitea antwortet mit HTTP {status}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pull(merge_base: &str, base_sha: &str, mergeable: bool) -> Pull {
        Pull {
            mergeable,
            draft: None,
            merge_base: Some(merge_base.into()),
            head: Branch {
                name: "feature".into(),
                sha: "h".repeat(40),
            },
            base: Branch {
                name: "main".into(),
                sha: base_sha.into(),
            },
        }
    }

    #[test]
    fn detects_behind_and_conflicts() {
        assert_eq!(status(&pull("aaa", "bbb", true)), Status::Behind);
        assert_eq!(status(&pull("bbb", "bbb", true)), Status::Clean);
        assert_eq!(status(&pull("bbb", "bbb", false)), Status::Conflict);
    }

    #[test]
    fn drops_entries_with_foreign_links_or_bad_names() {
        let issue = |url: &str, owner: &str| Issue {
            number: 1,
            title: "t".into(),
            html_url: url.into(),
            updated_at: String::new(),
            created_at: String::new(),
            user: None,
            repository: Some(RepositoryMeta {
                name: "app".into(),
                owner: owner.into(),
            }),
        };
        let host = "gitea.example.com";
        assert!(entry(
            host,
            issue("https://gitea.example.com/team/app/pulls/1", "team"),
            None
        )
        .is_some());
        assert!(entry(
            host,
            issue("https://evil.example/team/app/pulls/1", "team"),
            None
        )
        .is_none());
        assert!(entry(
            host,
            issue("https://gitea.example.com/x/app/pulls/1", "a/b"),
            None
        )
        .is_none());
    }
}
