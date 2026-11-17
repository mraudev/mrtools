//! Search over all file names on the local drives. Reading the NTFS master file table directly (like
//! "Everything") needs administrator rights, so the app keeps its own index of names instead: built in
//! parallel in the background, kept current with the changes Windows reports while the app runs
//! (`watch.rs`), saved to disk now and then and built again at startup when the saved one is older
//! than `MAX_AGE`.

use crate::watch::{self, Change};
use memchr::memmem;
use rayon::prelude::*;
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, OnceLock, RwLock,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};

/// A saved index older than this is built again when the app starts – what changed while the app was
/// closed is not known otherwise.
const MAX_AGE: Duration = Duration::from_secs(10 * 60);
/// How often a changed index is saved.
const SAVE_EVERY: Duration = Duration::from_secs(5 * 60);
const NONE: u32 = u32::MAX;
const MAGIC: &[u8; 8] = b"MRFSIDX2";

/// Folders that are left out: deleted files with cryptic names, and nothing readable anyway.
fn skipped(name: &str) -> bool {
    name.eq_ignore_ascii_case("$Recycle.Bin") || name.eq_ignore_ascii_case("System Volume Information")
}

fn has_skipped(path: &Path) -> bool {
    path.components().any(|c| skipped(&c.as_os_str().to_string_lossy()))
}

/// All names in a few flat arrays – a few dozen bytes per entry instead of a full path. Entries can be
/// added, renamed and removed; removed ones (and old names) stay as gaps until the index is compacted.
#[derive(Default)]
pub struct Index {
    /// Names as written and in lower case – a piece is appended each time an entry gets a name.
    names: String,
    lower: String,
    /// Per piece, in the order appended: its start in `names` and `lower`, and the entry it named.
    piece_name: Vec<u32>,
    piece_lower: Vec<u32>,
    piece_entry: Vec<u32>,
    /// Per entry: its current piece, the tree, and flags.
    piece: Vec<u32>,
    parent: Vec<u32>,
    first_child: Vec<u32>,
    next_sibling: Vec<u32>,
    is_dir: Vec<bool>,
    removed: Vec<bool>,
    removed_count: usize,
    /// Entries without parent: the drives (or the folder of a folder tree).
    roots: Vec<u32>,
    /// ms since 1970.
    built_at: u64,
}

impl Index {
    /// Entries including removed ones – ids go up to this.
    fn len(&self) -> usize {
        self.parent.len()
    }

    fn live(&self) -> usize {
        self.len() - self.removed_count
    }

    fn piece_range(starts: &[u32], text: &str, piece: usize) -> std::ops::Range<usize> {
        starts[piece] as usize..starts.get(piece + 1).map_or(text.len(), |&e| e as usize)
    }

    fn name(&self, id: usize) -> &str {
        &self.names[Self::piece_range(&self.piece_name, &self.names, self.piece[id] as usize)]
    }

    fn lower_name(&self, id: usize) -> &str {
        &self.lower[Self::piece_range(&self.piece_lower, &self.lower, self.piece[id] as usize)]
    }

    fn set_name(&mut self, id: u32, name: &str) {
        let piece = self.piece_entry.len() as u32;
        self.piece_name.push(self.names.len() as u32);
        self.names.push_str(name);
        self.piece_lower.push(self.lower.len() as u32);
        self.lower.push_str(&name.to_lowercase());
        self.piece_entry.push(id);
        match self.piece.get_mut(id as usize) {
            Some(p) => *p = piece,
            None => self.piece.push(piece),
        }
    }

    fn push(&mut self, name: &str, parent: u32, is_dir: bool) -> u32 {
        let id = self.len() as u32;
        self.set_name(id, name);
        self.parent.push(NONE);
        self.first_child.push(NONE);
        self.next_sibling.push(NONE);
        self.is_dir.push(is_dir);
        self.removed.push(false);
        self.link(id, parent);
        id
    }

    fn link(&mut self, id: u32, parent: u32) {
        self.parent[id as usize] = parent;
        if parent == NONE {
            self.roots.push(id);
        } else {
            self.next_sibling[id as usize] = self.first_child[parent as usize];
            self.first_child[parent as usize] = id;
        }
    }

    fn unlink(&mut self, id: u32) {
        let parent = self.parent[id as usize];
        let next = self.next_sibling[id as usize];
        if parent == NONE {
            self.roots.retain(|&r| r != id);
        } else if self.first_child[parent as usize] == id {
            self.first_child[parent as usize] = next;
        } else {
            let mut c = self.first_child[parent as usize];
            while c != NONE {
                if self.next_sibling[c as usize] == id {
                    self.next_sibling[c as usize] = next;
                    break;
                }
                c = self.next_sibling[c as usize];
            }
        }
        self.next_sibling[id as usize] = NONE;
    }

