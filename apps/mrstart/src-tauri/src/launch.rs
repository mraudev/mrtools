//! Starting external programs: editor, terminals, project apps, custom
//! commands and the configured Git GUI.

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchFailed {
    command: String,
    code: Option<i32>,
}

/// Runs a command line through `cmd.exe` without a console window. The process
/// is detached; a non-zero exit code is reported via the `launch-failed` event.
fn spawn_shell(app: &AppHandle, command_line: &str, cwd: Option<&Path>) -> Result<(), String> {
    let mut cmd = Command::new("cmd");
    // `/S /C "<line>"`: cmd strips the outer quotes and runs <line> verbatim.
    cmd.raw_arg(format!("/D /S /C \"{command_line}\""))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    let app = app.clone();
    let command = command_line.to_string();
    std::thread::spawn(move || {
        if let Ok(status) = child.wait() {
            if !status.success() {
                let _ = app.emit(
                    "launch-failed",
                    LaunchFailed {
                        command,
                        code: status.code(),
                    },
                );
            }
        }
    });
    Ok(())
}

/// Runs a user-defined command line, optionally inside `cwd`.
#[tauri::command]
pub fn run_command(app: AppHandle, command: String, cwd: Option<String>) -> Result<(), String> {
    let cwd = cwd.filter(|d| !d.is_empty()).map(PathBuf::from);
    if let Some(dir) = &cwd {
        if !dir.is_dir() {
            return Err(format!("Ordner existiert nicht: {}", dir.display()));
        }
    }
    spawn_shell(&app, &command, cwd.as_deref())
}

/// Opens a folder in Explorer, or a file with its associated program.
#[tauri::command]
pub fn open_path(app: AppHandle, path: String) -> Result<(), String> {
    let path = PathBuf::from(path.replace('/', "\\"));
    if path.is_dir() {
        // explorer.exe always exits with code 1, so its status is not checked.
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    } else if path.is_file() {
        spawn_shell(
            &app,
            &format!("start \"\" \"{}\"", path.display()),
            path.parent(),
        )
    } else {
        Err(format!("Pfad existiert nicht: {}", path.display()))
    }
}

fn default_git_tool(tool: &str) -> Option<PathBuf> {
    let candidates: Vec<PathBuf> = match tool {
        "tortoise" => vec![PathBuf::from(
            r"C:\Program Files\TortoiseGit\bin\TortoiseGitProc.exe",
        )],
        _ => {
            let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
            vec![
                Path::new(&local).join(r"Fork\current\Fork.exe"),
                Path::new(&local).join(r"Fork\Fork.exe"),
            ]
        }
    };
    candidates.into_iter().find(|p| p.is_file())
}

/// Opens a repository in Fork or TortoiseGit. `action` is `status` or `log`
/// (Fork has no command line for these and simply opens the repository).
#[tauri::command]
pub fn open_git_tool(
    tool: String,
    executable: String,
    action: String,
    path: String,
) -> Result<(), String> {
    let exe = if executable.trim().is_empty() {
        default_git_tool(&tool).ok_or_else(|| {
            "Git-Programm nicht gefunden – bitte den Pfad in den Einstellungen setzen.".to_string()
        })?
    } else {
        PathBuf::from(executable.trim())
    };

    let mut cmd = Command::new(&exe);
    if tool == "tortoise" {
        let command = if action == "log" { "log" } else { "repostatus" };
        cmd.raw_arg(format!("/command:{command} /path:\"{path}\""));
    } else {
        cmd.arg(&path);
    }
    cmd.spawn().map_err(|e| format!("{}: {e}", exe.display()))?;
    Ok(())
}
