//! Fast copying and moving: trees are copied in parallel (Explorer copies file by file), moves within
//! a drive are renames, moves across drives copy and then delete the source – only if nothing failed.
//! Links to folders (symbolic links, junctions) are not followed.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
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

const MAX_FAILURES: usize = 50;
/// ERROR_NOT_SAME_DEVICE – a rename across drives.
const NOT_SAME_DEVICE: i32 = 17;

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Op {
    Copy,
    Move,
}

/// What happens to items whose name exists in the target already.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Conflict {
    /// Files are overwritten, folders merged.
    Replace,
    Skip,
    /// The new item gets a free name: `name (2)`.
    KeepBoth,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    path: String,
    message: String,
}

#[derive(Default)]
struct Progress {
    /// Files copied (or items renamed).
    files: AtomicU64,
    bytes: AtomicU64,
    /// Files to copy, counted alongside; 0 until known.
    total_files: AtomicU64,
    total_bytes: AtomicU64,
    skipped: AtomicU64,
    failed: AtomicU64,
    failures: Mutex<Vec<Failure>>,
    current: Mutex<String>,
    cancel: AtomicBool,
}

impl Progress {
    fn fail(&self, path: &Path, error: impl ToString) {
        self.failed.fetch_add(1, Ordering::Relaxed);
        let mut failures = self.failures.lock().unwrap();
        if failures.len() < MAX_FAILURES {
            failures.push(Failure { path: path.to_string_lossy().into_owned(), message: error.to_string() });
        }
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
}

/// `x - Kopie.txt`, `x - Kopie (2).txt`, … – like Explorer when pasting into the same folder.
fn copy_name(dir: &Path, name: &str, is_dir: bool) -> PathBuf {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 && !is_dir => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    crate::files::unique(dir, &format!("{stem} - Kopie{ext}"))
}

/// Copies a file; a read-only file in the way is made writable first.
fn copy_file(from: &Path, to: &Path, progress: &Progress) -> io::Result<()> {
    *progress.current.lock().unwrap() = from.to_string_lossy().into_owned();
    let bytes = match fs::copy(from, to) {
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied && to.exists() => {
            let mut permissions = fs::metadata(to)?.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            fs::set_permissions(to, permissions)?;
            fs::copy(from, to)?
        }
        result => result?,
    };
    progress.files.fetch_add(1, Ordering::Relaxed);
    progress.bytes.fetch_add(bytes, Ordering::Relaxed);
    Ok(())
}

/// Copies `from` to `to`, merging into an existing folder (files in it are overwritten).
fn copy_tree(from: &Path, to: &Path, progress: &Progress) {
    if progress.cancelled() {
        return;
    }
    let meta = match fs::symlink_metadata(from) {
        Ok(meta) => meta,
        Err(e) => return progress.fail(from, e),
    };
    if meta.file_type().is_symlink() && fs::metadata(from).is_ok_and(|m| m.is_dir()) {
        return progress.fail(from, "Verknüpfung auf einen Ordner – nicht kopiert");
    }
    if !meta.is_dir() {
        if let Err(e) = copy_file(from, to, progress) {
            progress.fail(from, e);
        }
        return;
    }
    if let Err(e) = fs::create_dir_all(to) {
        return progress.fail(to, e);
    }
    match fs::read_dir(from) {
        Ok(entries) => {
            let children: Vec<_> = entries.flatten().collect();
            children.par_iter().for_each(|child| copy_tree(&child.path(), &to.join(child.file_name()), progress));
        }
        Err(e) => progress.fail(from, e),
    }
}

/// Counts what `copy_tree` will copy, so the progress can show "x of y".
fn count(path: &Path, progress: &Progress) {
    if progress.cancelled() {
        return;
    }
    let Ok(meta) = fs::symlink_metadata(path) else { return };
    if meta.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            let children: Vec<_> = entries.flatten().map(|e| e.path()).collect();
            children.par_iter().for_each(|child| count(child, progress));
        }
    } else if !meta.file_type().is_symlink() || fs::metadata(path).is_ok_and(|m| m.is_file()) {
        progress.total_files.fetch_add(1, Ordering::Relaxed);
        progress.total_bytes.fetch_add(fs::metadata(path).map_or(0, |m| m.len()), Ordering::Relaxed);
    }
}

