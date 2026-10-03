//! Open pull requests (GitHub and one configurable Gitea host) whose source
//! branch is currently checked out in one of the given project folders.
//!
//! Tokens are read from the credential store here and only ever sent via
//! HTTPS to api.github.com or the configured Gitea host – never to a host
//! derived from repository data alone, and never across redirects.

use crate::git::git_output;
use crate::secrets::{self, Secret};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// Host names: letters, digits, dots and dashes only.
fn is_host(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
}

/// Owner/repository names as used by GitHub and Gitea. Rejects anything that
/// could alter the API URL (slashes, `..`, `?`, `#`, …).
fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

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

    let (owner, repo) = path.trim_matches('/').split_once('/')?;
    if !is_host(host) || !is_name(owner) || !is_name(repo) {
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
    gitea_host: String,
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
    token: Option<&str>,
) -> Result<Vec<ApiPull>, String> {
    let request = if remote.host == "github.com" {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls?state=open&per_page=100",
            remote.owner, remote.repo
        );
        let request = client
            .get(url)
            .header("Accept", "application/vnd.github+json");
        match token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    } else {
        let url = format!(
            "https://{}/api/v1/repos/{}/{}/pulls?state=open&limit=50",
            remote.host, remote.owner, remote.repo
        );
        let request = client.get(url).header("Accept", "application/json");
        match token {
            Some(token) => request.header("Authorization", format!("token {token}")),
            None => request,
        }
    };
    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        let hint = match status.as_u16() {
            301 | 302 | 307 | 308 => " – Repository umgezogen? Remote-URL aktualisieren",
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
    let mut result = PullRequestResult::default();

    // Only github.com and the configured Gitea host are queried.
    let mut gitea_host = query.gitea_host.trim().to_lowercase();
    if !gitea_host.is_empty() && !is_host(&gitea_host) {
        result
            .errors
            .push(format!("Ungültiger Gitea-Host: {gitea_host}"));
        gitea_host.clear();
    }

    // Resolve the checked-out branch and remote of every folder in parallel
    // and read the tokens (blocking calls, kept off the async runtime).
    let paths = query.paths;
    let (checkouts, github_token, gitea_token) = tauri::async_runtime::spawn_blocking(move || {
        let checkouts: Vec<(Remote, String)> = std::thread::scope(|scope| {
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
        });
        (
            checkouts,
            secrets::get(Secret::GitHub),
            secrets::get(Secret::Gitea),
        )
    })
    .await
    .unwrap_or_default();

    let mut branches_by_remote: BTreeMap<Remote, BTreeSet<String>> = BTreeMap::new();
    for (remote, branch) in checkouts {
        if remote.host == "github.com" || (!gitea_host.is_empty() && remote.host == gitea_host) {
            branches_by_remote.entry(remote).or_default().insert(branch);
        }
    }

    let client = match reqwest::Client::builder()
        .user_agent("mrstart")
        .timeout(Duration::from_secs(10))
        .https_only(true)
        // A redirect must never carry a token to another URL.
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            result.errors.push(e.to_string());
            return result;
        }
    };

    let tasks: Vec<_> = branches_by_remote
        .into_iter()
        .map(|(remote, branches)| {
            let client = client.clone();
            let token = if remote.host == "github.com" {
                github_token.clone()
            } else {
                gitea_token.clone()
            };
            tauri::async_runtime::spawn(async move {
                let response = fetch_open_pulls(&client, &remote, token.as_deref()).await;
                (remote, branches, response)
            })
        })
        .collect();

    for task in tasks {
        let Ok((remote, branches, response)) = task.await else {
            continue;
        };
        // Links are opened in the browser, so they must point to the same host.
        let link_prefix = format!("https://{}/", remote.host);
        match response {
            Ok(pulls) => result.pulls.extend(
                pulls
                    .into_iter()
                    .filter(|pull| branches.contains(&pull.head.branch))
                    .filter(|pull| pull.html_url.starts_with(&link_prefix))
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
            parse_remote("https://user@Code.Example.org:3000/team/app"),
            remote("code.example.org", "team", "app")
        );
    }

    #[test]
    fn rejects_local_paths() {
        assert_eq!(parse_remote("C:/repos/app"), None);
        assert_eq!(parse_remote("/srv/git/app.git"), None);
    }

    #[test]
    fn rejects_names_that_could_alter_the_api_url() {
        assert_eq!(parse_remote("https://github.com/a/../../user"), None);
        assert_eq!(parse_remote("https://github.com/a/b?x=1"), None);
        assert_eq!(parse_remote("https://github.com/a/b#frag"), None);
        assert_eq!(parse_remote("https://github.com/group/sub/app"), None);
        assert_eq!(parse_remote("https://evil.com?.github.com/a/b"), None);
        assert_eq!(parse_remote("git@github.com:../b"), None);
    }
}
