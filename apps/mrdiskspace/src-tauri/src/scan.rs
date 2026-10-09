use rayon::prelude::*;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

/// Children beyond this count are not sent to the frontend (they are the smallest ones).
const MAX_CHILDREN: usize = 1000;

/// A file or folder of a finished scan. Folders hold the totals of their contents.
#[derive(Default)]
pub struct Node {
    name: Box<str>,
    size: u64,
    /// Last modification in ms since 1970; for folders the newest of their contents.
    modified: i64,
    files: u64,
    dirs: u64,
    /// Entries that could not be read (usually access denied).
    errors: u64,
    is_dir: bool,
    /// Sorted by size, largest first.
    children: Vec<Node>,
}

struct ScanResult {
    root: PathBuf,
    tree: Node,
}

#[derive(Default)]
pub struct Scanner {
    result: Mutex<Option<Arc<ScanResult>>>,
    cancel: Mutex<Arc<AtomicBool>>,
    next_id: AtomicU64,
}

/// Shared between the scanning threads.
#[derive(Default)]
struct Progress {
    files: AtomicU64,
    dirs: AtomicU64,
    bytes: AtomicU64,
    errors: AtomicU64,
    current: Mutex<String>,
    cancel: Arc<AtomicBool>,
}

fn millis(meta: &Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64)
}

fn scan_dir(path: &Path, name: Box<str>, own_modified: i64, progress: &Progress) -> Node {
    let mut node = Node { name, is_dir: true, modified: own_modified, ..Default::default() };
    if progress.cancel.load(Ordering::Relaxed) {
        return node;
    }
    progress.dirs.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut current) = progress.current.try_lock() {
        current.clear();
        current.push_str(&path.to_string_lossy());
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => {
            node.errors = 1;
            progress.errors.fetch_add(1, Ordering::Relaxed);
            return node;
        }
    };

    let mut subdirs = Vec::new();
    for entry in entries {
        // On Windows the metadata comes from the directory listing itself (no extra system call)
        // and does not follow links.
        let Ok((entry, meta)) = entry.and_then(|e| e.metadata().map(|m| (e, m))) else {
            node.errors += 1;
            progress.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        // Symbolic links and junctions point to data that is counted elsewhere (or on another drive).
        if meta.file_type().is_symlink() {
            continue;
        }
        let name: Box<str> = entry.file_name().to_string_lossy().into();
        if meta.is_dir() {
            subdirs.push((entry.path(), name, millis(&meta)));
        } else {
            let size = meta.len();
            progress.files.fetch_add(1, Ordering::Relaxed);
            progress.bytes.fetch_add(size, Ordering::Relaxed);
            node.files += 1;
            node.size += size;
            node.modified = node.modified.max(millis(&meta));
            node.children.push(Node { name, size, modified: millis(&meta), ..Default::default() });
        }
    }

    let dirs: Vec<Node> = subdirs
        .into_par_iter()
        .map(|(path, name, modified)| scan_dir(&path, name, modified, progress))
        .collect();
    for dir in &dirs {
        node.size += dir.size;
        node.files += dir.files;
        node.dirs += dir.dirs + 1;
        node.errors += dir.errors;
        node.modified = node.modified.max(dir.modified);
    }
    node.children.extend(dirs);
    node.children.sort_unstable_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    node
}