    fn children(&self, id: u32) -> impl Iterator<Item = u32> + '_ {
        let some = |c: u32| (c != NONE).then_some(c);
        std::iter::successors(some(self.first_child[id as usize]), move |&c| some(self.next_sibling[c as usize]))
    }

    fn child(&self, parent: u32, lower_name: &str) -> Option<u32> {
        self.children(parent).find(|&c| self.lower_name(c as usize) == lower_name)
    }

    /// Removes an entry with everything below it.
    fn remove(&mut self, id: u32) {
        self.unlink(id);
        let mut stack = vec![id];
        while let Some(x) = stack.pop() {
            self.removed[x as usize] = true;
            self.removed_count += 1;
            stack.extend(self.children(x));
        }
    }

    /// Id of a file or folder by its path.
    fn resolve(&self, path: &str) -> Option<u32> {
        let target = normalized(path);
        self.roots.iter().find_map(|&root| {
            let root_name = normalized(self.lower_name(root as usize));
            if !covers(&root_name, &target) {
                return None;
            }
            target[root_name.len()..]
                .split('\\')
                .filter(|part| !part.is_empty())
                .try_fold(root, |id, part| self.child(id, part))
        })
    }

    fn path(&self, id: usize) -> PathBuf {
        let mut parts = vec![self.name(id)];
        let mut p = self.parent[id];
        while p != NONE {
            parts.push(self.name(p as usize));
            p = self.parent[p as usize];
        }
        parts.iter().rev().collect()
    }

    fn is_below(&self, id: usize, folder: u32) -> bool {
        let mut p = self.parent[id];
        while p != NONE {
            if p == folder {
                return true;
            }
            p = self.parent[p as usize];
        }
        false
    }

    /// Visits the live entries depth first, parents before children, with the parent's number in this
    /// order – the order a compacted index has.
    fn depth_first(&self, mut visit: impl FnMut(&str, u32, bool)) {
        let mut stack: Vec<(u32, u32)> = self.roots.iter().rev().map(|&r| (r, NONE)).collect();
        let mut next = 0u32;
        while let Some((id, parent)) = stack.pop() {
            visit(self.name(id as usize), parent, self.is_dir[id as usize]);
            stack.extend(self.children(id).map(|c| (c, next)));
            next += 1;
        }
    }

    /// The same entries without gaps.
    fn compacted(&self) -> Index {
        let mut out = Index { built_at: self.built_at, ..Default::default() };
        self.depth_first(|name, parent, is_dir| {
            out.push(name, parent, is_dir);
        });
        out
    }

    /// Old names and removed entries take more than a quarter of the space.
    fn wasteful(&self) -> bool {
        let waste = self.piece_entry.len() - self.live();
        waste > 100_000 && waste > self.live() / 4
    }
}

// ---------------------------------------------------------------------------
// Building

struct Node {
    name: String,
    is_dir: bool,
    children: Vec<Node>,
}

/// Reads a folder tree in parallel. Links to folders are listed but not followed.
fn scan(path: &Path, name: String, scanned: &AtomicU64) -> Node {
    let children = match fs::read_dir(path) {
        Ok(entries) => {
            let entries: Vec<_> = entries.flatten().collect();
            scanned.fetch_add(entries.len() as u64, Ordering::Relaxed);
            entries
                .par_iter()
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let kind = entry.file_type().ok()?;
                    if kind.is_dir() && !kind.is_symlink() {
                        if skipped(&name) {
                            return None;
                        }
                        Some(scan(&entry.path(), name, scanned))
                    } else {
                        let is_dir = kind.is_symlink() && entry.path().is_dir();
                        Some(Node { name, is_dir, children: Vec::new() })
                    }
                })
                .collect()
        }
        Err(_) => Vec::new(),
    };
    Node { name, is_dir: true, children }
}

fn flatten(index: &mut Index, node: Node, parent: u32) {
    let id = index.push(&node.name, parent, node.is_dir);
    for child in node.children {
        flatten(index, child, id);
    }
}

