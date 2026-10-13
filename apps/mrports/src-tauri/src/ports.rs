//! Sockets together with the process that owns them: name, exe, command line and working folder.

use crate::win::{self, Socket};
use serde::Serialize;
use std::{collections::HashMap, sync::Mutex};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    /// Empty if access is denied (processes of other users without admin rights).
    pub exe: String,
    pub cmd: String,
    /// Working folder – for `node.exe` and the like usually the project.
    pub cwd: String,
}

#[derive(Serialize)]
pub struct Snapshot {
    sockets: Vec<Socket>,
    processes: Vec<ProcessInfo>,
}

/// Keeps sysinfo's process list between refreshes (cheaper than starting over).
#[derive(Default)]
pub struct Monitor {
    system: Mutex<Option<System>>,
}

fn process_info(system: &System, pid: u32) -> ProcessInfo {
    let text = |p: Option<&std::path::Path>| p.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    match system.process(Pid::from_u32(pid)) {
        Some(p) => ProcessInfo {
            pid,
            name: p.name().to_string_lossy().into_owned(),
            exe: text(p.exe()),
            cmd: p.cmd().iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" "),
            cwd: text(p.cwd()),
        },
        // pid 0 and 4 are not real processes for sysinfo.
        None => ProcessInfo {
            pid,
            name: match pid {
                0 => "Leerlaufprozess".into(),
                4 => "System".into(),
                _ => format!("Prozess {pid}"),
            },
            ..Default::default()
        },
    }
}

/// All sockets and the processes that own them.
#[tauri::command]
pub async fn snapshot(monitor: tauri::State<'_, Monitor>) -> Result<Snapshot, String> {
    let sockets = win::sockets();
    let mut pids: Vec<u32> = sockets.iter().map(|s| s.pid).collect();
    pids.sort_unstable();
    pids.dedup();

    let mut guard = monitor.system.lock().unwrap();
    let system = guard.get_or_insert_with(System::new);
    let wanted: Vec<Pid> = pids.iter().map(|&p| Pid::from_u32(p)).collect();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&wanted),
        true,
        ProcessRefreshKind::nothing()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet)
            .with_cwd(UpdateKind::OnlyIfNotSet),
    );
    let processes = pids.iter().map(|&pid| process_info(system, pid)).collect();
    Ok(Snapshot { sockets, processes })
}

#[tauri::command]
pub fn kill(pid: u32) -> Result<(), String> {
    if pid == std::process::id() {
        return Err("mrports kann sich nicht selbst beenden".into());
    }
    if pid <= 4 {
        return Err("Systemprozesse können nicht beendet werden".into());
    }
    win::terminate(pid)
}

/// Exe icons as PNG data URLs (null where there is none), by path.
#[tauri::command]
pub async fn icons(paths: Vec<String>) -> Result<HashMap<String, Option<String>>, String> {
    tauri::async_runtime::spawn_blocking(move || paths.into_iter().map(|p| (p.clone(), win::icon(&p))).collect())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_elevated() -> bool {
    win::self_elevated()
}

/// Starts an elevated instance and quits this one. Does nothing if the UAC prompt is declined.
#[tauri::command]
pub fn restart_as_admin(app: tauri::AppHandle) -> Result<(), String> {
    if win::run_as_admin()? {
        app.exit(0);
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn describes_the_own_process() {
        let mut system = System::new();
        let me = std::process::id();
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[Pid::from_u32(me)]),
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::Always).with_cmd(UpdateKind::Always).with_cwd(UpdateKind::Always),
        );
        let info = process_info(&system, me);
        assert!(info.exe.to_lowercase().ends_with(".exe"));
        assert!(!info.cwd.is_empty());
        assert_eq!(process_info(&system, 4).name, "System");
    }

    #[test]
    fn refuses_to_kill_itself_and_the_system() {
        assert!(kill(std::process::id()).is_err());
        assert!(kill(4).is_err());
    }
}
