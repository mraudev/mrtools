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
  id number title url isDraft updatedAt headRefName baseRefName headRefOid
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
    head_ref_name: String,
    base_ref_name: String,
    head_ref_oid: String,
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
            head: pr.head_ref_name,
            base: pr.base_ref_name,
            head_sha: pr.head_ref_oid,
            status: status(&pr.merge_state_status),
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

pub async fn fetch(client: &reqwest::Client, token: &str) -> Result<Lists, String> {
    let mut data = graphql(client, token, json!({ "query": QUERY })).await?;
    let parse = |value: Value| serde_json::from_value::<Search>(value).map_err(|e| e.to_string());
    Ok((
        to_dashboard(parse(data["authored"].take())?),
        to_dashboard(parse(data["reviews"].take())?),
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
            head_ref_name: "feature".into(),
            base_ref_name: "main".into(),
            head_ref_oid: sha.into(),
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