pub fn build(roots: &[String], scanned: &AtomicU64) -> Index {
    let trees: Vec<Node> = roots.par_iter().map(|root| scan(Path::new(root), root.clone(), scanned)).collect();
    let mut index = Index::default();
    for tree in trees {
        flatten(&mut index, tree, NONE);
    }
    index.built_at = now_ms();
    index
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

// ---------------------------------------------------------------------------
// Keeping it current

/// A change, ready to be applied (a folder that appeared has been read already).
enum Op {
    Add(PathBuf, Node),
    Remove(PathBuf),
    Rename(PathBuf, PathBuf),
}

/// Reads what is needed for a change – outside of the index lock.
fn prepare(change: Change) -> Option<Op> {
    match change {
        Change::Added(path) => {
            if has_skipped(&path) {
                return None;
            }
            // Gone again already: nothing to add.
            let meta = fs::symlink_metadata(&path).ok()?;
            let name = path.file_name()?.to_string_lossy().into_owned();
            let link = meta.file_type().is_symlink();
            let node = if meta.is_dir() && !link {
                // Usually empty – unless it was moved here with all its content.
                scan(&path, name, &AtomicU64::new(0))
            } else {
                Node { name, is_dir: meta.is_dir() || (link && path.is_dir()), children: Vec::new() }
            };
            Some(Op::Add(path, node))
        }
        Change::Removed(path) => Some(Op::Remove(path)),
        Change::Renamed(from, to) => Some(Op::Rename(from, to)),
    }
}

fn parent_of(path: &Path) -> Option<String> {
    Some(path.parent()?.to_string_lossy().into_owned())
}

/// Applies a change; changes can arrive twice (e.g. while the index was rebuilt), so each one checks
/// first. Returns a path to add when a renamed entry was not known under its old name.
fn apply(index: &mut Index, op: Op) -> Option<PathBuf> {
    match op {
        Op::Add(path, node) => {
            let parent = index.resolve(&parent_of(&path)?)?;
            if index.child(parent, &node.name.to_lowercase()).is_none() {
                flatten(index, node, parent);
            }
        }
        Op::Remove(path) => {
            if let Some(id) = index.resolve(&path.to_string_lossy()).filter(|&id| index.parent[id as usize] != NONE) {
                index.remove(id);
            }
        }
        Op::Rename(from, to) => {
            let Some(id) = index.resolve(&from.to_string_lossy()) else {
                return Some(to);
            };
            let name = to.file_name()?.to_string_lossy().into_owned();
            let parent = parent_of(&to).and_then(|p| index.resolve(&p)).filter(|_| !has_skipped(&to));
            match parent {
                // Moved out of sight, e.g. into the recycle bin.
                None => index.remove(id),
                Some(parent) => {
                    if let Some(existing) = index.child(parent, &name.to_lowercase()).filter(|&e| e != id) {
                        index.remove(existing);
                    }
                    index.unlink(id);
                    index.set_name(id, &name);
                    index.link(id, parent);
                }
            }
        }
    }
    None
}

/// Applies changes Windows reported to the drive index – or keeps them for the index being built.
fn apply_changes(search: &Search, changes: Vec<Change>) {
    let ops: Vec<Op> = changes.into_par_iter().filter_map(prepare).collect();
    let unknown: Vec<PathBuf> = {
        let mut pending = search.pending.lock().unwrap();
        if search.building.load(Ordering::Relaxed) {
            pending.extend(ops);
            return;
        }
        let mut guard = search.index.write().unwrap();
        let Some(index) = guard.as_mut() else { return };
        let unknown = ops.into_iter().filter_map(|op| apply(index, op)).collect();
        if index.wasteful() {
            *index = index.compacted();
        }
        unknown
    };
    search.dirty.store(true, Ordering::Relaxed);
    if !unknown.is_empty() {
        apply_changes(search, unknown.into_iter().map(Change::Added).collect());
    }
}

// ---------------------------------------------------------------------------
// Cache file

/// Writes the live entries compacted; `known_at` is when the index was last known to be complete.
fn save(index: &Index, file: &Path, known_at: u64) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("tmp");
    let mut out = BufWriter::new(fs::File::create(&tmp)?);
    out.write_all(MAGIC)?;
    out.write_all(&known_at.to_le_bytes())?;
    out.write_all(&(index.live() as u64).to_le_bytes())?;
    let mut result = Ok(());
    index.depth_first(|name, parent, is_dir| {
        if result.is_ok() {
            let name = &name.as_bytes()[..name.len().min(u16::MAX as usize)];
            result = out
                .write_all(&parent.to_le_bytes())
                .and_then(|_| out.write_all(&[is_dir as u8]))
                .and_then(|_| out.write_all(&(name.len() as u16).to_le_bytes()))
                .and_then(|_| out.write_all(name));
        }
    });
    result?;
    out.into_inner()?.sync_all()?;
    fs::rename(tmp, file)
}

fn load(file: &Path) -> Option<Index> {
    let data = fs::read(file).ok()?;
    let mut pos = 0;
    let mut take = |n: usize| -> Option<&[u8]> {
        let slice = data.get(pos..pos + n)?;
        pos += n;
        Some(slice)
    };
    if take(8)? != MAGIC {
        return None;
    }
    let built_at = u64::from_le_bytes(take(8)?.try_into().ok()?);
    let count = u64::from_le_bytes(take(8)?.try_into().ok()?) as usize;
    let mut index = Index { built_at, ..Default::default() };
    for i in 0..count {
        let parent = u32::from_le_bytes(take(4)?.try_into().ok()?);
        let is_dir = take(1)?[0] != 0;
        let len = u16::from_le_bytes(take(2)?.try_into().ok()?) as usize;
        let name = std::str::from_utf8(take(len)?).ok()?;
        // A damaged file must not make the tree inconsistent.
        if parent != NONE && parent as usize >= i {
            return None;
        }
        index.push(name, parent, is_dir);
    }
    Some(index)
}

// ---------------------------------------------------------------------------
// Searching

enum Term {
    /// Part of the name.
    Text(String),
    /// `*` and `?` – the whole name must match, e.g. `*.pdf`.
    Pattern(Vec<char>),
    /// Contains `\` – part of the full path.
    Path(String),
}

fn parse(query: &str) -> Vec<Term> {
    query
        .split_whitespace()
        .map(|t| t.to_lowercase())
        .map(|t| {
            if t.contains('\\') {
                Term::Path(t)
            } else if t.contains(['*', '?']) {
                Term::Pattern(t.chars().collect())
            } else {
                Term::Text(t)
            }
        })
        .collect()
}

/// Wildcard match of a whole name (`*` any text, `?` one character).
fn wildcard(pattern: &[char], name: &str) -> bool {
    let name: Vec<char> = name.chars().collect();
    let (mut p, mut n, mut star, mut mark) = (0, 0, usize::MAX, 0);
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = p;
            mark = n;
            p += 1;
        } else if star != usize::MAX {
            p = star + 1;
            mark += 1;
            n = mark;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|&c| c == '*')
}

fn matches(index: &Index, id: usize, terms: &[Term]) -> bool {
    let name = index.lower_name(id);
    terms.iter().all(|term| match term {
        Term::Text(t) => name.contains(t.as_str()),
        Term::Pattern(p) => wildcard(p, name),
        Term::Path(t) => index.path(id).to_string_lossy().to_lowercase().contains(t.as_str()),
    })
}

