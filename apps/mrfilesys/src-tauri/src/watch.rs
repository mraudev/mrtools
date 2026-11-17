//! Changes to file and folder names on a drive, as Windows reports them (`ReadDirectoryChangesW`, no
//! administrator rights needed) – keeps the search index current while the app runs.

use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum Change {
    Added(PathBuf),
    Removed(PathBuf),
    /// Renamed or moved (within the drive).
    Renamed(PathBuf, PathBuf),
}

/// Reads the `FILE_NOTIFY_INFORMATION` records Windows filled `buffer` with.
fn parse(root: &Path, buffer: &[u8]) -> Vec<Change> {
    const ADDED: u32 = 1;
    const REMOVED: u32 = 2;
    const RENAMED_OLD: u32 = 4;
    const RENAMED_NEW: u32 = 5;

    let u32_at = |pos: usize| buffer.get(pos..pos + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let mut changes = Vec::new();
    let mut old: Option<PathBuf> = None;
    let mut pos = 0;
    while let (Some(next), Some(action), Some(len)) = (u32_at(pos), u32_at(pos + 4), u32_at(pos + 8)) {
        let Some(bytes) = buffer.get(pos + 12..pos + 12 + len as usize) else { break };
        let name: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
        let path = root.join(String::from_utf16_lossy(&name));
        match action {
            ADDED => changes.push(Change::Added(path)),
            REMOVED => changes.push(Change::Removed(path)),
            RENAMED_OLD => old = Some(path),
            RENAMED_NEW => changes.push(match old.take() {
                Some(from) => Change::Renamed(from, path),
                None => Change::Added(path),
            }),
            _ => {}
        }
        if next == 0 {
            break;
        }
        pos += next as usize;
    }
    // Renamed to somewhere outside – gone from here.
    changes.extend(old.map(Change::Removed));
    changes
}

/// Watches `root` and everything below it in a thread of its own; `changes` gets them in batches as
/// Windows reports them. `lost` is called when Windows dropped changes (too many at once) – the index
/// has to be read again then. Returns false if the drive cannot be watched.
#[cfg(windows)]
pub fn watch(root: String, changes: impl Fn(Vec<Change>) + Send + 'static, lost: impl Fn() + Send + 'static) -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY,
            FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, OPEN_EXISTING,
        },
    };

    let wide: Vec<u16> = root.encode_utf16().chain([0]).collect();
    // SAFETY: plain CreateFileW on a NUL-terminated path; the handle is checked.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return false;
    }
    let handle = handle as usize;
    std::thread::spawn(move || {
        let handle = handle as windows_sys::Win32::Foundation::HANDLE;
        // u32 for the alignment the records need.
        let mut buffer = vec![0u32; 256 * 1024];
        loop {
            let mut filled = 0u32;
            // SAFETY: synchronous call (no OVERLAPPED) into our own buffer of the given size.
            let ok = unsafe {
                ReadDirectoryChangesW(
                    handle,
                    buffer.as_mut_ptr().cast(),
                    (buffer.len() * 4) as u32,
                    1,
                    FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME,
                    &mut filled,
                    std::ptr::null_mut(),
                    None,
                )
            };
            if ok == 0 {
                break;
            }
            if filled == 0 {
                lost();
                continue;
            }
            // SAFETY: Windows filled `filled` bytes of the buffer.
            let bytes = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), filled as usize) };
            let found = parse(Path::new(&root), bytes);
            if !found.is_empty() {
                changes(found);
            }
        }
        // SAFETY: our own handle, closed once.
        unsafe { CloseHandle(handle) };
    });
    true
}

#[cfg(not(windows))]
pub fn watch(_root: String, _changes: impl Fn(Vec<Change>) + Send + 'static, _lost: impl Fn() + Send + 'static) -> bool {
    false
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{fs, sync::mpsc, time::Duration};

    #[test]
    fn reports_added_renamed_and_removed_entries() {
        let dir = std::env::temp_dir().join(format!("mrfilesys-watch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let (tx, rx) = mpsc::channel();
        assert!(watch(dir.to_string_lossy().into_owned(), move |c| c.into_iter().for_each(|c| tx.send(c).unwrap()), || {}));
        // Changes count from the first wait for them on.
        std::thread::sleep(Duration::from_millis(200));

        fs::create_dir(dir.join("ordner")).unwrap();
        fs::write(dir.join("ordner").join("a.txt"), "").unwrap();
        fs::rename(dir.join("ordner").join("a.txt"), dir.join("ordner").join("b.txt")).unwrap();
        fs::rename(dir.join("ordner").join("b.txt"), dir.join("c.txt")).unwrap();
        fs::remove_file(dir.join("c.txt")).unwrap();

        let mut seen = Vec::new();
        while let Ok(change) = rx.recv_timeout(Duration::from_secs(2)) {
            seen.push(change);
            if seen.len() == 6 {
                break;
            }
        }
        assert_eq!(
            seen,
            [
                Change::Added(dir.join("ordner")),
                Change::Added(dir.join("ordner").join("a.txt")),
                Change::Renamed(dir.join("ordner").join("a.txt"), dir.join("ordner").join("b.txt")),
                // Moved to another folder: gone here, new there.
                Change::Removed(dir.join("ordner").join("b.txt")),
                Change::Added(dir.join("c.txt")),
                Change::Removed(dir.join("c.txt")),
            ]
        );
        let _ = fs::remove_dir_all(dir);
    }
}
