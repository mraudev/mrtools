//! Git integration: branch lookup and streamed pull/push runs.

use crate::launch::CREATE_NO_WINDOW;
use serde::Serialize;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use tauri::ipc::Channel;

fn git(path: &str) -> Command {
    let mut cmd = Command::new("git");
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
    let out = git(path).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
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
}