#[cfg(test)]
fn find(index: &Index, query: &str) -> Vec<u32> {
    find_in(index, query, None)
}

/// Ids of the entries (below `folder`, if given) matching `query`, best first: exact name, name starting
/// with the first term, shorter names.
fn find_in(index: &Index, query: &str, folder: Option<u32>) -> Vec<u32> {
    let terms = parse(query);
    if terms.is_empty() {
        return Vec::new();
    }
    let wanted = |id: usize| {
        !index.removed[id] && folder.is_none_or(|f| index.is_below(id, f)) && matches(index, id, &terms)
    };
    // The longest plain term is searched in the whole name buffer at once; the rest is checked per hit.
    let anchor = terms
        .iter()
        .filter_map(|t| if let Term::Text(t) = t { Some(t.as_str()) } else { None })
        .max_by_key(|t| t.len());
    let mut ids: Vec<u32> = match anchor {
        Some(anchor) => {
            let mut ids = Vec::new();
            let mut last = usize::MAX;
            for pos in memmem::find_iter(index.lower.as_bytes(), anchor.as_bytes()) {
                let piece = index.piece_lower.partition_point(|&o| o as usize <= pos) - 1;
                let id = index.piece_entry[piece] as usize;
                // Old names do not count, neither do hits across two names.
                let current = index.piece[id] as usize == piece;
                let end = Index::piece_range(&index.piece_lower, &index.lower, piece).end;
                if current && id != last && pos + anchor.len() <= end && wanted(id) {
                    ids.push(id as u32);
                    last = id;
                }
            }
            ids
        }
        None => (0..index.len()).into_par_iter().filter(|&id| wanted(id)).map(|id| id as u32).collect(),
    };
    let first = match &terms[0] {
        Term::Text(t) | Term::Path(t) => t.clone(),
        Term::Pattern(_) => String::new(),
    };
    ids.par_sort_by_cached_key(|&id| {
        let name = index.lower_name(id as usize);
        let rank = if name == first {
            0
        } else if !first.is_empty() && name.starts_with(&first) {
            1
        } else {
            2
        };
        (rank, name.len(), id)
    });
    ids
}

/// Lower case, `C:\` keeps its backslash, `C:\Users\` loses it.
fn normalized(dir: &str) -> String {
    let mut dir = dir.to_lowercase();
    if dir.len() > 3 && dir.ends_with('\\') {
        dir.pop();
    }
    dir
}

/// `dir` is `root` or below it (both normalized).
fn covers(root: &str, dir: &str) -> bool {
    dir == root || dir.strip_prefix(root).is_some_and(|rest| root.ends_with('\\') || rest.starts_with('\\'))
}

/// Folder trees kept – the most recent last.
const FOLDER_CACHE: usize = 3;

/// Trees of folders searched lately that the drive index does not cover (USB sticks, network drives):
/// read when a search there starts, then the search answers from what was read and reads the folder
/// again in the background.
#[derive(Default)]
struct Folders {
    /// Normalized root and tree, the most recent last.
    cache: Mutex<Vec<(String, Arc<Index>)>>,
    /// Trees being read – searches arriving meanwhile wait for the same read instead of starting one.
    reading: Mutex<HashMap<String, Arc<OnceLock<Arc<Index>>>>>,
}

impl Folders {
    /// Reads `dir` (or waits for a read already under way) and keeps the tree.
    fn read(&self, dir: &str) -> Arc<Index> {
        let target = normalized(dir);
        let cell = self.reading.lock().unwrap().entry(target.clone()).or_default().clone();
        let index = cell.get_or_init(|| Arc::new(build(&[dir.to_owned()], &AtomicU64::new(0)))).clone();
        let mut cache = self.cache.lock().unwrap();
        if !cache.iter().any(|(_, i)| Arc::ptr_eq(i, &index)) {
            // Replaces older trees of the folder and its subfolders.
            cache.retain(|(root, _)| !covers(&target, root));
            cache.push((target.clone(), index.clone()));
            if cache.len() > FOLDER_CACHE {
                cache.remove(0);
            }
        }
        let mut reading = self.reading.lock().unwrap();
        if reading.get(&target).is_some_and(|c| Arc::ptr_eq(c, &cell)) {
            reading.remove(&target);
        }
        index
    }

    fn is_reading(&self, target: &str) -> bool {
        self.reading.lock().unwrap().contains_key(target)
    }

    /// The most specific tree read that contains `target`, and the folder's id in it.
    fn cached(&self, target: &str) -> Option<(Arc<Index>, u32)> {
        let index = {
            let cache = self.cache.lock().unwrap();
            let (_, index) =
                cache.iter().filter(|(root, _)| covers(root, target)).max_by_key(|(root, _)| root.len())?;
            index.clone()
        };
        let id = index.resolve(target)?;
        Some((index, id))
    }
}

