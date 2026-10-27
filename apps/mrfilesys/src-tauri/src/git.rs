//! Git status for the file list, like TortoiseGit's overlays. Uses the installed `git` (so
//! `.gitignore`, submodules and the user's config behave exactly as on the command line).

use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

/// From least to most important – a folder shows the most important state of its content.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ignored,
    Untracked,
    /// Changed and added to the index (staged), nothing else changed in the working tree.
    Staged,
    Modified,
    Conflict,
}

#[derive(Serialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    /// Folder that contains `.git`.
    root: String,
    /// `None` on a detached HEAD.
    branch: Option<String>,
    upstream: Option<String>,
    ahead: u32,
    behind: u32,
    /// Changed files (and submodules), relative to `root` with `/`.
    files: HashMap<String, Status>,
    /// Folders that are untracked or ignored as a whole – everything below has that state.
    whole: HashMap<String, Status>,
    /// Folders with changes somewhere below, with the most important state.
    dirs: HashMap<String, Status>,
}

fn git() -> Command {
    let mut command = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// The folder containing `.git` (a folder, or a file for worktrees and submodules).
fn find_root(path: &Path) -> Option<PathBuf> {
    path.ancestors().find(|dir| dir.join(".git").exists()).map(Path::to_path_buf)
}

/// Marks all parent folders of `path` with `status` unless they already have a more important one.
fn mark_parents(dirs: &mut HashMap<String, Status>, path: &str, status: Status) {
    let mut current = path;
    while let Some(i) = current.rfind('/') {
        current = &current[..i];
        let entry = dirs.entry(current.to_owned()).or_insert(status);
        *entry = (*entry).max(status);
    }
}

/// Parses `git status --porcelain=v2 -z --branch`.
fn parse(output: &str, root: &Path) -> RepoStatus {
    let mut status = RepoStatus { root: root.to_string_lossy().into_owned(), ..Default::default() };
    let mut records = output.split('\0').filter(|r| !r.is_empty());
    while let Some(record) = records.next() {
        if let Some(header) = record.strip_prefix("# ") {
            if let Some(head) = header.strip_prefix("branch.head ") {
                status.branch = (head != "(detached)").then(|| head.to_owned());
            } else if let Some(upstream) = header.strip_prefix("branch.upstream ") {
                status.upstream = Some(upstream.to_owned());
            } else if let Some(ab) = header.strip_prefix("branch.ab ") {
                let mut parts = ab.split(' ');
                status.ahead = parts.next().and_then(|a| a.trim_start_matches('+').parse().ok()).unwrap_or(0);
                status.behind = parts.next().and_then(|b| b.trim_start_matches('-').parse().ok()).unwrap_or(0);
            }
            continue;
        }
        // Fields between the kind and the path (the path itself may contain spaces):
        // "1 XY sub mH mI mW hH hI path", "2 … Xscore path" + original path, "u XY sub m1 m2 m3 mW h1 h2 h3 path".
        let Some((kind, rest)) = record.split_once(' ') else { continue };
        let (fields, file_status) = match kind {
            "1" => (7, None),
            "2" => (8, None),
            "u" => (9, Some(Status::Conflict)),
            "?" => (0, Some(Status::Untracked)),
            "!" => (0, Some(Status::Ignored)),
            _ => continue,
        };
        let parts: Vec<&str> = rest.splitn(fields + 1, ' ').collect();
        let (Some(&path), xy) = (parts.get(fields), if fields > 0 { parts[0] } else { ".." }) else { continue };
        if kind == "2" {
            records.next(); // the original path of a rename
        }
        let file_status = file_status.unwrap_or_else(|| {
            let (index, worktree) = (xy.as_bytes()[0], xy.as_bytes().get(1).copied().unwrap_or(b'.'));
            if worktree != b'.' {
                Status::Modified
            } else if index != b'.' {
                Status::Staged
            } else {
                Status::Modified
            }
        });
        // Untracked and ignored folders come as "dir/".
        if let Some(dir) = path.strip_suffix('/') {
            status.whole.insert(dir.to_owned(), file_status);
            if file_status != Status::Ignored {
                mark_parents(&mut status.dirs, dir, file_status);
            }
            continue;
        }
        status.files.insert(path.to_owned(), file_status);
        if file_status != Status::Ignored {
            mark_parents(&mut status.dirs, path, file_status);
        }
    }
    status
}

/// Git status of the repository that contains `path`, `None` outside of repositories.
#[tauri::command]
pub async fn git_status(path: String, ignored: bool) -> Result<Option<RepoStatus>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(root) = find_root(Path::new(&path)) else { return Ok(None) };
        let mut command = git();
        command
            .arg("-C")
            .arg(&root)
            .args(["status", "--porcelain=v2", "-z", "--branch", "--untracked-files=normal"]);
        if ignored {
            command.arg("--ignored=matching");
        }
        let output = command.output().map_err(|e| format!("git nicht gefunden: {e}"))?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);
            return Err(message.lines().next().unwrap_or("git status fehlgeschlagen").to_owned());
        }
        Ok(Some(parse(&String::from_utf8_lossy(&output.stdout), &root)))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    hash: String,
    short: String,
    author: String,
    /// ISO 8601.
    date: String,
    subject: String,
}