fn scan_root(root: &Path, progress: &Progress) -> Node {
    let modified = fs::metadata(root).map(|m| millis(&m)).unwrap_or(0);
    scan_dir(root, root.to_string_lossy().into(), modified, progress)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    id: u64,
    files: u64,
    dirs: u64,
    bytes: u64,
    errors: u64,
    current: String,
    elapsed_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoneEvent {
    id: u64,
    cancelled: bool,
    elapsed_ms: u64,
}

/// Starts scanning `path` in the background (a running scan is cancelled) and returns the scan id.
/// Progress arrives as `scan-progress` events, the end as `scan-done`.
#[tauri::command]
pub fn start_scan(app: AppHandle, scanner: State<'_, Scanner>, path: String) -> Result<u64, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("Ordner nicht gefunden: {path}"));
    }
    let id = scanner.next_id.fetch_add(1, Ordering::Relaxed) + 1;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut current = scanner.cancel.lock().unwrap();
        current.store(true, Ordering::Relaxed);
        *current = cancel.clone();
    }

    let progress = Arc::new(Progress { cancel, ..Default::default() });
    let done = Arc::new(AtomicBool::new(false));
    let started = Instant::now();

    let (reporter_progress, reporter_done, reporter_app) = (progress.clone(), done.clone(), app.clone());
    std::thread::spawn(move || {
        while !reporter_done.load(Ordering::Relaxed) {
            let p = &reporter_progress;
            let _ = reporter_app.emit(
                "scan-progress",
                ProgressEvent {
                    id,
                    files: p.files.load(Ordering::Relaxed),
                    dirs: p.dirs.load(Ordering::Relaxed),
                    bytes: p.bytes.load(Ordering::Relaxed),
                    errors: p.errors.load(Ordering::Relaxed),
                    current: p.current.lock().unwrap().clone(),
                    elapsed_ms: started.elapsed().as_millis() as u64,
                },
            );
            std::thread::sleep(Duration::from_millis(150));
        }
    });

    std::thread::spawn(move || {
        use tauri::Manager;
        let tree = scan_root(&root, &progress);
        done.store(true, Ordering::Relaxed);
        let cancelled = progress.cancel.load(Ordering::Relaxed);
        if !cancelled {
            *app.state::<Scanner>().result.lock().unwrap() = Some(Arc::new(ScanResult { root, tree }));
        }
        let elapsed_ms = started.elapsed().as_millis() as u64;
        let _ = app.emit("scan-done", DoneEvent { id, cancelled, elapsed_ms });
    });
    Ok(id)
}

#[tauri::command]
pub fn cancel_scan(scanner: State<'_, Scanner>) {
    scanner.cancel.lock().unwrap().store(true, Ordering::Relaxed);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    name: String,
    size: u64,
    modified: i64,
    files: u64,
    dirs: u64,
    errors: u64,
    is_dir: bool,
    has_children: bool,
}