/// The tree and folder id for searching below `dir` (not in the drive index), and whether a fresh read
/// is still under way. Answers at once from a tree read before; a folder not read yet is read right
/// away. `fresh` (a new search) reads the folder again in the background and calls `done` when that is
/// finished, so the search can be repeated.
fn folder_search(
    folders: &Arc<Folders>,
    dir: &str,
    fresh: bool,
    done: impl FnOnce() + Send + 'static,
) -> (Arc<Index>, u32, bool) {
    let target = normalized(dir);
    let Some((index, id)) = folders.cached(&target) else {
        let index = folders.read(dir);
        let id = index.resolve(dir).unwrap_or(0);
        return (index, id, false);
    };
    if fresh && !folders.is_reading(&target) {
        let (folders, dir) = (folders.clone(), dir.to_owned());
        // Marked as reading before the thread runs, so the answer below already says so.
        folders.reading.lock().unwrap().entry(target.clone()).or_default();
        std::thread::spawn(move || {
            folders.read(&dir);
            done();
        });
    }
    let refreshing = folders.is_reading(&target);
    (index, id, refreshing)
}

// ---------------------------------------------------------------------------
// Commands

#[derive(Default)]
pub struct Search {
    index: RwLock<Option<Index>>,
    building: AtomicBool,
    scanned: AtomicU64,
    cache_file: Mutex<Option<PathBuf>>,
    folders: Arc<Folders>,
    /// Changes that arrived while the index was being built – applied to the new one.
    pending: Mutex<Vec<Op>>,
    /// Changed since last saved.
    dirty: AtomicBool,
    /// Changes on the drives are followed.
    watching: AtomicBool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Entries in the index, 0 if there is none yet.
    entries: u64,
    built_at: u64,
    building: bool,
    /// Entries read so far while building.
    scanned: u64,
    /// Kept current with the changes on the drives.
    watching: bool,
}

fn status(search: &Search) -> Status {
    let index = search.index.read().unwrap();
    Status {
        entries: index.as_ref().map_or(0, |i| i.live() as u64),
        built_at: index.as_ref().map_or(0, |i| i.built_at),
        building: search.building.load(Ordering::Relaxed),
        scanned: search.scanned.load(Ordering::Relaxed),
        watching: search.watching.load(Ordering::Relaxed),
    }
}

/// Builds the index in the background (unless that is running already) and reports via
/// `index-status` events.
fn rebuild(app: &AppHandle) {
    let search = app.state::<Search>();
    if search.building.swap(true, Ordering::Relaxed) {
        return;
    }
    search.scanned.store(0, Ordering::Relaxed);
    let app = app.clone();
    std::thread::spawn(move || {
        let search = app.state::<Search>();
        let reporter_app = app.clone();
        let done = Arc::new(AtomicBool::new(false));
        let reporter_done = done.clone();
        std::thread::spawn(move || {
            while !reporter_done.load(Ordering::Relaxed) {
                let _ = reporter_app.emit("index-status", status(&reporter_app.state::<Search>()));
                std::thread::sleep(Duration::from_millis(250));
            }
        });

        let mut index = build(&crate::drives::fixed_roots(), &search.scanned);
        let unknown: Vec<PathBuf> = {
            // Changes made while reading may be missing – they were kept and are applied now.
            let mut pending = search.pending.lock().unwrap();
            let unknown = pending.drain(..).filter_map(|op| apply(&mut index, op)).collect();
            *search.index.write().unwrap() = Some(index);
            search.building.store(false, Ordering::Relaxed);
            unknown
        };
        if !unknown.is_empty() {
            apply_changes(&search, unknown.into_iter().map(Change::Added).collect());
        }
        if let Some(file) = search.cache_file.lock().unwrap().clone() {
            if let Some(index) = search.index.read().unwrap().as_ref() {
                let _ = save(index, &file, index.built_at);
            }
        }
        done.store(true, Ordering::Relaxed);
        let _ = app.emit("index-status", status(&search));
    });
}

/// Follows the changes on all local drives. When Windows drops changes (too many at once), the index is
/// built again.
fn start_watching(app: &AppHandle) {
    let mut any = false;
    for root in crate::drives::fixed_roots() {
        let (changes_app, lost_app) = (app.clone(), app.clone());
        any |= watch::watch(
            root,
            move |changes| apply_changes(&changes_app.state::<Search>(), changes),
            move || rebuild(&lost_app),
        );
    }
    app.state::<Search>().watching.store(any, Ordering::Relaxed);
}

/// Loads the saved index, follows the changes on the drives, renews the index if it is missing or old,
/// and saves it now and then. Called once at startup.
pub fn init(app: &AppHandle) {
    let file = app.path().app_cache_dir().ok().map(|dir| dir.join("search-index.bin"));
    *app.state::<Search>().cache_file.lock().unwrap() = file.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let cached = file.as_deref().and_then(load);
        let fresh = cached.as_ref().is_some_and(|i| now_ms().saturating_sub(i.built_at) < MAX_AGE.as_millis() as u64);
        let search = app.state::<Search>();
        if let Some(index) = cached {
            *search.index.write().unwrap() = Some(index);
            let _ = app.emit("index-status", status(&search));
        }
        start_watching(&app);
        if !fresh {
            rebuild(&app);
        }
        let Some(file) = file else { return };
        loop {
            std::thread::sleep(SAVE_EVERY);
            if search.dirty.swap(false, Ordering::Relaxed) {
                if let Some(index) = search.index.read().unwrap().as_ref() {
                    let _ = save(index, &file, now_ms());
                }
            }
        }
    });
}

#[tauri::command]
pub fn search_status(search: State<'_, Search>) -> Status {
    status(&search)
}