/// Fields separated by the unit separator – it never appears in names or subjects.
const COMMIT_FORMAT: &str = "--format=%H%x1f%h%x1f%an%x1f%aI%x1f%s";

fn parse_commit(text: &str) -> Option<Commit> {
    let mut fields = text.trim_end_matches(['\r', '\n']).split('\x1f');
    let mut next = || fields.next().map(str::to_owned);
    let commit = Commit { hash: next()?, short: next()?, author: next()?, date: next()?, subject: next()? };
    (!commit.hash.is_empty()).then_some(commit)
}

/// The last commit that changed `path` (a file or folder), `None` if there is none.
#[tauri::command]
pub async fn git_last_commit(path: String) -> Result<Option<Commit>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = PathBuf::from(path);
        let Some(root) = find_root(&path) else { return Ok(None) };
        let output = git()
            .arg("-C")
            .arg(&root)
            .args(["log", "-1", COMMIT_FORMAT, "--"])
            .arg(&path)
            .output()
            .map_err(|e| format!("git nicht gefunden: {e}"))?;
        Ok(if output.status.success() { parse_commit(&String::from_utf8_lossy(&output.stdout)) } else { None })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The installed git's version, `None` if there is none.
#[tauri::command]
pub async fn git_version() -> Option<String> {
    tauri::async_runtime::spawn_blocking(|| {
        let output = git().arg("--version").output().ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        output.status.success().then(|| text.trim().trim_start_matches("git version ").to_owned())
    })
    .await
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_files_and_folders() {
        let output = [
            "# branch.oid 1234",
            "# branch.head main",
            "# branch.upstream origin/main",
            "# branch.ab +2 -1",
            "1 .M N... 100644 100644 100644 aaa bbb src/lib/store.ts",
            "1 A. N... 000000 100644 100644 000 ccc src/new file.ts",
            "2 R. N... 100644 100644 100644 ddd eee R100 docs/neu.md",
            "docs/alt.md",
            "u UU N... 100644 100644 100644 100644 f g h README.md",
            "? scratch/",
            "? notes.txt",
            "! node_modules/",
            "! src/debug.log",
        ]
        .join("\0");
        let s = parse(&output, Path::new("C:\\dev\\repo"));
        assert_eq!((s.branch.as_deref(), s.upstream.as_deref(), s.ahead, s.behind), (Some("main"), Some("origin/main"), 2, 1));
        assert_eq!(s.files["src/lib/store.ts"], Status::Modified);
        assert_eq!(s.files["src/new file.ts"], Status::Staged);
        assert_eq!(s.files["docs/neu.md"], Status::Staged);
        assert!(!s.files.contains_key("docs/alt.md"));
        assert_eq!(s.files["README.md"], Status::Conflict);
        assert_eq!(s.files["notes.txt"], Status::Untracked);
        assert_eq!(s.files["src/debug.log"], Status::Ignored);
        assert_eq!(s.whole["scratch"], Status::Untracked);
        assert_eq!(s.whole["node_modules"], Status::Ignored);
        // Folders show the most important state below them; ignored files do not count.
        assert_eq!(s.dirs["src"], Status::Modified);
        assert_eq!(s.dirs["src/lib"], Status::Modified);
        assert_eq!(s.dirs["docs"], Status::Staged);
        assert!(!s.dirs.contains_key("node_modules"));
    }

    #[test]
    fn handles_a_detached_head_and_a_clean_tree() {
        let s = parse("# branch.oid abc\0# branch.head (detached)\0", Path::new("C:\\r"));
        assert_eq!((s.branch, s.files.len(), s.dirs.len()), (None, 0, 0));
    }

    #[test]
    fn finds_the_repository_root() {
        let dir = std::env::temp_dir().join(format!("mrfilesys-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::create_dir_all(dir.join("a").join("b")).unwrap();
        assert_eq!(find_root(&dir.join("a").join("b")), Some(dir.clone()));
        assert_eq!(find_root(Path::new("C:\\Windows\\System32\\drivers")), None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn parses_commits() {
        let text = "abc123def\x1fabc123d\x1fMichael Rau\x1f2026-10-27T10:00:00+01:00\x1fFix: a | b\n";
        assert_eq!(
            parse_commit(text),
            Some(Commit {
                hash: "abc123def".into(),
                short: "abc123d".into(),
                author: "Michael Rau".into(),
                date: "2026-10-27T10:00:00+01:00".into(),
                subject: "Fix: a | b".into(),
            })
        );
        assert_eq!(parse_commit(""), None);
    }

    /// Runs against this repository – git is installed on developer machines and CI runners.
    #[test]
    fn reads_this_repository() {
        let here = env!("CARGO_MANIFEST_DIR").to_owned();
        let status = tauri::async_runtime::block_on(git_status(here.clone(), false)).unwrap().unwrap();
        assert!(Path::new(&status.root).join(".git").exists());
        assert!(tauri::async_runtime::block_on(git_version()).is_some());
        let manifest = format!("{here}\\Cargo.toml");
        let commit = tauri::async_runtime::block_on(git_last_commit(manifest)).unwrap();
        // CI may check out a shallow clone – then there is still the one commit.
        assert!(commit.is_some_and(|c| c.hash.len() >= 7));
    }
}