/// Where `source` ends up in `target` – `None` if it is skipped.
fn destination(source: &Path, target: &Path, op: Op, conflict: Conflict) -> Option<PathBuf> {
    let name = source.file_name()?.to_string_lossy().into_owned();
    let dest = target.join(&name);
    if same_path(source, &dest) {
        // Copying into its own folder makes a copy next to it; moving there changes nothing.
        return (op == Op::Copy).then(|| copy_name(target, &name, source.is_dir()));
    }
    if fs::symlink_metadata(&dest).is_err() {
        return Some(dest);
    }
    match conflict {
        Conflict::Replace => Some(dest),
        Conflict::Skip => None,
        Conflict::KeepBoth => Some(crate::files::unique(target, &name)),
    }
}

/// Makes room for `source` at `dest` when replacing: a folder is merged into a folder, anything else
/// that is in the way is deleted first.
fn clear_way(source: &Path, dest: &Path) -> io::Result<()> {
    let Ok(existing) = fs::symlink_metadata(dest) else { return Ok(()) };
    if existing.is_dir() && !existing.file_type().is_symlink() && source.is_dir() {
        return Ok(());
    }
    if crate::delete::remove_all(dest) {
        Ok(())
    } else {
        Err(io::Error::other(format!("„{}“ konnte nicht ersetzt werden", dest.display())))
    }
}

fn transfer_one(source: &Path, target: &Path, op: Op, conflict: Conflict, progress: &Progress) {
    if progress.cancelled() {
        return;
    }
    if source.is_dir() && target.starts_with(source) {
        return progress.fail(source, "Ein Ordner kann nicht in sich selbst kopiert oder verschoben werden");
    }
    let Some(dest) = destination(source, target, op, conflict) else {
        progress.skipped.fetch_add(1, Ordering::Relaxed);
        return;
    };
    if let Err(e) = clear_way(source, &dest) {
        return progress.fail(&dest, e);
    }
    if op == Op::Move {
        match fs::rename(source, &dest) {
            Ok(()) => {
                progress.files.fetch_add(1, Ordering::Relaxed);
                return;
            }
            // Across drives, or into an existing folder (merge): copy, then delete.
            Err(e) if e.raw_os_error() != Some(NOT_SAME_DEVICE) && !dest.is_dir() => return progress.fail(source, e),
            Err(_) => {}
        }
    }
    let failed_before = progress.failed.load(Ordering::Relaxed);
    copy_tree(source, &dest, progress);
    let clean = progress.failed.load(Ordering::Relaxed) == failed_before && !progress.cancelled();
    if op == Op::Move && clean && !crate::delete::remove_all(source) {
        progress.fail(source, "Kopiert, aber die Quelle konnte nicht vollständig gelöscht werden");
    }
}

/// Names of `paths` that already exist in `target` (pasting into the own folder is no conflict).
#[tauri::command]
pub fn check_conflicts(paths: Vec<String>, target: String) -> Vec<String> {
    let target = PathBuf::from(target);
    paths
        .iter()
        .map(PathBuf::from)
        .filter_map(|source| {
            let name = source.file_name()?;
            let dest = target.join(name);
            (!same_path(&source, &dest) && fs::symlink_metadata(&dest).is_ok()).then(|| name.to_string_lossy().into_owned())
        })
        .collect()
}

