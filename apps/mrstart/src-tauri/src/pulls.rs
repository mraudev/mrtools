//! Open pull requests (GitHub and one configurable Gitea host) whose source
//! branch is currently checked out in one of the given project folders.
//!
//! Tokens are read from the credential store here and only ever sent via
//! HTTPS to api.github.com or the configured Gitea host – never to a host
//! derived from repository data alone, and never across redirects.

use crate::git::git_output;
use crate::secrets::{self, Secret};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

/// Host names: letters, digits, dots and dashes only.
fn is_host(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
}

/// Owner/repository names as used by GitHub and Gitea. Rejects anything that
/// could alter the API URL (slashes, `..`, `?`, `#`, …).
pub(crate) fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// HTTP client for all API calls: HTTPS only and no redirects, so a token can
/// never be carried to another URL.
pub(crate) fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("mrstart")
        .timeout(Duration::from_secs(15))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())
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

/// The configured Gitea host, if it is set and syntactically valid.
pub(crate) fn gitea_host(raw: &str) -> Option<String> {
    let host = raw.trim().to_lowercase();
    is_host(&host).then_some(host)
}

/// Only github.com and the configured Gitea host are ever contacted or linked.
fn is_supported(remote: &Remote, gitea_host: Option<&str>) -> bool {
    remote.host == "github.com" || Some(remote.host.as_str()) == gitea_host
}

/// Whether the `origin` remote of `path` is `host/owner/repo`.
pub(crate) fn origin_matches(path: &str, host: &str, owner: &str, repo: &str) -> bool {
    git_output(path, &["remote", "get-url", "origin"])
        .and_then(|url| parse_remote(&url))
        .is_some_and(|remote| {
            remote.host == host
                && remote.owner.eq_ignore_ascii_case(owner)
                && remote.repo.eq_ignore_ascii_case(repo)
        })
}

