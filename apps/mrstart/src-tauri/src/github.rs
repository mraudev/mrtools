//! GitHub part of the dashboard (GraphQL API).

use crate::dashboard::{is_sha, DashboardPull, Lists, Status};
use crate::pulls::is_name;
use serde::Deserialize;
use serde_json::{json, Value};

const QUERY: &str = r#"
query {
  authored: search(query: "is:pr is:open author:@me archived:false sort:updated-desc", type: ISSUE, first: 50) {
    nodes { ...pr }
  }
  reviews: search(query: "is:pr is:open review-requested:@me archived:false sort:updated-desc", type: ISSUE, first: 50) {
    nodes { ...pr }
  }
}
fragment pr on PullRequest {
  id number title url isDraft updatedAt createdAt headRefName baseRefName headRefOid baseRefOid
  mergeStateStatus viewerCanUpdateBranch reviewDecision
  author { login }
  repository { name owner { login } }
}
"#;

const UPDATE_MUTATION: &str = r#"
mutation($id: ID!, $sha: GitObjectID!, $method: PullRequestBranchUpdateMethod!) {
  updatePullRequestBranch(input: {pullRequestId: $id, expectedHeadOid: $sha, updateMethod: $method}) {
    pullRequest { number }
  }
}
"#;

#[derive(Deserialize)]
struct Search {
    nodes: Vec<Option<Pull>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pull {
    id: String,
    number: u64,
    title: String,
    url: String,
    is_draft: bool,
    updated_at: String,
    created_at: String,
    head_ref_name: String,
    base_ref_name: String,
    head_ref_oid: String,
    base_ref_oid: String,
    merge_state_status: String,
    viewer_can_update_branch: bool,
    review_decision: Option<String>,
    author: Option<Login>,
    repository: Repository,
}

#[derive(Deserialize)]
struct Login {
    login: String,
}

#[derive(Deserialize)]
struct Repository {
    name: String,
    owner: Login,
}

fn status(merge_state: &str) -> Status {
    match merge_state {
        "BEHIND" => Status::Behind,
        "DIRTY" => Status::Conflict,
        "CLEAN" | "HAS_HOOKS" => Status::Clean,
        "BLOCKED" => Status::Blocked,
        "UNSTABLE" => Status::Unstable,
        "DRAFT" => Status::Draft,
        _ => Status::Unknown,
    }
}

/// Drops entries whose data could not safely be used in links or API calls.
fn to_dashboard(search: Search) -> Vec<DashboardPull> {
    search
        .nodes
        .into_iter()
        .flatten()
        .filter(|pr| {
            is_name(&pr.repository.owner.login)
                && is_name(&pr.repository.name)
                && is_sha(&pr.head_ref_oid)
                && pr.url.starts_with("https://github.com/")
        })
        .map(|pr| DashboardPull {
            provider: "github",
            node_id: pr.id,
            owner: pr.repository.owner.login,
            repo: pr.repository.name,
            number: pr.number,
            title: pr.title,
            url: pr.url,
            is_draft: pr.is_draft,
            updated_at: pr.updated_at,
            created_at: pr.created_at,
            head: pr.head_ref_name,
            base: pr.base_ref_name,
            head_sha: pr.head_ref_oid,
            base_sha: pr.base_ref_oid,
            base_date: None,
            // mergeStateStatus only reports BEHIND when branch protection
            // requires up-to-date branches; viewerCanUpdateBranch is reliable.
            status: if pr.viewer_can_update_branch {
                Status::Behind
            } else {
                status(&pr.merge_state_status)
            },
            can_update: pr.viewer_can_update_branch,
            review_decision: pr.review_decision,
            author: pr.author.map(|a| a.login).unwrap_or_default(),
        })
        .collect()
}

/// Sends a GraphQL request and returns `data`, or the reported errors.
async fn graphql(client: &reqwest::Client, token: &str, body: Value) -> Result<Value, String> {
    let response = client
        .post("https://api.github.com/graphql")
        .bearer_auth(token)
        .json(&body)
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
    let mut body: Value = response.json().await.map_err(|e| e.to_string())?;
    if let Some(errors) = body["errors"].as_array().filter(|e| !e.is_empty()) {
        return Err(errors
            .iter()
            .filter_map(|e| e["message"].as_str())
            .collect::<Vec<_>>()
            .join("; "));
    }
    Ok(body["data"].take())
}

/// Commit date of the merge base of `base_sha` and `head_sha`, i.e. the last
/// state of the base branch the pull request branch contains.
async fn merge_base_date(
    client: &reqwest::Client,
    token: &str,
    (owner, repo, base_sha, head_sha): (String, String, String, String),
) -> Option<String> {
    // owner/repo/head were validated in to_dashboard.
    if !is_sha(&base_sha) {
        return None;
    }
    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/compare/{base_sha}...{head_sha}?per_page=1"
    );
    let response = client
        .get(url)
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body: Value = response.json().await.ok()?;
    body["merge_base_commit"]["commit"]["committer"]["date"]
        .as_str()
        .map(str::to_string)
}

