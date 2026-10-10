//! Search over all file names on the local drives. Reading the NTFS master file table directly (like
//! "Everything") needs administrator rights, so the app keeps its own index of names instead: built in
//! parallel in the background, cached on disk and renewed when it is older than `MAX_AGE`.

use memchr::memmem;
use rayon::prelude::*;
use serde::Serialize;
use std::{
    fs,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};

/// An index older than this is rebuilt in the background when the app starts.
const MAX_AGE: Duration = Duration::from_secs(30 * 60);
const NO_PARENT: u32 = u32::MAX;
const MAGIC: &[u8; 8] = b"MRFSIDX1";

/// Folders that are left out: deleted files with cryptic names, and nothing readable anyway.
fn skipped(name: &str) -> bool {
    name.eq_ignore_ascii_case("$Recycle.Bin") || name.eq_ignore_ascii_case("System Volume Information")
}

/// All names in a few flat arrays – a few dozen bytes per entry instead of a full path.
#[derive(Default)]
pub struct Index {
    names: String,
    /// Start of each name in `names`, plus the end.
    name_off: Vec<u32>,
    /// Lower-case names (separate offsets: lower-casing can change the length).
    lower: String,
    lower_off: Vec<u32>,
    parent: Vec<u32>,
    is_dir: Vec<bool>,
    /// ms since 1970.
    built_at: u64,
}

impl Index {
    fn len(&self) -> usize {
        self.parent.len()
    }

    fn name(&self, id: usize) -> &str {
        &self.names[self.name_off[id] as usize..self.name_off[id + 1] as usize]
    }

    fn lower_name(&self, id: usize) -> &str {
        &self.lower[self.lower_off[id] as usize..self.lower_off[id + 1] as usize]
    }

    fn push(&mut self, name: &str, parent: u32, is_dir: bool) -> u32 {
        let id = self.parent.len() as u32;
        self.names.push_str(name);
        self.name_off.push(self.names.len() as u32);
        self.lower.push_str(&name.to_lowercase());
        self.lower_off.push(self.lower.len() as u32);
        self.parent.push(parent);
        self.is_dir.push(is_dir);
        id
    }

    fn path(&self, id: usize) -> PathBuf {
        let mut parts = vec![self.name(id)];
        let mut p = self.parent[id];
        while p != NO_PARENT {
            parts.push(self.name(p as usize));
            p = self.parent[p as usize];
        }
        parts.iter().rev().collect()
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
    index.name_off.push(0);
    index.lower_off.push(0);
    for tree in trees {
        flatten(&mut index, tree, NO_PARENT);
    }
    index.built_at = now_ms();
    index
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

// ---------------------------------------------------------------------------
// Cache file

fn save(index: &Index, file: &Path) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("tmp");
    let mut out = BufWriter::new(fs::File::create(&tmp)?);
    out.write_all(MAGIC)?;
    out.write_all(&index.built_at.to_le_bytes())?;
    for len in [index.len(), index.names.len(), index.lower.len()] {
        out.write_all(&(len as u64).to_le_bytes())?;
    }
    out.write_all(index.names.as_bytes())?;
    out.write_all(index.lower.as_bytes())?;
    for list in [&index.name_off, &index.lower_off, &index.parent] {
        for v in list.iter() {
            out.write_all(&v.to_le_bytes())?;
        }
    }
    out.write_all(&index.is_dir.iter().map(|&d| d as u8).collect::<Vec<_>>())?;
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
    let mut u64_at = || Some(u64::from_le_bytes(take(8)?.try_into().ok()?));
    let built_at = u64_at()?;
    let (n, names_len, lower_len) = (u64_at()? as usize, u64_at()? as usize, u64_at()? as usize);
    let names = String::from_utf8(take(names_len)?.to_vec()).ok()?;
    let lower = String::from_utf8(take(lower_len)?.to_vec()).ok()?;
    let mut u32s = |count: usize| -> Option<Vec<u32>> {
        Some(take(count * 4)?.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect())
    };
    let name_off = u32s(n + 1)?;
    let lower_off = u32s(n + 1)?;
    let parent = u32s(n)?;
    let is_dir = take(n)?.iter().map(|&b| b != 0).collect();
    let index = Index { names, name_off, lower, lower_off, parent, is_dir, built_at };
    // A damaged file must not make slicing panic later.
    let valid = index.name_off.last() == Some(&(index.names.len() as u32))
        && index.lower_off.last() == Some(&(index.lower.len() as u32))
        && index.name_off.windows(2).all(|w| w[0] <= w[1] && index.names.is_char_boundary(w[1] as usize))
        && index.lower_off.windows(2).all(|w| w[0] <= w[1] && index.lower.is_char_boundary(w[1] as usize))
        && index.parent.iter().enumerate().all(|(i, &p)| p == NO_PARENT || (p as usize) < i);
    valid.then_some(index)
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

/// Ids of all entries matching `query`, best first: exact name, name starting with the first term,
/// shorter names.
fn find(index: &Index, query: &str) -> Vec<u32> {
    let terms = parse(query);
    if terms.is_empty() {
        return Vec::new();
    }
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
                let id = index.lower_off.partition_point(|&o| o as usize <= pos) - 1;
                // Hits across two names do not count.
                if id != last && pos + anchor.len() <= index.lower_off[id + 1] as usize && matches(index, id, &terms) {
                    ids.push(id as u32);
                    last = id;
                }
            }
            ids
        }
        None => (0..index.len()).into_par_iter().filter(|&id| matches(index, id, &terms)).map(|id| id as u32).collect(),
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

// ---------------------------------------------------------------------------
// Commands

#[derive(Default)]
pub struct Search {
    index: RwLock<Option<Arc<Index>>>,
    building: AtomicBool,
    scanned: AtomicU64,
    cache_file: Mutex<Option<PathBuf>>,
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
}

fn status(search: &Search) -> Status {
    let index = search.index.read().unwrap();
    Status {
        entries: index.as_ref().map_or(0, |i| i.len() as u64),
        built_at: index.as_ref().map_or(0, |i| i.built_at),
        building: search.building.load(Ordering::Relaxed),
        scanned: search.scanned.load(Ordering::Relaxed),
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

        let index = build(&crate::drives::fixed_roots(), &search.scanned);
        if let Some(file) = search.cache_file.lock().unwrap().clone() {
            let _ = save(&index, &file);
        }
        *search.index.write().unwrap() = Some(Arc::new(index));
        done.store(true, Ordering::Relaxed);
        search.building.store(false, Ordering::Relaxed);
        let _ = app.emit("index-status", status(&search));
    });
}

