use std::collections::HashMap;

use serde::Serialize;

use crate::procs::{tree_of, Monitor};
use crate::win;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    /// Windows priority class, 0 if it cannot be read.
    priority: u32,
    /// `None` if access is denied.
    elevated: Option<bool>,
}

#[tauri::command]
pub async fn kill(pid: u32) -> Result<(), String> {
    win::terminate(pid)
}

/// Ends the process and all its descendants. Returns how many were ended.
#[tauri::command]
pub async fn kill_tree(monitor: tauri::State<'_, Monitor>, pid: u32) -> Result<usize, String> {
    let mut ended = 0;
    let mut first_error = None;
    for p in tree_of(&monitor, pid)? {
        match win::terminate(p) {
            Ok(()) => ended += 1,
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    match first_error {
        Some(e) if ended == 0 => Err(e),
        _ => Ok(ended),
    }
}

#[tauri::command]
pub async fn suspend(pid: u32, suspend: bool) -> Result<(), String> {
    win::suspend(pid, suspend)
}

#[tauri::command]
pub async fn set_priority(pid: u32, class: u32) -> Result<(), String> {
    win::set_priority(pid, class)
}

#[tauri::command]
pub async fn details(pid: u32) -> Details {
    Details { priority: win::priority(pid), elevated: win::elevated(pid) }
}

#[tauri::command]
pub async fn modules(pid: u32) -> Result<Vec<win::Module>, String> {
    win::modules(pid)
}

#[tauri::command]
pub async fn connections(pid: u32) -> Vec<win::Connection> {
    win::connections(pid)
}

/// Icons by exe path; paths without an icon map to null.
#[tauri::command]
pub async fn icons(paths: Vec<String>) -> HashMap<String, Option<String>> {
    paths.into_iter().map(|p| (p.clone(), win::icon(&p))).collect()
}

/// Sync on purpose: the properties dialog needs the main thread's message loop.
#[tauri::command]
pub fn show_properties(path: String) -> Result<(), String> {
    win::show_properties(&path)
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