impl From<&Node> for Entry {
    fn from(n: &Node) -> Self {
        Entry {
            name: n.name.to_string(),
            size: n.size,
            modified: n.modified,
            files: n.files,
            dirs: n.dirs,
            errors: n.errors,
            is_dir: n.is_dir,
            has_children: !n.children.is_empty(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeView {
    #[serde(flatten)]
    entry: Entry,
    /// Full file system path.
    path: String,
    children: Vec<Entry>,
    /// Number of (small) children not included in `children`.
    omitted: usize,
}

fn current(scanner: &Scanner) -> Result<Arc<ScanResult>, String> {
    scanner.result.lock().unwrap().clone().ok_or_else(|| "Kein Scan vorhanden".into())
}

/// Resolves an index path (child indices from the root) to the node and its file system path.
fn resolve<'a>(scan: &'a ScanResult, index: &[u32]) -> Result<(&'a Node, PathBuf), String> {
    let mut node = &scan.tree;
    let mut path = scan.root.clone();
    for &i in index {
        node = node.children.get(i as usize).ok_or("Eintrag nicht gefunden")?;
        path.push(&*node.name);
    }
    Ok((node, path))
}

#[tauri::command]
pub fn get_node(scanner: State<'_, Scanner>, index: Vec<u32>) -> Result<NodeView, String> {
    let scan = current(&scanner)?;
    let (node, path) = resolve(&scan, &index)?;
    Ok(NodeView {
        entry: node.into(),
        path: path.to_string_lossy().into_owned(),
        children: node.children.iter().take(MAX_CHILDREN).map(Entry::from).collect(),
        omitted: node.children.len().saturating_sub(MAX_CHILDREN),
    })
}

#[derive(Serialize, Debug, PartialEq)]
pub struct TypeStat {
    /// Lower-case extension without dot, empty for files without one.
    ext: String,
    size: u64,
    count: u64,
}

fn extension(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => name[i + 1..].to_lowercase(),
        _ => String::new(),
    }
}

fn collect_types(node: &Node, stats: &mut HashMap<String, (u64, u64)>) {
    for child in &node.children {
        if child.is_dir {
            collect_types(child, stats);
        } else {
            let stat = stats.entry(extension(&child.name)).or_default();
            stat.0 += child.size;
            stat.1 += 1;
        }
    }
}

/// Space per file extension below the node, largest first (at most `limit`).
#[tauri::command]
pub fn file_types(scanner: State<'_, Scanner>, index: Vec<u32>, limit: usize) -> Result<Vec<TypeStat>, String> {
    let scan = current(&scanner)?;
    let (node, _) = resolve(&scan, &index)?;
    let mut stats = HashMap::new();
    collect_types(node, &mut stats);
    let mut list: Vec<TypeStat> =
        stats.into_iter().map(|(ext, (size, count))| TypeStat { ext, size, count }).collect();
    list.sort_unstable_by(|a, b| b.size.cmp(&a.size).then_with(|| a.ext.cmp(&b.ext)));
    list.truncate(limit);
    Ok(list)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHit {
    name: String,
    path: String,
    size: u64,
    modified: i64,
    /// Index path from the scan root, usable with `get_node` for its parent.
    index: Vec<u32>,
}

fn collect_largest(node: &Node, index: &mut Vec<u32>, limit: usize, heap: &mut BinaryHeap<Reverse<(u64, Vec<u32>)>>) {
    for (i, child) in node.children.iter().enumerate() {
        // Children are sorted by size: once one is too small, all following ones are too.
        if heap.len() == limit && heap.peek().is_some_and(|Reverse((min, _))| child.size <= *min) {
            break;
        }
        index.push(i as u32);
        if child.is_dir {
            collect_largest(child, index, limit, heap);
        } else {
            heap.push(Reverse((child.size, index.clone())));
            if heap.len() > limit {
                heap.pop();
            }
        }
        index.pop();
    }
}

/// Sizes and index paths of the `limit` largest files below `node` (at `index`), largest first.
fn largest(node: &Node, mut index: Vec<u32>, limit: usize) -> Vec<(u64, Vec<u32>)> {
    let mut heap = BinaryHeap::new();
    collect_largest(node, &mut index, limit.max(1), &mut heap);
    // Ascending order of `Reverse` is descending order of size.
    heap.into_sorted_vec().into_iter().map(|Reverse(hit)| hit).collect()
}

/// The `limit` largest files below the node, largest first.
#[tauri::command]
pub fn largest_files(scanner: State<'_, Scanner>, index: Vec<u32>, limit: usize) -> Result<Vec<FileHit>, String> {
    let scan = current(&scanner)?;
    let (node, _) = resolve(&scan, &index)?;
    largest(node, index, limit)
        .into_iter()
        .map(|(_, index)| {
            let (file, path) = resolve(&scan, &index)?;
            Ok(FileHit {
                name: file.name.to_string(),
                path: path.to_string_lossy().into_owned(),
                size: file.size,
                modified: file.modified,
                index,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mrdiskspace-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("big/inner")).unwrap();
        fs::create_dir_all(dir.join("empty")).unwrap();
        fs::write(dir.join("a.txt"), vec![0u8; 10]).unwrap();
        fs::write(dir.join("big/b.BIN"), vec![0u8; 300]).unwrap();
        fs::write(dir.join("big/inner/c.bin"), vec![0u8; 200]).unwrap();
        fs::write(dir.join("big/inner/noext"), vec![0u8; 5]).unwrap();
        dir
    }

    #[test]
    fn scans_totals_and_sorts_by_size() {
        let dir = fixture();
        let tree = scan_root(&dir, &Progress::default());
        assert_eq!(tree.size, 515);
        assert_eq!(tree.files, 4);
        assert_eq!(tree.dirs, 3);
        let names: Vec<_> = tree.children.iter().map(|c| &*c.name).collect();
        assert_eq!(names, ["big", "a.txt", "empty"]);
        assert_eq!(tree.children[0].size, 505);
        assert_eq!(tree.children[0].children[0].name.as_ref(), "b.BIN");

        let scan = ScanResult { root: dir.clone(), tree };
        let mut stats = HashMap::new();
        collect_types(&scan.tree, &mut stats);
        assert_eq!(stats["bin"], (500, 2));
        assert_eq!(stats[""], (5, 1));

        let top = largest(&scan.tree, Vec::new(), 2);
        assert_eq!(top, [(300, vec![0, 0]), (200, vec![0, 1, 0])]);

        let (node, path) = resolve(&scan, &[0, 1]).unwrap();
        assert_eq!(&*node.name, "inner");
        assert_eq!(path, dir.join("big").join("inner"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cancelled_scan_stops_early() {
        let progress = Progress::default();
        progress.cancel.store(true, Ordering::Relaxed);
        let tree = scan_root(&std::env::temp_dir(), &progress);
        assert!(tree.children.is_empty());
    }

    #[test]
    fn extensions() {
        assert_eq!(extension("x.TXT"), "txt");
        assert_eq!(extension(".gitignore"), "");
        assert_eq!(extension("archive.tar.gz"), "gz");
        assert_eq!(extension("trailing."), "");
    }
}