/// Loads the cached index and renews it if it is missing or old. Called once at startup.
pub fn init(app: &AppHandle) {
    let file = app.path().app_cache_dir().ok().map(|dir| dir.join("search-index.bin"));
    *app.state::<Search>().cache_file.lock().unwrap() = file.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let cached = file.as_deref().and_then(load);
        let fresh = cached.as_ref().is_some_and(|i| now_ms().saturating_sub(i.built_at) < MAX_AGE.as_millis() as u64);
        if let Some(index) = cached {
            let search = app.state::<Search>();
            *search.index.write().unwrap() = Some(Arc::new(index));
            let _ = app.emit("index-status", status(&search));
        }
        if !fresh {
            rebuild(&app);
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
}

#[tauri::command]
pub async fn search(search: State<'_, Search>, query: String, limit: usize) -> Result<Results, String> {
    let Some(index) = search.index.read().unwrap().clone() else {
        return Ok(Results { total: 0, hits: Vec::new(), elapsed_ms: 0 });
    };
    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        let ids = find(&index, &query);
        let hits = ids
            .par_iter()
            .take(limit)
            .map(|&id| {
                let path = index.path(id as usize);
                // Size and date are read now (the index has names only); deleted files stay in the list.
                let meta = fs::metadata(&path).ok();
                Hit {
                    name: index.name(id as usize).to_owned(),
                    is_dir: index.is_dir[id as usize],
                    size: meta.as_ref().filter(|m| m.is_file()).map_or(0, |m| m.len()),
                    modified: meta
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_millis() as u64),
                    path: path.to_string_lossy().into_owned(),
                }
            })
            .collect();
        Results { total: ids.len() as u64, hits, elapsed_ms: started.elapsed().as_millis() as u64 }
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Index {
        let mut index = Index::default();
        index.name_off.push(0);
        index.lower_off.push(0);
        let root = index.push("C:\\", NO_PARENT, true);
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

    #[test]
    fn builds_paths_from_parents() {
        let index = sample();
        assert_eq!(index.path(3), PathBuf::from("C:\\Projekte\\mrfilesys\\README.md"));
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
        let index = sample();
        let file = std::env::temp_dir().join(format!("mrfilesys-index-{}.bin", std::process::id()));
        save(&index, &file).unwrap();
        let loaded = load(&file).unwrap();
        assert_eq!(loaded.len(), index.len());
        assert_eq!(loaded.path(4), index.path(4));
        assert_eq!(names(&loaded, "readme"), ["readme", "README.md"]);
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

    /// `cargo test -p mrfilesys --release --lib search::tests::whole_disk -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn whole_disk() {
        let started = Instant::now();
        let index = build(&crate::drives::fixed_roots(), &AtomicU64::new(0));
        let bytes = index.names.len() + index.lower.len() + index.len() * (4 * 3 + 1);
        println!("Index: {} Einträge in {} ms, ~{} MB", index.len(), started.elapsed().as_millis(), bytes >> 20);
        for query in ["notepad", "*.pdf", "readme", "a"] {
            let t = Instant::now();
            let ids = find(&index, query);
            println!("  „{query}“: {} Treffer in {} ms", ids.len(), t.elapsed().as_millis());
        }
    }
}
