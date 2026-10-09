//! Fast permanent deletion: whole trees in parallel, without counting first like Explorer does.
//! Never follows symbolic links or junctions – only the link itself is removed.

use rayon::prelude::*;
use serde::Serialize;
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, State};

/// Failed paths reported back – the rest are only counted.
const MAX_FAILURES: usize = 50;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    path: String,
    message: String,
}

#[derive(Default)]
struct Progress {
    deleted: AtomicU64,
    failed: AtomicU64,
    failures: Mutex<Vec<Failure>>,
    current: Mutex<String>,
    cancel: AtomicBool,
}

impl Progress {
    fn fail(&self, path: &Path, error: io::Error) {
        self.failed.fetch_add(1, Ordering::Relaxed);
        let mut failures = self.failures.lock().unwrap();
        if failures.len() < MAX_FAILURES {
            failures.push(Failure { path: path.to_string_lossy().into_owned(), message: error.to_string() });
        }
    }
}

/// Removes a file; a read-only one is made writable first (Windows refuses to delete it otherwise).
fn remove_file(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            let mut permissions = fs::symlink_metadata(path)?.permissions();
            if !permissions.readonly() {
                return Err(e);
            }
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            fs::set_permissions(path, permissions)?;
            fs::remove_file(path)
        }
        result => result,
    }
}

/// A link to a folder (directory symlink, junction) is removed like a folder, any other like a file.
fn remove_link(path: &Path, meta: &fs::Metadata) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
        if meta.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0 {
            return fs::remove_dir(path);
        }
    }
    #[cfg(not(windows))]
    let _ = meta;
    remove_file(path)
}

fn remove_tree(path: &Path, progress: &Progress) {
    if progress.cancel.load(Ordering::Relaxed) {
        return;
    }
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return,
        Err(e) => return progress.fail(path, e),
    };
    let result = if meta.file_type().is_symlink() {
        remove_link(path, &meta)
    } else if meta.is_dir() {
        match fs::read_dir(path) {
            Ok(entries) => {
                let children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
                *progress.current.lock().unwrap() = path.to_string_lossy().into_owned();
                children.par_iter().for_each(|child| remove_tree(child, progress));
                if progress.cancel.load(Ordering::Relaxed) {
                    return;
                }
                fs::remove_dir(path)
            }
            Err(e) => Err(e),
        }
    } else {
        remove_file(path)
    };
    match result {
        Ok(()) => {
            progress.deleted.fetch_add(1, Ordering::Relaxed);
        }
        // A folder still containing something that failed – that failure is reported already.
        Err(e) if meta.is_dir() && progress.failed.load(Ordering::Relaxed) > 0 && is_not_empty(&e) => {}
        Err(e) => progress.fail(path, e),
    }
}

/// Deletes `path` completely (used after copying for a move across drives). True if nothing failed.
pub(crate) fn remove_all(path: &Path) -> bool {
    let progress = Progress::default();
    remove_tree(path, &progress);
    progress.failed.load(Ordering::Relaxed) == 0
}

fn is_not_empty(error: &io::Error) -> bool {
    // ERROR_DIR_NOT_EMPTY on Windows, ENOTEMPTY elsewhere.
    error.kind() == io::ErrorKind::DirectoryNotEmpty || error.raw_os_error() == Some(145)
}

/// Drive roots and the like are never deleted, however they got here.
fn check(path: &Path) -> Result<(), String> {
    if path.parent().is_none() || !path.is_absolute() {
        return Err(format!("„{}“ kann nicht gelöscht werden", path.display()));
    }
    Ok(())
}

