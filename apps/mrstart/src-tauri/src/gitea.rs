//! Gitea part of the dashboard (REST API v1). `host` is always the validated
//! host from the settings; owner/repo are validated before use in a URL.

use crate::dashboard::{ci_state, is_sha, web_link, DashboardIssue, DashboardPull, Lists, Status};
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
    #[serde(default)]
    labels: Vec<Label>,
}

#[derive(Deserialize)]
struct Label {
    name: String,
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

/// Behind: the merge base is no longer the tip of the base branch, i.e. the
/// base has commits the pull request branch does not contain yet.
fn is_behind(pull: &Pull) -> bool {
    !pull.base.sha.is_empty()
        && pull
            .merge_base
            .as_deref()
            .is_some_and(|base| !base.is_empty() && base != pull.base.sha)
}

/// Conflicts take precedence: updating the branch would merge the same two
/// states and run into the same conflicts.
fn status(pull: &Pull) -> Status {
    if !pull.mergeable {
        Status::Conflict
    } else if is_behind(pull) {
        Status::Behind
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

/// Open pull requests or issues (`kind`: `pulls`/`issues`) across all
/// repositories, filtered by `filter` (`created`, `review_requested`, `assigned`).
async fn search(
    client: &reqwest::Client,
    host: &str,
    token: &str,
    kind: &str,
    filter: &str,
) -> Result<Vec<Issue>, String> {
    let url = format!(
        "https://{host}/api/v1/repos/issues/search?type={kind}&state=open&{filter}=true&limit=50"
    );
    get(client, token, url).await
}

/// An assigned issue; `None` if its data is unsafe to link.
fn issue_entry(host: &str, issue: Issue) -> Option<DashboardIssue> {
    let repository = issue.repository?;
    if !issue.html_url.starts_with(&format!("https://{host}/"))
        || !is_name(&repository.owner)
        || !is_name(&repository.name)
    {
        return None;
    }
    Some(DashboardIssue {
        provider: "gitea",
        owner: repository.owner,
        repo: repository.name,
        number: issue.number,
        title: issue.title,
        url: issue.html_url,
        created_at: issue.created_at,
        updated_at: issue.updated_at,
        author: issue.user.map(|u| u.login).unwrap_or_default(),
        labels: issue.labels.into_iter().map(|l| l.name).collect(),
    })
}

#[derive(Deserialize)]
struct CommitInfo {
    commit: CommitMeta,
}

#[derive(Deserialize)]
struct CommitMeta {
    committer: CommitUser,
}

#[derive(Deserialize)]
struct CommitUser {
    date: String,
}

/// Combined commit status (Gitea Actions or external CI).
#[derive(Deserialize)]
struct CombinedStatus {
    state: Option<String>,
    #[serde(default)]
    statuses: Vec<CommitStatus>,
}

#[derive(Deserialize)]
struct CommitStatus {
    target_url: Option<String>,
}

#[derive(Default)]
struct Details {
    pull: Option<Pull>,
    base_date: Option<String>,
    ci: Option<&'static str>,
    ci_url: Option<String>,
}

/// Starts loading the merge information and the merge base date of each pull
/// request (the search result contains neither).
fn spawn_details(
    client: &reqwest::Client,
    host: &str,
    token: &str,
    issues: &[Issue],
) -> Vec<tauri::async_runtime::JoinHandle<Details>> {
    issues
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
                let Some((owner, name)) = repo else {
                    return Details::default();
                };
                let api = format!("https://{host}/api/v1/repos/{owner}/{name}");
                let Ok(pull) = get::<Pull>(&client, &token, format!("{api}/pulls/{number}")).await
                else {
                    return Details::default();
                };
                let (ci, ci_url) = if is_sha(&pull.head.sha) {
                    let url = format!("{api}/commits/{}/status", pull.head.sha);
                    match get::<CombinedStatus>(&client, &token, url).await {
                        Ok(s) if !s.statuses.is_empty() => (
                            s.state.as_deref().and_then(ci_state),
                            s.statuses
                                .iter()
                                .find_map(|st| st.target_url.as_deref().and_then(web_link)),
                        ),
                        _ => (None, None),
                    }
                } else {
                    (None, None)
                };
                let base_date = match pull.merge_base.as_deref().filter(|sha| is_sha(sha)) {
                    Some(sha) => get::<CommitInfo>(
                        &client,
                        &token,
                        format!(
                            "{api}/git/commits/{sha}?stat=false&verification=false&files=false"
                        ),
                    )
                    .await
                    .ok()
                    .map(|c| c.commit.committer.date),
                    None => None,
                };
                Details {
                    pull: Some(pull),
                    base_date,
                    ci,
                    ci_url,
                }
            })
        })
        .collect()
}

/// Builds a dashboard entry from a search hit; `None` if its data is unsafe.
fn entry(host: &str, issue: Issue, details: Details) -> Option<DashboardPull> {
    let Details {
        pull,
        base_date,
        ci,
        ci_url,
    } = details;
    let pull = pull.as_ref();
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
        base_sha: pull.map(|p| p.base.sha.clone()).unwrap_or_default(),
        base_date,
        status,
        can_update: pull.is_some_and(is_behind),
        has_conflicts: status == Status::Conflict,
        ci,
        ci_url,
        review_decision: None,
        author: issue.user.map(|u| u.login).unwrap_or_default(),
    })
}

pub async fn fetch(client: &reqwest::Client, host: &str, token: &str) -> Result<Lists, String> {
    let authored = search(client, host, token, "pulls", "created").await?;
    let reviews = search(client, host, token, "pulls", "review_requested").await?;
    let issues = search(client, host, token, "issues", "assigned").await?;

    // Both lists load their details in parallel.
    let authored_details = spawn_details(client, host, token, &authored);
    let review_details = spawn_details(client, host, token, &reviews);

    let mut authored_entries = Vec::new();
    for (issue, task) in authored.into_iter().zip(authored_details) {
        authored_entries.extend(entry(host, issue, task.await.unwrap_or_default()));
    }
    let mut review_entries = Vec::new();
    for (issue, task) in reviews.into_iter().zip(review_details) {
        review_entries.extend(entry(host, issue, task.await.unwrap_or_default()));
    }
    Ok(Lists {
        authored: authored_entries,
        reviews: review_entries,
        issues: issues
            .into_iter()
            .filter_map(|issue| issue_entry(host, issue))
            .collect(),
    })
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
        // Behind and conflicting: updating would not work without resolving.
        assert_eq!(status(&pull("aaa", "bbb", false)), Status::Conflict);
        assert!(is_behind(&pull("aaa", "bbb", false)));
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
            labels: vec![],
        };
        let host = "gitea.example.com";
        assert!(entry(
            host,
            issue("https://gitea.example.com/team/app/pulls/1", "team"),
            Details::default()
        )
        .is_some());
        assert!(entry(
            host,
            issue("https://evil.example/team/app/pulls/1", "team"),
            Details::default()
        )
        .is_none());
        assert!(entry(
            host,
            issue("https://gitea.example.com/x/app/pulls/1", "a/b"),
            Details::default()
        )
        .is_none());
    }
}