/// Percent-encodes everything except unreserved characters and `keep`
/// (like JavaScript's `encodeURIComponent` when `keep` is empty).
pub(crate) fn percent_encode(value: &str, keep: &[u8]) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) || keep.contains(&byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// Percent-encodes a branch name for use in a URL path (`/` is kept).
fn encode_path(value: &str) -> String {
    percent_encode(value, b"/")
}

/// Web page for opening a pull request from `branch` into `base`
/// (same URL scheme on GitHub and Gitea).
fn compare_url(remote: &Remote, base: &str, branch: &str) -> Option<String> {
    if branch.is_empty() || branch == base {
        return None;
    }
    Some(format!(
        "https://{}/{}/{}/compare/{}...{}?expand=1",
        remote.host,
        remote.owner,
        remote.repo,
        encode_path(base),
        encode_path(branch)
    ))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    /// Empty for a detached HEAD.
    branch: String,
    /// Link to create a pull request for the branch, if the remote is supported.
    create_pull_url: Option<String>,
    /// Comparison with the upstream branch as of the last fetch; `None` if
    /// the branch has no upstream.
    upstream: Option<Upstream>,
    /// Last fetch from a remote (modification time of FETCH_HEAD), in ms since 1970.
    fetched_at: Option<u64>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Upstream {
    /// e.g. `origin/main`
    name: String,
    /// Local commits not on the upstream yet (to push).
    ahead: u32,
    /// Upstream commits not in the local branch yet (to pull).
    behind: u32,
}

/// Parses `git rev-list --left-right --count HEAD...@{u}` ("<ahead>\t<behind>").
fn parse_ahead_behind(output: &str) -> Option<(u32, u32)> {
    let mut counts = output.split_whitespace().map(str::parse::<u32>);
    match (counts.next(), counts.next(), counts.next()) {
        (Some(Ok(ahead)), Some(Ok(behind)), None) => Some((ahead, behind)),
        _ => None,
    }
}

fn upstream(path: &str) -> Option<Upstream> {
    let name = git_output(
        path,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )?;
    let counts = git_output(
        path,
        &["rev-list", "--left-right", "--count", "HEAD...@{u}"],
    )?;
    let (ahead, behind) = parse_ahead_behind(&counts)?;
    Some(Upstream {
        name,
        ahead,
        behind,
    })
}

fn fetched_at(path: &str) -> Option<u64> {
    let file = git_output(path, &["rev-parse", "--git-path", "FETCH_HEAD"])?;
    let modified = std::fs::metadata(std::path::Path::new(path).join(file))
        .ok()?
        .modified()
        .ok()?;
    let millis = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();
    u64::try_from(millis).ok()
}

/// Checked-out branch of `path`; `None` if it is not a git repository.
#[tauri::command]
pub async fn branch_info(path: String, gitea_host: String) -> Option<BranchInfo> {
    tauri::async_runtime::spawn_blocking(move || {
        let branch = git_output(&path, &["branch", "--show-current"])?;
        let create_pull_url = (|| {
            let remote = parse_remote(&git_output(&path, &["remote", "get-url", "origin"])?)?;
            if !is_supported(&remote, self::gitea_host(&gitea_host).as_deref()) {
                return None;
            }
            // Default branch of the remote, as recorded by `git clone`.
            let base = git_output(
                &path,
                &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
            )
            .and_then(|head| head.strip_prefix("origin/").map(str::to_string))
            .unwrap_or_else(|| "main".to_string());
            compare_url(&remote, &base, &branch)
        })();
        Some(BranchInfo {
            upstream: if branch.is_empty() {
                None
            } else {
                upstream(&path)
            },
            fetched_at: fetched_at(&path),
            branch,
            create_pull_url,
        })
    })
    .await
    .ok()
    .flatten()
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
    /// Project folders that have this branch checked out.
    paths: Vec<String>,
}

/// A failed lookup for one remote, reported on the tiles of `paths`.
#[derive(Serialize)]
pub struct PullRequestError {
    paths: Vec<String>,
    message: String,
}

#[derive(Serialize, Default)]
pub struct PullRequestResult {
    pulls: Vec<PullRequest>,
    errors: Vec<PullRequestError>,
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
    // An invalid host is ignored here; the settings page flags it.
    let gitea_host = gitea_host(&query.gitea_host);

    // Resolve the checked-out branch and remote of every folder in parallel
    // and read the tokens (blocking calls, kept off the async runtime).
    let paths = query.paths;
    let (checkouts, github_token, gitea_token) = tauri::async_runtime::spawn_blocking(move || {
        let checkouts: Vec<(Remote, String, String)> = std::thread::scope(|scope| {
            let handles: Vec<_> = paths
                .iter()
                .map(|path| {
                    scope.spawn(move || {
                        let branch = git_output(path, &["branch", "--show-current"])
                            .filter(|b| !b.is_empty())?;
                        let remote =
                            parse_remote(&git_output(path, &["remote", "get-url", "origin"])?)?;
                        Some((remote, branch, path.clone()))
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

    // remote -> branch -> project folders with that branch checked out
    let mut branches_by_remote: BTreeMap<Remote, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    for (remote, branch, path) in checkouts {
        if is_supported(&remote, gitea_host.as_deref()) {
            branches_by_remote
                .entry(remote)
                .or_default()
                .entry(branch)
                .or_default()
                .push(path);
        }
    }

    let client = match http_client() {
        Ok(client) => client,
        Err(e) => {
            result.errors.push(PullRequestError {
                paths: branches_by_remote
                    .into_values()
                    .flat_map(|b| b.into_values().flatten())
                    .collect(),
                message: e.to_string(),
            });
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
                    .filter(|pull| pull.html_url.starts_with(&link_prefix))
                    .filter_map(|pull| {
                        let paths = branches.get(&pull.head.branch)?.clone();
                        Some(PullRequest {
                            repo: remote.repo.clone(),
                            branch: pull.head.branch,
                            number: pull.number,
                            title: pull.title,
                            url: pull.html_url,
                            paths,
                        })
                    }),
            ),
            Err(e) => result.errors.push(PullRequestError {
                paths: branches.into_values().flatten().collect(),
                message: format!("{}/{}/{}: {e}", remote.host, remote.owner, remote.repo),
            }),
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
    fn parses_ahead_behind_counts() {
        assert_eq!(parse_ahead_behind("2\t3"), Some((2, 3)));
        assert_eq!(parse_ahead_behind("0\t0\n"), Some((0, 0)));
        assert_eq!(parse_ahead_behind("x"), None);
    }

    /// Real git repositories: a bare remote and two clones.
    #[test]
    fn compares_head_with_upstream() {
        use std::path::Path;
        use std::process::{Command, Stdio};

        let root = std::env::temp_dir().join(format!("mrstart-upstream-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let git = |dir: &Path, args: &[&str]| {
            let ok = Command::new("git")
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
                .args(args)
                .current_dir(dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        };
        let state =
            |dir: &Path| upstream(dir.to_str().unwrap()).map(|u| (u.name, u.ahead, u.behind));

        git(&root, &["init", "--bare", "-b", "main", "remote.git"]);
        git(&root, &["clone", "remote.git", "a"]);
        let a = root.join("a");
        git(&a, &["checkout", "-B", "main"]);
        git(&a, &["commit", "--allow-empty", "-m", "eins"]);
        git(&a, &["push", "-u", "origin", "main"]);
        assert_eq!(state(&a), Some(("origin/main".into(), 0, 0)));

        git(&a, &["commit", "--allow-empty", "-m", "zwei"]);
        assert_eq!(state(&a), Some(("origin/main".into(), 1, 0)));

        // Someone else pushes; after a fetch both sides have new commits.
        git(&root, &["clone", "remote.git", "b"]);
        let b = root.join("b");
        git(&b, &["commit", "--allow-empty", "-m", "drei"]);
        git(&b, &["push", "origin", "main"]);
        git(&a, &["fetch"]);
        assert_eq!(state(&a), Some(("origin/main".into(), 1, 1)));
        assert!(fetched_at(a.to_str().unwrap()).is_some());

        git(&a, &["checkout", "-b", "feature"]);
        assert_eq!(state(&a), None);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn builds_encoded_compare_urls() {
        let github = remote("github.com", "mraudev", "mrstart").unwrap();
        assert_eq!(
            compare_url(&github, "main", "feature/neu#1").as_deref(),
            Some("https://github.com/mraudev/mrstart/compare/main...feature/neu%231?expand=1")
        );
        assert_eq!(compare_url(&github, "main", "main"), None);
        assert_eq!(compare_url(&github, "main", ""), None);
    }

    #[test]
    fn only_github_and_the_configured_gitea_host_are_supported() {
        let gitea = remote("gitea.example.com", "team", "app").unwrap();
        assert!(is_supported(&gitea, Some("gitea.example.com")));
        assert!(!is_supported(&gitea, None));
        assert_eq!(
            gitea_host(" Gitea.Example.com "),
            Some("gitea.example.com".into())
        );
        assert_eq!(gitea_host("evil.com/path?"), None);
        assert_eq!(gitea_host(""), None);
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
