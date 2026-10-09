use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use sysinfo::{MemoryRefreshKind, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, UpdateKind};

use crate::win;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcInfo {
    pid: u32,
    /// 0 if unknown or the parent has exited.
    parent: u32,
    name: String,
    exe: String,
    cmd: String,
    user: String,
    /// Share of the whole machine like in Task Manager (0–100), not per core.
    cpu: f32,
    /// Private working set in bytes – the memory figure of Task Manager.
    memory: u64,
    /// Whole working set including shared pages.
    working_set: u64,
    /// Private bytes (commit charge).
    private: u64,
    disk_read: u64,
    disk_write: u64,
    threads: u32,
    handles: u32,
    /// Seconds since 1970.
    start_time: u64,
    /// Total CPU time in ms.
    cpu_time: u64,
    suspended: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    processes: Vec<ProcInfo>,
    cpu: f32,
    cpu_count: usize,
    memory_used: u64,
    memory_total: u64,
    /// Seconds since the last snapshot – the base of the disk rates.
    interval: f64,
}

pub struct Monitor {
    inner: Mutex<Inner>,
}

struct Inner {
    sys: System,
    last: Option<Instant>,
    /// SID → "DOMAIN\user", looked up once per SID.
    users: HashMap<String, String>,
}

impl Default for Monitor {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner { sys: System::new(), last: None, users: HashMap::new() }),
        }
    }
}

/// Refreshes all processes. The first call has no CPU or disk rates yet (they need two samples).
#[tauri::command]
pub async fn snapshot(monitor: tauri::State<'_, Monitor>) -> Result<Snapshot, String> {
    let mut inner = monitor.inner.lock().map_err(|e| e.to_string())?;
    let Inner { sys, last, users } = &mut *inner;

    sys.refresh_cpu_usage();
    sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .with_disk_usage()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet)
            .with_user(UpdateKind::OnlyIfNotSet),
    );
    let now = Instant::now();
    let interval = last.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0);
    *last = Some(now);

    let cpu_count = sys.cpus().len().max(1);
    let nt = win::system_processes();

    let processes = sys
        .processes()
        .iter()
        .map(|(pid, p)| {
            let pid = pid.as_u32();
            let user = p
                .user_id()
                .map(|uid| {
                    let sid = (**uid).to_string();
                    users.entry(sid.clone()).or_insert_with(|| win::account_name(&sid).unwrap_or(sid)).clone()
                })
                .unwrap_or_default();
            let disk = p.disk_usage();
            let extra = nt.get(&pid).copied().unwrap_or_default();
            ProcInfo {
                pid,
                parent: p.parent().map(|p| p.as_u32()).unwrap_or(0),
                name: p.name().to_string_lossy().into_owned(),
                exe: p.exe().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default(),
                cmd: p.cmd().iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" "),
                user,
                cpu: p.cpu_usage() / cpu_count as f32,
                memory: if extra.private_ws > 0 { extra.private_ws } else { p.memory() },
                working_set: p.memory(),
                private: p.virtual_memory(),
                disk_read: disk.read_bytes,
                disk_write: disk.written_bytes,
                threads: extra.threads,
                handles: extra.handles,
                start_time: p.start_time(),
                cpu_time: p.accumulated_cpu_time(),
                suspended: extra.suspended || p.status() == ProcessStatus::Stop,
            }
        })
        .collect();

    Ok(Snapshot {
        processes,
        cpu: sys.global_cpu_usage(),
        cpu_count,
        memory_used: sys.used_memory(),
        memory_total: sys.total_memory(),
        interval,
    })
}

/// Pids of `root` and all its descendants, children first so they go before their parent.
pub fn tree_of(monitor: &Monitor, root: u32) -> Result<Vec<u32>, String> {
    let inner = monitor.inner.lock().map_err(|e| e.to_string())?;
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for (pid, p) in inner.sys.processes() {
        if let Some(parent) = p.parent() {
            // Pid reuse can make a process look like the parent of an older one.
            let older = inner.sys.process(parent).is_some_and(|pp| pp.start_time() <= p.start_time());
            if older {
                children.entry(parent.as_u32()).or_default().push(pid.as_u32());
            }
        }
    }
    let mut out = Vec::new();
    let mut stack = vec![(root, false)];
    while let Some((pid, visited)) = stack.pop() {
        if visited {
            out.push(pid);
            continue;
        }
        stack.push((pid, true));
        for &child in children.get(&pid).into_iter().flatten() {
            if child != pid && !out.contains(&child) {
                stack.push((child, false));
            }
        }
    }
    Ok(out)
}