/// Fills `base_date` for all pull requests (one compare call each, in parallel).
async fn with_base_dates(
    client: &reqwest::Client,
    token: &str,
    mut pulls: Vec<DashboardPull>,
) -> Vec<DashboardPull> {
    let tasks: Vec<_> = pulls
        .iter()
        .map(|pr| {
            let (client, token) = (client.clone(), token.to_string());
            let refs = (
                pr.owner.clone(),
                pr.repo.clone(),
                pr.base_sha.clone(),
                pr.head_sha.clone(),
            );
            tauri::async_runtime::spawn(async move { merge_base_date(&client, &token, refs).await })
        })
        .collect();
    for (pr, task) in pulls.iter_mut().zip(tasks) {
        pr.base_date = task.await.ok().flatten();
    }
    pulls
}

pub async fn fetch(client: &reqwest::Client, token: &str) -> Result<Lists, String> {
    let mut data = graphql(client, token, json!({ "query": QUERY })).await?;
    let parse = |value: Value| serde_json::from_value::<Search>(value).map_err(|e| e.to_string());
    let authored = to_dashboard(parse(data["authored"].take())?);
    let reviews = to_dashboard(parse(data["reviews"].take())?);
    Ok((
        with_base_dates(client, token, authored).await,
        with_base_dates(client, token, reviews).await,
    ))
}

/// Merges or rebases the base branch into the pull request branch. Fails if
/// the branch moved since `head_sha` was loaded.
pub async fn update(
    client: &reqwest::Client,
    token: &str,
    node_id: &str,
    head_sha: &str,
    rebase: bool,
) -> Result<(), String> {
    let method = if rebase { "REBASE" } else { "MERGE" };
    let body = json!({
        "query": UPDATE_MUTATION,
        "variables": { "id": node_id, "sha": head_sha, "method": method },
    });
    graphql(client, token, body).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pull(owner: &str, repo: &str, sha: &str, url: &str) -> Option<Pull> {
        Some(Pull {
            id: "PR_1".into(),
            number: 1,
            title: "t".into(),
            url: url.into(),
            is_draft: false,
            updated_at: String::new(),
            created_at: String::new(),
            head_ref_name: "feature".into(),
            base_ref_name: "main".into(),
            head_ref_oid: sha.into(),
            base_ref_oid: "b".repeat(40),
            merge_state_status: "BEHIND".into(),
            viewer_can_update_branch: true,
            review_decision: None,
            author: None,
            repository: Repository {
                name: repo.into(),
                owner: Login {
                    login: owner.into(),
                },
            },
        })
    }

    #[test]
    fn keeps_only_entries_that_are_safe_to_use() {
        let sha = "a".repeat(40);
        let url = "https://github.com/o/r/pull/1";
        let nodes = vec![
            pull("o", "r", &sha, url),
            None,
            pull("o", "../x", &sha, url),
            pull("o", "r", "not-a-sha", url),
            pull("o", "r", &sha, "https://evil.example/pull/1"),
        ];
        let kept = to_dashboard(Search { nodes });
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].status, Status::Behind);
    }
}