#[derive(Default)]
pub struct Deletions {
    next_id: AtomicU64,
    running: Mutex<HashMap<u64, Arc<Progress>>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    id: u64,
    deleted: u64,
    failed: u64,
    current: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoneEvent {
    id: u64,
    deleted: u64,
    failed: u64,
    failures: Vec<Failure>,
    cancelled: bool,
    elapsed_ms: u64,
}

/// Deletes `paths` permanently in the background and returns the job id. Progress arrives as
/// `delete-progress` events, the end as `delete-done`.
#[tauri::command]
pub fn delete_permanently(app: AppHandle, deletions: State<'_, Deletions>, paths: Vec<String>) -> Result<u64, String> {
    let roots: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    roots.iter().try_for_each(|p| check(p))?;

    let id = deletions.next_id.fetch_add(1, Ordering::Relaxed) + 1;
    let progress = Arc::new(Progress::default());
    deletions.running.lock().unwrap().insert(id, progress.clone());
    let started = Instant::now();
    let done = Arc::new(AtomicBool::new(false));

    let (reporter, reporter_done, reporter_app) = (progress.clone(), done.clone(), app.clone());
    std::thread::spawn(move || {
        while !reporter_done.load(Ordering::Relaxed) {
            let _ = reporter_app.emit(
                "delete-progress",
                ProgressEvent {
                    id,
                    deleted: reporter.deleted.load(Ordering::Relaxed),
                    failed: reporter.failed.load(Ordering::Relaxed),
                    current: reporter.current.lock().unwrap().clone(),
                },
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    });

    std::thread::spawn(move || {
        use tauri::Manager;
        roots.par_iter().for_each(|root| remove_tree(root, &progress));
        done.store(true, Ordering::Relaxed);
        app.state::<Deletions>().running.lock().unwrap().remove(&id);
        let _ = app.emit(
            "delete-done",
            DoneEvent {
                id,
                deleted: progress.deleted.load(Ordering::Relaxed),
                failed: progress.failed.load(Ordering::Relaxed),
                failures: progress.failures.lock().unwrap().clone(),
                cancelled: progress.cancel.load(Ordering::Relaxed),
                elapsed_ms: started.elapsed().as_millis() as u64,
            },
        );
    });
    Ok(id)
}

/// Stops a running deletion – what is deleted already stays deleted.
#[tauri::command]
pub fn cancel_delete(deletions: State<'_, Deletions>, id: u64) {
    if let Some(progress) = deletions.running.lock().unwrap().get(&id) {
        progress.cancel.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mrfilesys-delete-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn deletes_a_tree_with_read_only_files() {
        let dir = temp_dir("tree");
        let root = dir.join("root");
        for i in 0..20 {
            let sub = root.join(format!("d{i}")).join("deeper");
            fs::create_dir_all(&sub).unwrap();
            for j in 0..5 {
                fs::write(sub.join(format!("f{j}.txt")), "x").unwrap();
            }
        }
        let locked = root.join("locked.txt");
        fs::write(&locked, "x").unwrap();
        let mut permissions = fs::metadata(&locked).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&locked, permissions).unwrap();

        let progress = Progress::default();
        remove_tree(&root, &progress);
        assert!(!root.exists());
        // 20 × (d, deeper, 5 files) + locked.txt + root
        assert_eq!(progress.deleted.load(Ordering::Relaxed), 20 * 7 + 2);
        assert_eq!(progress.failed.load(Ordering::Relaxed), 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn removes_a_junction_without_touching_its_target() {
        let dir = temp_dir("junction");
        let target = dir.join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep.txt"), "x").unwrap();
        let link = dir.join("link");
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .unwrap();
        assert!(status.status.success());

        let progress = Progress::default();
        remove_tree(&link, &progress);
        assert!(fs::symlink_metadata(&link).is_err());
        assert!(target.join("keep.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_paths_are_no_error() {
        let progress = Progress::default();
        remove_tree(Path::new("C:\\does-not-exist-mrfilesys-1234"), &progress);
        assert_eq!(progress.failed.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn refuses_roots() {
        assert!(check(Path::new("C:\\")).is_err());
        assert!(check(Path::new("relative")).is_err());
        assert!(check(Path::new("C:\\x")).is_ok());
    }

    #[test]
    fn cancel_stops_early() {
        let dir = temp_dir("cancel");
        fs::write(dir.join("a"), "x").unwrap();
        let progress = Progress::default();
        progress.cancel.store(true, Ordering::Relaxed);
        remove_tree(&dir, &progress);
        assert!(dir.join("a").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(all(test, windows))]
mod bench {
    use super::*;

    fn make_tree(root: &Path) {
        for d in 0..2000 {
            let dir = root.join(format!("pkg{d}")).join("lib");
            fs::create_dir_all(&dir).unwrap();
            for f in 0..20 {
                fs::write(dir.join(format!("file{f}.js")), "module.exports = 1;\n").unwrap();
            }
        }
    }

    /// `cargo test -p mrfilesys --release bench -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn compare_with_windows() {
        let base = std::env::temp_dir().join("mrfilesys-bench");
        let _ = fs::remove_dir_all(&base);
        let time = |label: &str, f: &dyn Fn(&Path)| {
            let root = base.join(label.replace(|c: char| !c.is_alphanumeric(), "_"));
            make_tree(&root);
            let start = Instant::now();
            f(&root);
            println!("{label:>14}: {:>6} ms (übrig: {})", start.elapsed().as_millis(), root.exists());
        };
        time("mrfilesys", &|root| remove_tree(root, &Progress::default()));
        time("rd /s /q", &|root| {
            std::process::Command::new("cmd").args(["/c", "rd", "/s", "/q"]).arg(root).status().unwrap();
        });
        time("SHFileOperation", &|root| {
            crate::shell::bench_delete(root.to_str().unwrap());
        });
        let _ = fs::remove_dir_all(&base);
    }
}