#[tauri::command]
pub fn rebuild_index(app: AppHandle) {
    rebuild(&app);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Results {
    /// All matches – `hits` holds only the first `limit`.
    total: u64,
    hits: Vec<Hit>,
    elapsed_ms: u64,
    /// The folder is being read again; `folder-read` follows when it is done.
    refreshing: bool,
}

/// Path, name and kind of the first `limit` hits.
fn first_hits(index: &Index, ids: &[u32], limit: usize) -> Vec<(PathBuf, String, bool)> {
    ids.iter()
        .take(limit)
        .map(|&id| (index.path(id as usize), index.name(id as usize).to_owned(), index.is_dir[id as usize]))
        .collect()
}

/// Searches everywhere, or with `within` only below that folder – in the drive index, which is kept
/// current; folders it does not cover are read for the search (see `folder_search`).
#[tauri::command]
pub async fn search(
    app: AppHandle,
    query: String,
    limit: usize,
    within: Option<String>,
    fresh: bool,
) -> Result<Results, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let search = app.state::<Search>();
        let started = Instant::now();
        let from_drives = {
            let guard = search.index.read().unwrap();
            guard.as_ref().and_then(|index| {
                let folder = match &within {
                    None => None,
                    Some(dir) => Some(index.resolve(dir)?),
                };
                let ids = find_in(index, &query, folder);
                Some((first_hits(index, &ids, limit), ids.len()))
            })
        };
        let (found, total, refreshing) = match (from_drives, within) {
            (Some((found, total)), _) => (found, total, false),
            (None, None) => (Vec::new(), 0, false),
            (None, Some(dir)) => {
                let read_app = app.clone();
                let (index, folder, refreshing) = folder_search(&search.folders, &dir, fresh, move || {
                    let _ = read_app.emit("folder-read", ());
                });
                let ids = find_in(&index, &query, Some(folder));
                (first_hits(&index, &ids, limit), ids.len(), refreshing)
            }
        };
        let shown = found.len();
        let hits: Vec<Hit> = found
            .into_par_iter()
            .filter_map(|(path, name, is_dir)| {
                // Size and date are read now (the index has names only); deleted entries are left out.
                let meta = fs::metadata(&path).or_else(|_| fs::symlink_metadata(&path)).ok()?;
                Some(Hit {
                    name,
                    is_dir,
                    size: if meta.is_file() { meta.len() } else { 0 },
                    modified: meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_millis() as u64),
                    path: path.to_string_lossy().into_owned(),
                })
            })
            .collect();
        let gone = (shown - hits.len()) as u64;
        Results { total: total as u64 - gone, hits, elapsed_ms: started.elapsed().as_millis() as u64, refreshing }
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Index {
        let mut index = Index::default();
        let root = index.push("C:\\", NONE, true);
        let projects = index.push("Projekte", root, true);
        let app = index.push("mrfilesys", projects, true);
        index.push("README.md", app, false);
        index.push("Größe.txt", app, false);
        index.push("readme", projects, false);
        index.push("Bericht 2026.pdf", root, false);
        index
    }

    fn names(index: &Index, query: &str) -> Vec<String> {
        find(index, query).iter().map(|&id| index.name(id as usize).to_owned()).collect()
    }

    fn paths(index: &Index, query: &str) -> Vec<PathBuf> {
        find(index, query).iter().map(|&id| index.path(id as usize)).collect()
    }

    #[test]
    fn builds_paths_from_parents() {
        let index = sample();
        assert_eq!(index.path(3), PathBuf::from("C:\\Projekte\\mrfilesys\\README.md"));
        assert_eq!(index.resolve("c:\\projekte\\MRFILESYS\\readme.md"), Some(3));
        assert_eq!(index.resolve("C:\\"), Some(0));
        assert_eq!(index.resolve("C:\\Projekte\\fehlt"), None);
        assert_eq!(index.resolve("D:\\Projekte"), None);
    }

    #[test]
    fn finds_parts_of_names_case_insensitively() {
        let index = sample();
        assert_eq!(names(&index, "readme"), ["readme", "README.md"]);
        assert_eq!(names(&index, "GRÖSSE"), Vec::<String>::new());
        assert_eq!(names(&index, "größe"), ["Größe.txt"]);
        assert_eq!(names(&index, "bericht 2026"), ["Bericht 2026.pdf"]);
        assert_eq!(names(&index, "  "), Vec::<String>::new());
    }

    #[test]
    fn supports_wildcards_and_paths() {
        let index = sample();
        assert_eq!(names(&index, "*.pdf"), ["Bericht 2026.pdf"]);
        assert_eq!(names(&index, "r??dme*"), ["readme", "README.md"]);
        assert_eq!(names(&index, "mrfilesys\\ read"), ["README.md"]);
    }

    #[test]
    fn matches_never_span_two_names() {
        let index = sample();
        // "...sysREADME" would only exist across two neighbouring names.
        assert!(names(&index, "sysreadme").is_empty());
    }

    #[test]
    fn searches_below_a_folder() {
        let index = sample();
        let names = |dir: &str, query: &str| -> Vec<String> {
            let folder = index.resolve(dir).unwrap();
            find_in(&index, query, Some(folder)).iter().map(|&id| index.name(id as usize).to_owned()).collect()
        };
        assert_eq!(names("C:\\Projekte", "readme"), ["readme", "README.md"]);
        assert_eq!(names("C:\\Projekte\\mrfilesys", "readme"), ["README.md"]);
        assert_eq!(names("C:\\Projekte\\mrfilesys", "*e*"), ["README.md", "Größe.txt"]);
        // Neither the folder itself nor anything next to it.
        assert!(names("C:\\Projekte", "projekte").is_empty());
        assert!(names("C:\\Projekte", "bericht").is_empty());
    }

    #[test]
    fn follows_renames_moves_and_removals() {
        let mut index = sample();
        let at = |p: &str| PathBuf::from(p);
        let leaf = |name: &str| Node { name: name.into(), is_dir: false, children: Vec::new() };

        // Added: a file, and a folder that was moved here with its content.
        apply(&mut index, Op::Add(at("C:\\Projekte\\neu.txt"), leaf("neu.txt")));
        let moved = Node { name: "Archiv".into(), is_dir: true, children: vec![leaf("alt.txt")] };
        apply(&mut index, Op::Add(at("C:\\Archiv"), moved));
        // Twice is harmless.
        apply(&mut index, Op::Add(at("C:\\Projekte\\neu.txt"), leaf("neu.txt")));
        assert_eq!(paths(&index, "neu"), [at("C:\\Projekte\\neu.txt")]);
        assert_eq!(paths(&index, "alt.txt"), [at("C:\\Archiv\\alt.txt")]);

        // Renaming a folder keeps what is inside; the old name is no longer found.
        apply(&mut index, Op::Rename(at("C:\\Projekte\\mrfilesys"), at("C:\\Projekte\\dateien")));
        assert_eq!(paths(&index, "größe"), [at("C:\\Projekte\\dateien\\Größe.txt")]);
        assert!(names(&index, "mrfilesys").is_empty());
        // Moved to another folder.
        apply(&mut index, Op::Rename(at("C:\\Projekte\\neu.txt"), at("C:\\Archiv\\neu.txt")));
        assert_eq!(paths(&index, "neu"), [at("C:\\Archiv\\neu.txt")]);
        // Into the recycle bin: gone.
        apply(&mut index, Op::Rename(at("C:\\Archiv\\neu.txt"), at("C:\\$Recycle.Bin\\S-1\\$R1.txt")));
        assert!(names(&index, "neu").is_empty());
        // Unknown under its old name: to be added under the new one.
        assert_eq!(apply(&mut index, Op::Rename(at("C:\\x"), at("C:\\y"))), Some(at("C:\\y")));

        // Removing a folder removes everything in it.
        apply(&mut index, Op::Remove(at("C:\\Projekte")));
        assert!(names(&index, "readme").is_empty());
        assert!(names(&index, "größe").is_empty());
        assert_eq!(names(&index, "bericht"), ["Bericht 2026.pdf"]);

        // Compacted, the same is found.
        let compact = index.compacted();
        assert_eq!(compact.len(), index.live());
        assert_eq!(paths(&compact, "alt"), [at("C:\\Archiv\\alt.txt")]);
        assert!(names(&compact, "readme").is_empty());
    }

    #[test]
    fn wildcard_matching() {
        let p = |s: &str| s.chars().collect::<Vec<_>>();
        assert!(wildcard(&p("*.txt"), "a.txt"));
        assert!(!wildcard(&p("*.txt"), "a.txt.bak"));
        assert!(wildcard(&p("a*b*c"), "aXXbYYc"));
        assert!(wildcard(&p("?"), "x"));
        assert!(!wildcard(&p("?"), ""));
    }

    #[test]
    fn cache_round_trip() {
        let mut index = sample();
        apply(&mut index, Op::Remove(PathBuf::from("C:\\Bericht 2026.pdf")));
        let file = std::env::temp_dir().join(format!("mrfilesys-index-{}.bin", std::process::id()));
        save(&index, &file, 42).unwrap();
        let loaded = load(&file).unwrap();
        assert_eq!(loaded.built_at, 42);
        assert_eq!(loaded.len(), index.live());
        assert_eq!(paths(&loaded, "größe"), paths(&index, "größe"));
        assert_eq!(names(&loaded, "readme"), ["readme", "README.md"]);
        assert!(names(&loaded, "bericht").is_empty());
        // A damaged file is rejected instead of crashing later.
        let mut data = fs::read(&file).unwrap();
        data.truncate(data.len() - 3);
        fs::write(&file, data).unwrap();
        assert!(load(&file).is_none());
        fs::remove_file(file).unwrap();
    }

    #[test]
    fn scans_a_tree() {
        let dir = std::env::temp_dir().join(format!("mrfilesys-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("a").join("b")).unwrap();
        fs::write(dir.join("a").join("b").join("needle.txt"), "").unwrap();
        fs::create_dir_all(dir.join("$Recycle.Bin")).unwrap();
        fs::write(dir.join("$Recycle.Bin").join("needle-deleted.txt"), "").unwrap();
        let root = dir.to_string_lossy().into_owned();
        let index = build(&[root], &AtomicU64::new(0));
        let hits = find(&index, "needle");
        assert_eq!(hits.len(), 1);
        assert_eq!(index.path(hits[0] as usize), dir.join("a").join("b").join("needle.txt"));
        fs::remove_dir_all(dir).unwrap();
    }

    /// The whole chain: Windows reports changes, they reach the index.
    #[cfg(windows)]
    #[test]
    fn keeps_the_index_current() {
        let dir = std::env::temp_dir().join(format!("mrfilesys-live-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("alt")).unwrap();
        let root = dir.to_string_lossy().into_owned();
        let search = Arc::new(Search::default());
        *search.index.write().unwrap() = Some(build(std::slice::from_ref(&root), &AtomicU64::new(0)));
        let watcher = search.clone();
        assert!(watch::watch(root, move |c| apply_changes(&watcher, c), || {}));
        std::thread::sleep(Duration::from_millis(200));

        fs::write(dir.join("alt").join("nadel.txt"), "").unwrap();
        fs::create_dir_all(dir.join("draußen").join("tief")).unwrap();
        fs::write(dir.join("draußen").join("tief").join("heu.txt"), "").unwrap();
        fs::rename(dir.join("alt"), dir.join("neu")).unwrap();

        let found = |query: &str| {
            let guard = search.index.read().unwrap();
            let index = guard.as_ref().unwrap();
            find(index, query).iter().map(|&id| index.path(id as usize)).collect::<Vec<_>>()
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while found("nadel") != [dir.join("neu").join("nadel.txt")] && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(found("nadel"), [dir.join("neu").join("nadel.txt")]);
        assert_eq!(found("heu.txt"), [dir.join("draußen").join("tief").join("heu.txt")]);
        assert!(search.dirty.load(Ordering::Relaxed));
    }

    #[test]
    fn folders_cover_their_subfolders() {
        assert!(covers("c:\\projekte", "c:\\projekte"));
        assert!(covers("c:\\projekte", "c:\\projekte\\app"));
        assert!(!covers("c:\\projekte", "c:\\projekte2"));
        assert!(covers("c:\\", "c:\\projekte"));
        assert!(!covers("c:\\", "d:\\"));
    }

    #[test]
    fn caches_folder_trees() {
        let dir = std::env::temp_dir().join(format!("mrfilesys-folders-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub").join("alt.txt"), "").unwrap();
        let root = dir.to_string_lossy().into_owned();
        let sub = dir.join("sub").to_string_lossy().into_owned();
        let (tx, rx) = std::sync::mpsc::channel();
        let search = |folders: &Arc<Folders>, dir: &str, fresh: bool| {
            let tx = tx.clone();
            let (index, folder, refreshing) = folder_search(folders, dir, fresh, move || tx.send(()).unwrap());
            let names: Vec<String> =
                find_in(&index, "txt", Some(folder)).iter().map(|&id| index.name(id as usize).to_owned()).collect();
            (names, refreshing)
        };
        let wait = || rx.recv_timeout(Duration::from_secs(10)).unwrap();

        // Not read yet: read right away.
        let folders = Arc::new(Folders::default());
        assert_eq!(search(&folders, &root, true), (vec!["alt.txt".into()], false));
        fs::write(dir.join("sub").join("neu.txt"), "").unwrap();
        // While the search goes on the tree read at its start is used – also for a subfolder.
        assert_eq!(search(&folders, &root, false), (vec!["alt.txt".into()], false));
        assert_eq!(search(&folders, &sub, false), (vec!["alt.txt".into()], false));
        // A new search answers from that tree at once and reads the folder again in the background.
        assert_eq!(search(&folders, &sub, true), (vec!["alt.txt".into()], true));
        wait();
        let (mut fresh, refreshing) = search(&folders, &sub, false);
        fresh.sort();
        assert_eq!((fresh, refreshing), (vec!["alt.txt".into(), "neu.txt".into()], false));
        // A folder created after the tree was read is read on its own.
        fs::create_dir_all(dir.join("sub").join("später")).unwrap();
        fs::write(dir.join("sub").join("später").join("x.txt"), "").unwrap();
        let later = dir.join("sub").join("später").to_string_lossy().into_owned();
        assert_eq!(search(&folders, &later, false), (vec!["x.txt".into()], false));
        fs::remove_dir_all(dir).unwrap();
    }

    /// `cargo test -p mrfilesys --release --lib search::tests::whole_disk -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn whole_disk() {
        let started = Instant::now();
        let index = build(&crate::drives::fixed_roots(), &AtomicU64::new(0));
        let bytes = index.names.len() + index.lower.len() + index.len() * (4 * 8 + 2);
        println!("Index: {} Einträge in {} ms, ~{} MB", index.len(), started.elapsed().as_millis(), bytes >> 20);
        for query in ["notepad", "*.pdf", "readme", "a"] {
            let t = Instant::now();
            let ids = find(&index, query);
            println!("  „{query}“: {} Treffer in {} ms", ids.len(), t.elapsed().as_millis());
        }
        let users = index.resolve("C:\\Users").unwrap();
        let t = Instant::now();
        let ids = find_in(&index, "readme", Some(users));
        println!("  „readme“ in C:\\Users: {} Treffer in {} ms", ids.len(), t.elapsed().as_millis());
        let t = Instant::now();
        let compact = index.compacted();
        println!("  kompaktiert in {} ms ({} Einträge)", t.elapsed().as_millis(), compact.len());
    }

    /// `MRFILESYS_SCAN=C:\Users cargo test -p mrfilesys --release --lib search::tests::scan_folder -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn scan_folder() {
        let dir = std::env::var("MRFILESYS_SCAN").unwrap_or_else(|_| "C:\\Users".into());
        for _ in 0..2 {
            let started = Instant::now();
            let index = build(std::slice::from_ref(&dir), &AtomicU64::new(0));
            println!("{dir}: {} Einträge in {} ms", index.len(), started.elapsed().as_millis());
        }
    }
}