#[derive(Default)]
pub struct Transfers {
    next_id: AtomicU64,
    running: Mutex<HashMap<u64, Arc<Progress>>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    id: u64,
    files: u64,
    bytes: u64,
    total_files: u64,
    total_bytes: u64,
    failed: u64,
    current: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoneEvent {
    id: u64,
    files: u64,
    bytes: u64,
    skipped: u64,
    failed: u64,
    failures: Vec<Failure>,
    cancelled: bool,
    elapsed_ms: u64,
}

/// Copies or moves `paths` into the folder `target` in the background and returns the job id.
/// Progress arrives as `transfer-progress` events, the end as `transfer-done`.
#[tauri::command]
pub fn start_transfer(
    app: AppHandle,
    transfers: State<'_, Transfers>,
    op: Op,
    paths: Vec<String>,
    target: String,
    conflict: Conflict,
) -> Result<u64, String> {
    let target = PathBuf::from(target);
    if !target.is_dir() {
        return Err(format!("Zielordner nicht gefunden: {}", target.display()));
    }
    let sources: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

    let id = transfers.next_id.fetch_add(1, Ordering::Relaxed) + 1;
    let progress = Arc::new(Progress::default());
    transfers.running.lock().unwrap().insert(id, progress.clone());
    let started = Instant::now();
    let done = Arc::new(AtomicBool::new(false));

    // Counting runs next to the copying instead of before it, like Explorer's "Calculating…".
    let (counter, counter_sources) = (progress.clone(), sources.clone());
    std::thread::spawn(move || counter_sources.par_iter().for_each(|s| count(s, &counter)));

    let (reporter, reporter_done, reporter_app) = (progress.clone(), done.clone(), app.clone());
    std::thread::spawn(move || {
        while !reporter_done.load(Ordering::Relaxed) {
            let _ = reporter_app.emit(
                "transfer-progress",
                ProgressEvent {
                    id,
                    files: reporter.files.load(Ordering::Relaxed),
                    bytes: reporter.bytes.load(Ordering::Relaxed),
                    total_files: reporter.total_files.load(Ordering::Relaxed),
                    total_bytes: reporter.total_bytes.load(Ordering::Relaxed),
                    failed: reporter.failed.load(Ordering::Relaxed),
                    current: reporter.current.lock().unwrap().clone(),
                },
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    });

    std::thread::spawn(move || {
        use tauri::Manager;
        sources.par_iter().for_each(|source| transfer_one(source, &target, op, conflict, &progress));
        done.store(true, Ordering::Relaxed);
        app.state::<Transfers>().running.lock().unwrap().remove(&id);
        let _ = app.emit(
            "transfer-done",
            DoneEvent {
                id,
                files: progress.files.load(Ordering::Relaxed),
                bytes: progress.bytes.load(Ordering::Relaxed),
                skipped: progress.skipped.load(Ordering::Relaxed),
                failed: progress.failed.load(Ordering::Relaxed),
                failures: progress.failures.lock().unwrap().clone(),
                cancelled: progress.cancelled(),
                elapsed_ms: started.elapsed().as_millis() as u64,
            },
        );
    });
    Ok(id)
}

/// Stops a running copy or move – what is done already stays.
#[tauri::command]
pub fn cancel_transfer(transfers: State<'_, Transfers>, id: u64) {
    if let Some(progress) = transfers.running.lock().unwrap().get(&id) {
        progress.cancel.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mrfilesys-transfer-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tree(root: &Path) {
        fs::create_dir_all(root.join("sub").join("deep")).unwrap();
        fs::write(root.join("a.txt"), "aaa").unwrap();
        fs::write(root.join("sub").join("b.txt"), "bb").unwrap();
        fs::write(root.join("sub").join("deep").join("c.txt"), "c").unwrap();
    }

    fn run(op: Op, sources: &[&Path], target: &Path, conflict: Conflict) -> Progress {
        let progress = Progress::default();
        for source in sources {
            transfer_one(source, target, op, conflict, &progress);
        }
        progress
    }

    #[test]
    fn copies_a_tree() {
        let dir = temp_dir("copy");
        let src = dir.join("src");
        tree(&src);
        fs::create_dir(dir.join("dst")).unwrap();
        let p = run(Op::Copy, &[&src], &dir.join("dst"), Conflict::Skip);
        assert_eq!(fs::read_to_string(dir.join("dst/src/sub/deep/c.txt")).unwrap(), "c");
        assert_eq!((p.files.load(Ordering::Relaxed), p.bytes.load(Ordering::Relaxed)), (3, 6));
        assert!(src.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn copy_into_own_folder_makes_a_copy() {
        let dir = temp_dir("own");
        fs::write(dir.join("x.txt"), "1").unwrap();
        fs::create_dir(dir.join("folder.d")).unwrap();
        run(Op::Copy, &[&dir.join("x.txt"), &dir.join("folder.d")], &dir, Conflict::Skip);
        run(Op::Copy, &[&dir.join("x.txt")], &dir, Conflict::Skip);
        assert!(dir.join("x - Kopie.txt").exists());
        assert!(dir.join("x - Kopie (2).txt").exists());
        assert!(dir.join("folder.d - Kopie").is_dir());
        // Moving into the own folder does nothing.
        let p = run(Op::Move, &[&dir.join("x.txt")], &dir, Conflict::Replace);
        assert_eq!(p.skipped.load(Ordering::Relaxed), 1);
        assert!(dir.join("x.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn conflicts() {
        let dir = temp_dir("conflict");
        let (src, dst) = (dir.join("src"), dir.join("dst"));
        tree(&src);
        tree(&dst.join("src"));
        fs::write(src.join("a.txt"), "new").unwrap();
        fs::write(dst.join("src").join("only-in-target.txt"), "keep").unwrap();
        assert_eq!(check_conflicts(vec![src.to_string_lossy().into()], dst.to_string_lossy().into()), ["src"]);

        let p = run(Op::Copy, &[&src], &dst, Conflict::Skip);
        assert_eq!(p.skipped.load(Ordering::Relaxed), 1);
        assert_eq!(fs::read_to_string(dst.join("src/a.txt")).unwrap(), "aaa");

        run(Op::Copy, &[&src], &dst, Conflict::KeepBoth);
        assert_eq!(fs::read_to_string(dst.join("src (2)/a.txt")).unwrap(), "new");

        // Replace merges folders: files are overwritten, others in the target stay.
        run(Op::Copy, &[&src], &dst, Conflict::Replace);
        assert_eq!(fs::read_to_string(dst.join("src/a.txt")).unwrap(), "new");
        assert!(dst.join("src/only-in-target.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replace_overwrites_read_only_files() {
        let dir = temp_dir("readonly");
        fs::write(dir.join("f.txt"), "new").unwrap();
        fs::create_dir(dir.join("dst")).unwrap();
        let old = dir.join("dst").join("f.txt");
        fs::write(&old, "old").unwrap();
        let mut permissions = fs::metadata(&old).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&old, permissions).unwrap();
        let p = run(Op::Copy, &[&dir.join("f.txt")], &dir.join("dst"), Conflict::Replace);
        assert_eq!(p.failed.load(Ordering::Relaxed), 0);
        assert_eq!(fs::read_to_string(&old).unwrap(), "new");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn moves_by_renaming_and_merges_folders() {
        let dir = temp_dir("move");
        let src = dir.join("src");
        tree(&src);
        fs::create_dir(dir.join("dst")).unwrap();
        run(Op::Move, &[&src], &dir.join("dst"), Conflict::Skip);
        assert!(!src.exists());
        assert!(dir.join("dst/src/sub/deep/c.txt").exists());

        // Move into an existing folder with "replace" merges and removes the source.
        tree(&src);
        fs::write(src.join("a.txt"), "new").unwrap();
        run(Op::Move, &[&src], &dir.join("dst"), Conflict::Replace);
        assert!(!src.exists());
        assert_eq!(fs::read_to_string(dir.join("dst/src/a.txt")).unwrap(), "new");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn refuses_to_copy_a_folder_into_itself() {
        let dir = temp_dir("itself");
        let src = dir.join("src");
        tree(&src);
        let p = run(Op::Copy, &[&src], &src.join("sub"), Conflict::KeepBoth);
        assert_eq!(p.failed.load(Ordering::Relaxed), 1);
        assert!(!src.join("sub").join("src").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn counts_files_and_bytes() {
        let dir = temp_dir("count");
        tree(&dir);
        let p = Progress::default();
        count(&dir, &p);
        assert_eq!((p.total_files.load(Ordering::Relaxed), p.total_bytes.load(Ordering::Relaxed)), (3, 6));
        fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(all(test, windows))]
mod bench {
    use super::*;

    /// `cargo test -p mrfilesys --release --lib transfer::bench -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn copy_many_small_files() {
        let base = std::env::temp_dir().join("mrfilesys-transfer-bench");
        let _ = fs::remove_dir_all(&base);
        let src = base.join("src");
        for d in 0..2000 {
            let dir = src.join(format!("pkg{d}")).join("lib");
            fs::create_dir_all(&dir).unwrap();
            for f in 0..20 {
                fs::write(dir.join(format!("file{f}.js")), "module.exports = 1;\n".repeat(20)).unwrap();
            }
        }
        let dst = base.join("dst");
        fs::create_dir(&dst).unwrap();
        let progress = Progress::default();
        let start = Instant::now();
        transfer_one(&src, &dst, Op::Copy, Conflict::Skip, &progress);
        println!(
            "mrfilesys: {} Dateien in {} ms, {} Fehler",
            progress.files.load(Ordering::Relaxed),
            start.elapsed().as_millis(),
            progress.failed.load(Ordering::Relaxed)
        );
        let _ = fs::remove_dir_all(&base);
    }
}
