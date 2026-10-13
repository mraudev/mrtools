//! Data for the properties dialog: sizes (also on disk), times, the program that opens a file,
//! link targets, attributes and checksums.

use md5::Md5;
use rayon::prelude::*;
use serde::Serialize;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::UNIX_EPOCH,
};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[derive(Default)]
struct Totals {
    size: AtomicU64,
    on_disk: AtomicU64,
    files: AtomicU64,
    dirs: AtomicU64,
    errors: AtomicU64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measure {
    size: u64,
    /// Rounded up to whole clusters, like "Größe auf Datenträger" in Explorer (approximately:
    /// compressed, sparse and tiny files stored in the MFT need less).
    on_disk: u64,
    files: u64,
    dirs: u64,
    /// Entries that could not be read.
    errors: u64,
}

fn round_up(size: u64, cluster: u64) -> u64 {
    if cluster == 0 {
        size
    } else {
        size.div_ceil(cluster) * cluster
    }
}

/// Adds `path` and – for folders – everything below it. Links are counted, not followed.
fn walk(path: &Path, cluster: u64, totals: &Totals, top: bool) {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(_) => {
            totals.errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
    };
    if meta.is_dir() && !meta.file_type().is_symlink() {
        if !top {
            totals.dirs.fetch_add(1, Ordering::Relaxed);
        }
        match fs::read_dir(path) {
            Ok(entries) => {
                let children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
                children.par_iter().for_each(|child| walk(child, cluster, totals, false));
            }
            Err(_) => {
                totals.errors.fetch_add(1, Ordering::Relaxed);
            }
        }
    } else if !meta.file_type().is_symlink() {
        totals.files.fetch_add(1, Ordering::Relaxed);
        totals.size.fetch_add(meta.len(), Ordering::Relaxed);
        totals.on_disk.fetch_add(round_up(meta.len(), cluster), Ordering::Relaxed);
    }
}

/// Total size of files and folders together with everything below them (in parallel).
#[tauri::command]
pub async fn measure(paths: Vec<String>) -> Result<Measure, String> {
    blocking(move || {
        let totals = Totals::default();
        paths.par_iter().for_each(|p| {
            let path = Path::new(p);
            walk(path, imp::cluster_size(path), &totals, true);
        });
        let get = |a: &AtomicU64| a.load(Ordering::Relaxed);
        Ok(Measure {
            size: get(&totals.size),
            on_disk: get(&totals.on_disk),
            files: get(&totals.files),
            dirs: get(&totals.dirs),
            errors: get(&totals.errors),
        })
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemInfo {
    /// ms since 1970, 0 if unknown.
    accessed: u64,
    /// Files only; `None` for folders (see `measure`).
    on_disk: Option<u64>,
    /// Program that opens files of this type, e.g. "Visual Studio Code".
    open_with: Option<String>,
    /// Target of a symbolic link or junction.
    link_target: Option<String>,
}

/// What the properties dialog shows beyond the entry itself.
#[tauri::command]
pub async fn item_info(path: String) -> Result<ItemInfo, String> {
    blocking(move || {
        let path = PathBuf::from(path);
        let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        let link = meta.file_type().is_symlink();
        let is_dir = if link { fs::metadata(&path).is_ok_and(|m| m.is_dir()) } else { meta.is_dir() };
        let accessed = meta
            .accessed()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as u64);
        Ok(ItemInfo {
            accessed,
            on_disk: (!is_dir).then(|| imp::size_on_disk(&path, meta.len())),
            open_with: if is_dir { None } else { imp::open_with(&path) },
            link_target: link.then(|| fs::read_link(&path).ok()).flatten().map(|t| t.to_string_lossy().into_owned()),
        })
    })
    .await
}

/// Sets or clears "read-only" and "hidden" on all `paths` (`None` leaves it as it is).
#[tauri::command]
pub async fn set_attributes(paths: Vec<String>, readonly: Option<bool>, hidden: Option<bool>) -> Result<(), String> {
    blocking(move || {
        let failed: Vec<String> = paths
            .iter()
            .filter_map(|p| imp::set_attributes(Path::new(p), readonly, hidden).err().map(|e| format!("{p}: {e}")))
            .collect();
        if failed.is_empty() {
            Ok(())
        } else {
            Err(failed.join("\n"))
        }
    })
    .await
}

#[derive(Serialize)]
pub struct Hashes {
    sha256: String,
    sha1: String,
    md5: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_file(path: &Path) -> std::io::Result<Hashes> {
    let mut file = fs::File::open(path)?;
    let (mut sha256, mut sha1, mut md5) = (Sha256::new(), Sha1::new(), Md5::new());
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        sha256.update(&buf[..n]);
        sha1.update(&buf[..n]);
        md5.update(&buf[..n]);
    }
    Ok(Hashes { sha256: hex(&sha256.finalize()), sha1: hex(&sha1.finalize()), md5: hex(&md5.finalize()) })
}

/// SHA-256, SHA-1 and MD5 of a file, read once.
#[tauri::command]
pub async fn hashes(path: String) -> Result<Hashes, String> {
    blocking(move || hash_file(Path::new(&path)).map_err(|e| e.to_string())).await
}

#[cfg(windows)]
mod imp {
    use std::path::Path;
    use windows_sys::Win32::{
        Storage::FileSystem::{
            GetCompressedFileSizeW, GetDiskFreeSpaceW, GetFileAttributesW, GetVolumePathNameW, SetFileAttributesW,
            FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY, INVALID_FILE_ATTRIBUTES,
        },
        UI::Shell::{AssocQueryStringW, ASSOCF_NONE, ASSOCSTR_FRIENDLYAPPNAME},
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    fn from_wide(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    /// Bytes per cluster of the volume `path` is on (0 if unknown).
    pub fn cluster_size(path: &Path) -> u64 {
        let mut root = [0u16; 1024];
        let path = wide(&path.to_string_lossy());
        unsafe {
            if GetVolumePathNameW(path.as_ptr(), root.as_mut_ptr(), root.len() as u32) == 0 {
                return 0;
            }
            let (mut sectors, mut bytes, mut free, mut total) = (0u32, 0u32, 0u32, 0u32);
            if GetDiskFreeSpaceW(root.as_ptr(), &mut sectors, &mut bytes, &mut free, &mut total) == 0 {
                return 0;
            }
            u64::from(sectors) * u64::from(bytes)
        }
    }

    /// Allocated size of a file: compressed size rounded to clusters.
    pub fn size_on_disk(path: &Path, len: u64) -> u64 {
        let wide_path = wide(&path.to_string_lossy());
        let mut high = 0u32;
        let low = unsafe { GetCompressedFileSizeW(wide_path.as_ptr(), &mut high) };
        let size = if low == u32::MAX && std::io::Error::last_os_error().raw_os_error() != Some(0) {
            len
        } else {
            (u64::from(high) << 32) | u64::from(low)
        };
        super::round_up(size, cluster_size(path))
    }

    pub fn open_with(path: &Path) -> Option<String> {
        let ext = path.extension()?.to_string_lossy();
        let ext = wide(&format!(".{ext}"));
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let hr = unsafe {
            AssocQueryStringW(ASSOCF_NONE as _, ASSOCSTR_FRIENDLYAPPNAME, ext.as_ptr(), std::ptr::null(), buf.as_mut_ptr(), &mut len)
        };
        let name = from_wide(&buf);
        (hr == 0 && !name.is_empty()).then_some(name)
    }

    pub fn set_attributes(path: &Path, readonly: Option<bool>, hidden: Option<bool>) -> std::io::Result<()> {
        let wide_path = wide(&path.to_string_lossy());
        let mut attrs = unsafe { GetFileAttributesW(wide_path.as_ptr()) };
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(std::io::Error::last_os_error());
        }
        let mut set = |flag: u32, on: Option<bool>| match on {
            Some(true) => attrs |= flag,
            Some(false) => attrs &= !flag,
            None => {}
        };
        set(FILE_ATTRIBUTE_READONLY, readonly);
        set(FILE_ATTRIBUTE_HIDDEN, hidden);
        // Without any other attribute, Windows wants NORMAL instead of 0.
        let attrs = if attrs == 0 { FILE_ATTRIBUTE_NORMAL } else { attrs & !FILE_ATTRIBUTE_NORMAL };
        if unsafe { SetFileAttributesW(wide_path.as_ptr(), attrs) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    pub fn cluster_size(_path: &Path) -> u64 {
        4096
    }

    pub fn size_on_disk(_path: &Path, len: u64) -> u64 {
        super::round_up(len, 4096)
    }

    pub fn open_with(_path: &Path) -> Option<String> {
        None
    }

    pub fn set_attributes(_path: &Path, _readonly: Option<bool>, _hidden: Option<bool>) -> std::io::Result<()> {
        Err(std::io::Error::other("Nur unter Windows verfügbar"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mrfilesys-props-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn measures_trees_and_rounds_to_clusters() {
        let dir = temp_dir("measure");
        fs::write(dir.join("a"), "12345").unwrap();
        fs::create_dir(dir.join("sub")).unwrap();
        fs::write(dir.join("sub").join("b"), "123").unwrap();
        let totals = Totals::default();
        walk(&dir, 4096, &totals, true);
        let get = |a: &AtomicU64| a.load(Ordering::Relaxed);
        assert_eq!(
            (get(&totals.size), get(&totals.on_disk), get(&totals.files), get(&totals.dirs), get(&totals.errors)),
            (8, 8192, 2, 1, 0)
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rounding() {
        assert_eq!(round_up(0, 4096), 0);
        assert_eq!(round_up(1, 4096), 4096);
        assert_eq!(round_up(4096, 4096), 4096);
        assert_eq!(round_up(5, 0), 5);
    }

    #[test]
    fn hashes_known_values() {
        let dir = temp_dir("hash");
        fs::write(dir.join("abc"), "abc").unwrap();
        let h = hash_file(&dir.join("abc")).unwrap();
        assert_eq!(h.sha256, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(h.sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(h.md5, "900150983cd24fb0d6963f7d28e17f72");
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn attributes_can_be_set_and_cleared() {
        let dir = temp_dir("attrs");
        let file = dir.join("f.txt");
        fs::write(&file, "x").unwrap();
        imp::set_attributes(&file, Some(true), Some(true)).unwrap();
        assert!(fs::metadata(&file).unwrap().permissions().readonly());
        imp::set_attributes(&file, Some(false), Some(false)).unwrap();
        assert!(!fs::metadata(&file).unwrap().permissions().readonly());
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn knows_the_program_for_text_files() {
        assert!(imp::open_with(Path::new("C:\\x.txt")).is_some());
        assert_eq!(imp::open_with(Path::new("C:\\no-extension")), None);
    }
}
