//! Git integration: branch lookup and streamed pull/push runs.

use crate::launch::CREATE_NO_WINDOW;
use serde::Serialize;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use tauri::ipc::Channel;

/// git.exe from PATH, or from the usual Git for Windows install locations
/// (e.g. when Git was installed with "Use Git from Git Bash only").
fn git_exe() -> &'static Path {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        for (var, sub) in [
            ("ProgramFiles", r"Git\cmd"),
            ("ProgramFiles(x86)", r"Git\cmd"),
            ("LOCALAPPDATA", r"Programs\Git\cmd"),
        ] {
            if let Some(base) = std::env::var_os(var) {
                dirs.push(Path::new(&base).join(sub));
            }
        }
        dirs.iter()
            .map(|dir| dir.join("git.exe"))
            .find(|exe| exe.is_file())
            .unwrap_or_else(|| PathBuf::from("git"))
    })
}

fn git(path: &str) -> Command {
    let mut cmd = Command::new(git_exe());
    cmd.current_dir(path)
        // Never wait for credentials on a terminal nobody can see.
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Runs `git <args>` in `path` and returns the trimmed stdout, or `None` if
/// git fails (e.g. because `path` is not inside a repository).
pub fn git_output(path: &str, args: &[&str]) -> Option<String> {
    git_try(path, args).ok()
}

/// Why a git call failed.
pub enum GitFailure {
    /// git.exe could not be started.
    NotInstalled,
    /// git ran and failed; the first line of stderr.
    Failed(String),
}

pub fn git_try(path: &str, args: &[&str]) -> Result<String, GitFailure> {
    let out = git(path)
        .args(args)
        .output()
        .map_err(|_| GitFailure::NotInstalled)?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        Err(GitFailure::Failed(stderr.trim().to_string()))
    }
}

/// Checked-out branch; empty for a detached HEAD. Works with any Git version
/// (`git branch --show-current` needs Git 2.22).
pub fn current_branch(path: &str) -> Option<String> {
    git_output(path, &["rev-parse", "--is-inside-work-tree"])?;
    Some(git_output(path, &["symbolic-ref", "--short", "-q", "HEAD"]).unwrap_or_default())
}

/// Number of changed or untracked files (`git status --porcelain`).
pub fn changed_files(path: &str) -> u32 {
    git_output(path, &["status", "--porcelain", "--untracked-files=normal"])
        .map(|out| out.lines().filter(|l| !l.trim().is_empty()).count() as u32)
        .unwrap_or(0)
}

