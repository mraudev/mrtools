//! Scans the app folder: every subfolder is one app, described by its
//! `package.json`, `src-tauri/tauri.conf.json` and the `origin` git remote.
//! A subfolder with npm workspaces is a monorepo; its Tauri apps are listed
//! individually and released with tags `<folder>-v<version>`.

use crate::installed::{self, Installed};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct App {
    /// Folder name.
    folder: String,
    path: String,
    /// Product name (as shown in Windows' installed apps).
    name: String,
    description: Option<String>,
    /// Version in the source folder.
    version: Option<String>,
    /// `owner/repo` on github.com.
    github: Option<String>,
    /// Folder inside the repository (apps in a monorepo).
    repo_path: Option<String>,
    /// Prefix of this app's release tags (apps in a monorepo), e.g. `mrprocs-v`.
    tag_prefix: Option<String>,
    /// Logo as data URL.
    icon: Option<String>,
    installed: Option<Installed>,
}

/// The folder that contains the mrtools repository in development builds
/// (`<root>/mrtools/apps/mrtools/src-tauri`).
#[tauri::command]
pub fn default_root() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[tauri::command]
pub async fn scan(root: String) -> Result<Vec<App>, String> {
    tauri::async_runtime::spawn_blocking(move || scan_blocking(&root))
        .await
        .map_err(|e| e.to_string())?
}

fn scan_blocking(root: &str) -> Result<Vec<App>, String> {
    // Report an unreadable folder instead of showing it as empty.
    fs::read_dir(root).map_err(|e| format!("{root}: {e}"))?;
    let installed = installed::all();
    let mut apps = Vec::new();
    for dir in subfolders(Path::new(root)) {
        let package = read_json(&dir.join("package.json")).unwrap_or(Value::Null);
        match package["workspaces"].as_array() {
            Some(workspaces) => {
                let repo = Repo { root: &dir, github: github_of(&dir, &package) };
                for member in workspace_members(&dir, workspaces) {
                    if member.join("src-tauri").join("tauri.conf.json").is_file() {
                        apps.push(read_app(&member, Some(&repo), &installed));
                    }
                }
            }
            None => apps.push(read_app(&dir, None, &installed)),
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    Ok(apps)
}

/// The monorepo an app belongs to.
struct Repo<'a> {
    root: &'a Path,
    github: Option<String>,
}

fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs
}

/// Folders matched by npm `workspaces` entries (`apps/*` or `tools/app`).
fn workspace_members(root: &Path, workspaces: &[Value]) -> Vec<PathBuf> {
    workspaces
        .iter()
        .filter_map(Value::as_str)
        .filter(|w| !w.contains(".."))
        .flat_map(|w| match w.strip_suffix("/*") {
            Some(parent) => subfolders(&root.join(parent)),
            None => vec![root.join(w)],
        })
        .collect()
}

/// `owner/repo` from the `origin` remote or `repository` in package.json.
fn github_of(dir: &Path, package: &Value) -> Option<String> {
    let repository = text(&package["repository"]).or_else(|| text(&package["repository"]["url"]));
    origin_url(dir)
        .as_deref()
        .and_then(github_repo)
        .or_else(|| repository.as_deref().and_then(github_repo))
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn text(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn read_app(dir: &Path, repo: Option<&Repo>, installed: &[Installed]) -> App {
    let folder = dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let package = read_json(&dir.join("package.json")).unwrap_or(Value::Null);
    let tauri = read_json(&dir.join("src-tauri").join("tauri.conf.json")).unwrap_or(Value::Null);

    let name = text(&tauri["productName"])
        .or_else(|| text(&package["productName"]))
        .or_else(|| text(&package["build"]["productName"]))
        .or_else(|| text(&package["name"]))
        .unwrap_or_else(|| folder.clone());

    // tauri.conf.json usually points to package.json ("../package.json").
    let version = text(&tauri["version"])
        .filter(|v| !v.ends_with(".json"))
        .or_else(|| text(&package["version"]));

    let github = match repo {
        Some(repo) => repo.github.clone(),
        None => github_of(dir, &package),
    };
    let repo_path = repo.and_then(|r| dir.strip_prefix(r.root).ok()).map(|p| {
        p.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    });
    let tag_prefix = repo.map(|_| format!("{folder}-v"));

    let installed = installed
        .iter()
        .find(|i| i.name.eq_ignore_ascii_case(&name) || i.name.eq_ignore_ascii_case(&folder))
        .cloned();

    App {
        path: dir.to_string_lossy().into_owned(),
        description: text(&package["description"]),
        icon: icon(dir),
        folder,
        name,
        version,
        github,
        repo_path,
        tag_prefix,
        installed,
    }
}

/// URL of the `origin` remote, read from `.git/config` without running git.
fn origin_url(dir: &Path) -> Option<String> {
    let config = fs::read_to_string(dir.join(".git").join("config")).ok()?;
    let mut in_origin = false;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            in_origin = line == r#"[remote "origin"]"#;
        } else if in_origin {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "url" {
                    return Some(value.trim().to_string());
                }
            }
        }
    }
    None
}

/// GitHub owner/repository names. Rejects anything that could alter a URL.
pub(crate) fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// `owner/repo` for github.com remotes (HTTPS, SSH or scp-like syntax).
fn github_repo(url: &str) -> Option<String> {
    let url = url.trim().trim_start_matches("git+");
    let url = url.strip_suffix(".git").unwrap_or(url);
    let path = ["https://github.com/", "http://github.com/", "ssh://git@github.com/", "git@github.com:"]
        .iter()
        .find_map(|prefix| url.strip_prefix(prefix))?;
    let (owner, repo) = path.trim_matches('/').split_once('/')?;
    (is_name(owner) && is_name(repo)).then(|| format!("{owner}/{repo}"))
}

/// The project's logo as data URL, from the usual places in these projects.
fn icon(dir: &Path) -> Option<String> {
    const CANDIDATES: [(&str, &str); 6] = [
        ("src/assets/logo.svg", "image/svg+xml"),
        ("assets/logo.svg", "image/svg+xml"),
        ("src-tauri/icons/128x128.png", "image/png"),
        ("assets/icon.png", "image/png"),
        ("public/icon.png", "image/png"),
        ("public/favicon.svg", "image/svg+xml"),
    ];
    CANDIDATES.iter().find_map(|(file, mime)| {
        let path = dir.join(file);
        let size = fs::metadata(&path).ok()?.len();
        if size > 1024 * 1024 {
            return None;
        }
        let bytes = fs::read(path).ok()?;
        Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_github_remotes() {
        for url in [
            "https://github.com/mraudev/mrstart.git",
            "https://github.com/mraudev/mrstart",
            "git@github.com:mraudev/mrstart.git",
            "ssh://git@github.com/mraudev/mrstart.git",
            "git+https://github.com/mraudev/mrstart.git",
        ] {
            assert_eq!(github_repo(url).as_deref(), Some("mraudev/mrstart"), "{url}");
        }
    }

    #[test]
    fn rejects_other_hosts_and_bad_names() {
        assert_eq!(github_repo("https://gitea.example.com/a/b.git"), None);
        assert_eq!(github_repo("https://github.com/a/../b"), None);
        assert_eq!(github_repo("https://github.com/a/b?x=1"), None);
        assert_eq!(github_repo("C:/repos/app"), None);
    }
}
