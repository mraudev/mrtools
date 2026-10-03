//! Open pull requests (GitHub and one configurable Gitea host) whose source
//! branch is currently checked out in one of the given project folders.

use crate::git::git_output;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct Remote {
    host: String,
    owner: String,
    repo: String,
}

/// Parses `git@host:owner/repo(.git)` and `http(s)://host[:port]/owner/repo(.git)`.
fn parse_remote(url: &str) -> Option<Remote> {
    let url = url.trim();
    let url = url.strip_suffix(".git").unwrap_or(url);

    let (host, path) = if let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    {
        let (host, path) = rest.split_once('/')?;
        let host = host.rsplit('@').next()?; // drop credentials
        (host.split(':').next()?, path)
    } else if let Some(rest) = url.strip_prefix("ssh://") {
        let (host, path) = rest.split_once('/')?;
        let host = host.rsplit('@').next()?;
        (host.split(':').next()?, path)
    } else {
        // scp-like syntax; an absolute path after the colon means a local
        // path such as `C:/repos/app`.
        let (user_host, path) = url.split_once(':')?;
        if path.starts_with(['/', '\\']) {
            return None;
        }
        (user_host.rsplit('@').next()?, path)
    };

    let (owner, repo) = path.trim_matches('/').rsplit_once('/')?;
    if host.is_empty() || owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(Remote {
        host: host.to_lowercase(),
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestQuery {
    paths: Vec<String>,
    github_token: String,
    gitea_host: String,
    gitea_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    repo: String,
    branch: String,
    number: u64,
    title: String,
    url: String,
}

#[derive(Serialize, Default)]
pub struct PullRequestResult {
    pulls: Vec<PullRequest>,
    errors: Vec<String>,
}

#[derive(Deserialize)]
struct ApiPull {
    number: u64,
    title: String,
    html_url: String,
    head: ApiHead,
}

#[derive(Deserialize)]
struct ApiHead {
    #[serde(rename = "ref")]
    branch: String,
}

async fn fetch_open_pulls(
    client: &reqwest::Client,
    remote: &Remote,
    query: &PullRequestQuery,
) -> Result<Vec<ApiPull>, String> {
    let request = if remote.host == "github.com" {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls?state=open&per_page=100",
            remote.owner, remote.repo
        );
        let request = client
            .get(url)
            .header("Accept", "application/vnd.github+json");
        match query.github_token.trim() {
            "" => request,
            token => request.bearer_auth(token),
        }
    } else {
        let url = format!(
            "https://{}/api/v1/repos/{}/{}/pulls?state=open&limit=50",
            remote.host, remote.owner, remote.repo
        );
        let request = client.get(url).header("Accept", "application/json");
        match query.gitea_token.trim() {
            "" => request,
            token => request.header("Authorization", format!("token {token}")),
        }
    };
    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        let hint = match status.as_u16() {
            401 | 404 => " – privates Repository? Token in den Einstellungen hinterlegen",
            403 | 429 => " – Rate-Limit? Token in den Einstellungen hinterlegen",
            _ => "",
        };
        return Err(format!("HTTP {status}{hint}"));
    }
    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pull_requests(query: PullRequestQuery) -> PullRequestResult {
    // Resolve the checked-out branch and remote of every folder in parallel.
    let paths = query.paths.clone();
    let checkouts: Vec<(Remote, String)> = tauri::async_runtime::spawn_blocking(move || {
        std::thread::scope(|scope| {
            let handles: Vec<_> = paths
                .iter()
                .map(|path| {
                    scope.spawn(move || {
                        let branch = git_output(path, &["branch", "--show-current"])
                            .filter(|b| !b.is_empty())?;
                        let remote =
                            parse_remote(&git_output(path, &["remote", "get-url", "origin"])?)?;
                        Some((remote, branch))
                    })
                })
                .collect();
            handles
                .into_iter()
                .filter_map(|h| h.join().ok().flatten())
                .collect()
        })
    })
    .await
    .unwrap_or_default();

    // Only github.com and the configured Gitea host are queried.
    let gitea_host = query.gitea_host.trim().to_lowercase();
    let mut branches_by_remote: BTreeMap<Remote, BTreeSet<String>> = BTreeMap::new();
    for (remote, branch) in checkouts {
        if remote.host == "github.com" || (!gitea_host.is_empty() && remote.host == gitea_host) {
            branches_by_remote.entry(remote).or_default().insert(branch);
        }
    }

    let client = match reqwest::Client::builder()
        .user_agent("mrstart")
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return PullRequestResult {
                pulls: vec![],
                errors: vec![e.to_string()],
            }
        }
    };

    let query = std::sync::Arc::new(query);
    let tasks: Vec<_> = branches_by_remote
        .into_iter()
        .map(|(remote, branches)| {
            let client = client.clone();
            let query = query.clone();
            tauri::async_runtime::spawn(async move {
                let result = fetch_open_pulls(&client, &remote, &query).await;
                (remote, branches, result)
            })
        })
        .collect();

    let mut result = PullRequestResult::default();
    for task in tasks {
        let Ok((remote, branches, response)) = task.await else {
            continue;
        };
        match response {
            Ok(pulls) => result.pulls.extend(
                pulls
                    .into_iter()
                    .filter(|pull| branches.contains(&pull.head.branch))
                    .map(|pull| PullRequest {
                        repo: remote.repo.clone(),
                        branch: pull.head.branch,
                        number: pull.number,
                        title: pull.title,
                        url: pull.html_url,
                    }),
            ),
            Err(e) => result.errors.push(format!(
                "{}/{}/{}: {e}",
                remote.host, remote.owner, remote.repo
            )),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(host: &str, owner: &str, repo: &str) -> Option<Remote> {
        Some(Remote {
            host: host.into(),
            owner: owner.into(),
            repo: repo.into(),
        })
    }

    #[test]
    fn parses_common_remote_formats() {
        assert_eq!(
            parse_remote("https://github.com/mraudev/mrstart.git"),
            remote("github.com", "mraudev", "mrstart")
        );
        assert_eq!(
            parse_remote("git@github.com:mraudev/mrstart.git\n"),
            remote("github.com", "mraudev", "mrstart")
        );
        assert_eq!(
            parse_remote("ssh://git@code.example.org:2222/team/app.git"),
            remote("code.example.org", "team", "app")
        );
        assert_eq!(
            parse_remote("https://user@Code.Example.org:3000/group/sub/app"),
            remote("code.example.org", "group/sub", "app")
        );
    }

    #[test]
    fn rejects_local_paths() {
        assert_eq!(parse_remote("C:/repos/app"), None);
        assert_eq!(parse_remote("/srv/git/app.git"), None);
    }
}