/// Background fetch of all given repositories (in parallel). Credential
/// prompts are suppressed; returns one message per failed repository.
#[tauri::command]
pub async fn git_fetch_all(paths: Vec<String>) -> Vec<String> {
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::scope(|scope| {
            let handles: Vec<_> = paths
                .iter()
                .map(|path| {
                    scope.spawn(move || {
                        // Folders without a repository are skipped silently.
                        current_branch(path)?;
                        let out = git(path)
                            .env("GCM_INTERACTIVE", "never")
                            .args(["fetch", "--quiet"])
                            .output();
                        match out {
                            Ok(out) if out.status.success() => None,
                            Ok(out) => Some(format!(
                                "{path}: {}",
                                String::from_utf8_lossy(&out.stderr)
                                    .lines()
                                    .next()
                                    .unwrap_or("Fehler")
                            )),
                            Err(e) => Some(format!("{path}: {e}")),
                        }
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
    .unwrap_or_default()
}

/// Classifies why `path` cannot be used as a repository: `Ok(())` if it is
/// simply no repository, otherwise a message for the user.
pub fn repository_problem(path: &str) -> Result<(), String> {
    if !Path::new(path).is_dir() {
        return Err("Der Projektordner existiert nicht (mehr).".into());
    }
    match git_try(path, &["rev-parse", "--is-inside-work-tree"]) {
        Ok(_) => Ok(()),
        Err(GitFailure::NotInstalled) => Err(
            "Git wurde nicht gefunden – Git for Windows installieren (Option „Git from the command line“) \
             oder in den PATH aufnehmen."
                .into(),
        ),
        // Git ≥ 2.35.2 refuses repositories owned by another user.
        Err(GitFailure::Failed(e)) if e.contains("safe.directory") => Err(format!(
            "Git verweigert den Zugriff, weil der Ordner einem anderen Benutzer gehört (dubious ownership).\n\
             Lösung: git config --global --add safe.directory \"{}\"",
            path.replace('\\', "/")
        )),
        Err(GitFailure::Failed(_)) => Ok(()),
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "text")]
pub enum GitEvent {
    /// A git invocation is about to start.
    Command(String),
    /// A chunk of stdout/stderr output (may contain `\r` progress updates).
    Output(String),
}

fn steps(action: &str) -> Result<Vec<Vec<&'static str>>, String> {
    let fetch = vec!["fetch", "--progress"];
    let rebase = vec!["rebase", "--rebase-merges", "--autostash"];
    let push = vec!["push", "--porcelain", "--progress"];
    match action {
        "pull" => Ok(vec![fetch, rebase]),
        "push" => Ok(vec![fetch, rebase, push]),
        _ => Err(format!("Unbekannte Git-Aktion: {action}")),
    }
}

/// Forwards everything from `reader` to the channel, never splitting a UTF-8
/// sequence across two events.
fn pump(mut reader: impl Read, channel: &Channel<GitEvent>) {
    let mut buf = [0u8; 4096];
    let mut pending: Vec<u8> = Vec::new();
    while let Ok(n) = reader.read(&mut buf) {
        if n == 0 {
            break;
        }
        pending.extend_from_slice(&buf[..n]);
        let complete = match std::str::from_utf8(&pending) {
            Ok(_) => pending.len(),
            // Incomplete sequence at the end: keep it for the next read.
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            Err(_) => pending.len(),
        };
        if complete > 0 {
            let text = String::from_utf8_lossy(&pending[..complete]).into_owned();
            pending.drain(..complete);
            let _ = channel.send(GitEvent::Output(text));
        }
    }
    if !pending.is_empty() {
        let _ = channel.send(GitEvent::Output(
            String::from_utf8_lossy(&pending).into_owned(),
        ));
    }
}

/// Runs `pull` (fetch + rebase with autostash) or `push` (pull + push) and
/// streams the output to `on_event`. Stops at the first failing step.
#[tauri::command]
pub async fn git_run(
    path: String,
    action: String,
    on_event: Channel<GitEvent>,
) -> Result<(), String> {
    let steps = steps(&action)?;
    tauri::async_runtime::spawn_blocking(move || {
        for args in steps {
            let _ = on_event.send(GitEvent::Command(format!("git {}", args.join(" "))));
            let mut child = git(&path)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("git konnte nicht gestartet werden: {e}"))?;

            let stdout = child.stdout.take().expect("stdout is piped");
            let stderr = child.stderr.take().expect("stderr is piped");
            let channel = on_event.clone();
            let stdout_pump = std::thread::spawn(move || pump(stdout, &channel));
            pump(stderr, &on_event);
            let _ = stdout_pump.join();

            let status = child.wait().map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!(
                    "git {} ist fehlgeschlagen (Exit-Code {})",
                    args[0],
                    status.code().unwrap_or(-1)
                ));
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_runs_pull_first() {
        let steps = steps("push").unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0][0], "fetch");
        assert_eq!(steps[1][0], "rebase");
        assert_eq!(steps[2][0], "push");
    }

    #[test]
    fn unknown_action_is_rejected() {
        assert!(steps("reset --hard").is_err());
    }

    #[test]
    fn finds_branch_without_show_current() {
        let root = std::env::temp_dir().join(format!("mrstart-branch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let dir = root.to_str().unwrap();

        // Plain folder: no repository and no problem to report.
        assert_eq!(current_branch(dir), None);
        assert!(repository_problem(dir).is_ok());
        assert!(repository_problem(&format!("{dir}/fehlt")).is_err());

        let run = |args: &[&str]| {
            git(dir)
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
                .args(args)
                .output()
                .unwrap()
        };
        run(&["init", "-b", "main"]);
        run(&["commit", "--allow-empty", "-m", "eins"]);
        assert_eq!(current_branch(dir).as_deref(), Some("main"));

        run(&["checkout", "--detach"]);
        assert_eq!(current_branch(dir).as_deref(), Some(""));

        let _ = std::fs::remove_dir_all(&root);
    }
}
