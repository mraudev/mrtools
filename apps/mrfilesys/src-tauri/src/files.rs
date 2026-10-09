//! Reading folders and simple changes (rename, create) with `std::fs`.

use serde::Serialize;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    name: String,
    path: String,
    is_dir: bool,
    /// Bytes, 0 for folders.
    size: u64,
    /// ms since 1970, 0 if unknown.
    modified: u64,
    created: u64,
    hidden: bool,
    system: bool,
    readonly: bool,
    /// Symbolic link or junction.
    link: bool,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FolderSize {
    size: u64,
    files: u64,
    dirs: u64,
    /// Entries that could not be read.
    errors: u64,
}

const FILE_ATTRIBUTE_READONLY: u32 = 0x1;
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;

#[cfg(windows)]
fn attributes(meta: &fs::Metadata) -> u32 {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes()
}

#[cfg(not(windows))]
fn attributes(meta: &fs::Metadata) -> u32 {
    if meta.permissions().readonly() {
        FILE_ATTRIBUTE_READONLY
    } else {
        0
    }
}

fn millis(time: std::io::Result<SystemTime>) -> u64 {
    time.ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64)
}

/// Replaces `%NAME%` with the environment variable, like Explorer (`%appdata%`, `%temp%`, …).
/// Unknown variables stay as they are. Names are case-insensitive on Windows.
fn expand_env(path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('%') {
        let Some(len) = rest[start + 1..].find('%') else { break };
        let name = &rest[start + 1..start + 1 + len];
        out.push_str(&rest[..start]);
        match std::env::var(name) {
            Ok(value) if !name.is_empty() => {
                out.push_str(&value);
                rest = &rest[start + len + 2..];
            }
            // Not a variable: keep the first `%` – the second one may start a real variable.
            _ => {
                out.push('%');
                rest = &rest[start + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Expands environment variables. `C:` means "current directory on C:" to Windows – the user
/// means the drive root.
fn normalize(path: &str) -> PathBuf {
    let expanded = expand_env(path.trim());
    let trimmed = expanded.trim();
    if trimmed.len() == 2 && trimmed.ends_with(':') {
        PathBuf::from(format!("{trimmed}\\"))
    } else {
        PathBuf::from(trimmed)
    }
}

fn entry(path: PathBuf, meta: fs::Metadata) -> Entry {
    let link = meta.file_type().is_symlink();
    // Links show what they point to; broken ones stay plain entries.
    let meta = if link { fs::metadata(&path).unwrap_or(meta) } else { meta };
    let attrs = attributes(&meta);
    let name = path
        .file_name()
        .map_or_else(|| path.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned());
    Entry {
        hidden: attrs & FILE_ATTRIBUTE_HIDDEN != 0 || (cfg!(not(windows)) && name.starts_with('.')),
        system: attrs & FILE_ATTRIBUTE_SYSTEM != 0,
        readonly: attrs & FILE_ATTRIBUTE_READONLY != 0,
        is_dir: meta.is_dir(),
        size: if meta.is_dir() { 0 } else { meta.len() },
        modified: millis(meta.modified()),
        created: millis(meta.created()),
        link,
        name,
        path: path.to_string_lossy().into_owned(),
    }
}

fn read_dir(path: &Path) -> Result<Vec<Entry>, String> {
    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
    Ok(entries
        .flatten()
        .filter_map(|e| Some(entry(e.path(), e.metadata().ok()?)))
        .collect())
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

/// Contents of a folder (unsorted – the frontend sorts).
#[tauri::command]
pub async fn list_dir(path: String) -> Result<Vec<Entry>, String> {
    blocking(move || read_dir(&normalize(&path))).await
}

/// A single entry, e.g. to check a path typed into the address bar.
#[tauri::command]
pub async fn stat(path: String) -> Result<Entry, String> {
    blocking(move || {
        let path = normalize(&path);
        let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        Ok(entry(path, meta))
    })
    .await
}

fn check_name(name: &str) -> Result<&str, String> {
    let name = name.trim_end_matches([' ', '.']).trim_start();
    if name.is_empty() || name == "." || name == ".." {
        return Err("Ungültiger Name".into());
    }
    if let Some(c) = name.chars().find(|c| r#"\/:*?"<>|"#.contains(*c) || c.is_control()) {
        return Err(format!("Ein Name darf das Zeichen {c} nicht enthalten"));
    }
    Ok(name)
}

/// Renames within the same folder and returns the new path.
#[tauri::command]
pub async fn rename(path: String, name: String) -> Result<String, String> {
    blocking(move || {
        let from = PathBuf::from(&path);
        let to = from.with_file_name(check_name(&name)?);
        // Windows names are case-insensitive: "a.txt" → "A.txt" is fine, anything else must be free.
        let same = from.to_string_lossy().eq_ignore_ascii_case(&to.to_string_lossy());
        if !same && fs::symlink_metadata(&to).is_ok() {
            return Err(format!("„{}“ existiert bereits", to.file_name().unwrap_or_default().to_string_lossy()));
        }
        fs::rename(&from, &to).map_err(|e| e.to_string())?;
        Ok(to.to_string_lossy().into_owned())
    })
    .await
}

/// `name`, or `stem (2).ext`, `stem (3).ext`, … – the first that does not exist in `dir`.
fn unique(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if fs::symlink_metadata(&candidate).is_err() {
        return candidate;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (2..)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| fs::symlink_metadata(p).is_err())
        .unwrap()
}

/// Creates an empty folder or file with a free name based on `name` and returns its path.
#[tauri::command]
pub async fn create(dir: String, name: String, folder: bool) -> Result<String, String> {
    blocking(move || {
        let path = unique(&normalize(&dir), check_name(&name)?);
        if folder {
            fs::create_dir(&path)
        } else {
            fs::File::create_new(&path).map(drop)
        }
        .map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().into_owned())
    })
    .await
}

const PREVIEW_BYTES: u64 = 64 * 1024;

/// The beginning of a text file, `None` for binary files.
#[tauri::command]
pub async fn read_text(path: String) -> Result<Option<String>, String> {
    blocking(move || {
        let mut buf = Vec::new();
        fs::File::open(&path)
            .and_then(|f| f.take(PREVIEW_BYTES).read_to_end(&mut buf))
            .map_err(|e| e.to_string())?;
        Ok(text_of(&buf))
    })
    .await
}

fn text_of(buf: &[u8]) -> Option<String> {
    let buf = buf.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(buf);
    if buf.contains(&0) {
        return None;
    }
    // The cut at PREVIEW_BYTES may split a character – drop the broken tail.
    let text = match std::str::from_utf8(buf) {
        Ok(text) => text.to_owned(),
        Err(e) if e.error_len().is_none() => String::from_utf8_lossy(&buf[..e.valid_up_to()]).into_owned(),
        // Not UTF-8: most likely Windows-1252, close enough to Latin-1 for a preview.
        Err(_) => buf.iter().map(|&b| b as char).collect(),
    };
    Some(text)
}

fn walk(path: &Path, total: &mut FolderSize) {
    let Ok(entries) = fs::read_dir(path) else {
        total.errors += 1;
        return;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            total.errors += 1;
            continue;
        };
        match entry.metadata() {
            Ok(meta) if meta.is_symlink() => {}
            Ok(meta) if meta.is_dir() => {
                total.dirs += 1;
                walk(&entry.path(), total);
            }
            Ok(meta) => {
                total.files += 1;
                total.size += meta.len();
            }
            Err(_) => total.errors += 1,
        }
    }
}

/// Total size of a folder and everything below it (links are not followed).
#[tauri::command]
pub async fn folder_size(path: String) -> Result<FolderSize, String> {
    blocking(move || {
        let mut total = FolderSize::default();
        walk(&normalize(&path), &mut total);
        Ok(total)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mrfilesys-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn unique_appends_a_number() {
        let dir = temp_dir("unique");
        assert_eq!(unique(&dir, "a.txt"), dir.join("a.txt"));
        fs::write(dir.join("a.txt"), "").unwrap();
        fs::write(dir.join("a (2).txt"), "").unwrap();
        assert_eq!(unique(&dir, "a.txt"), dir.join("a (3).txt"));
        fs::create_dir(dir.join("Neuer Ordner")).unwrap();
        assert_eq!(unique(&dir, "Neuer Ordner"), dir.join("Neuer Ordner (2)"));
        assert_eq!(unique(&dir, ".gitignore"), dir.join(".gitignore"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn names_are_checked() {
        assert_eq!(check_name("  a.txt. ").unwrap(), "a.txt");
        assert!(check_name("..").is_err());
        assert!(check_name("a/b").is_err());
        assert!(check_name("a?").is_err());
    }

    #[test]
    fn text_detection() {
        assert_eq!(text_of(b"\xEF\xBB\xBFhallo").as_deref(), Some("hallo"));
        assert_eq!(text_of(b"a\0b"), None);
        // "ä" cut in half at the end
        assert_eq!(text_of(b"ab\xC3").as_deref(), Some("ab"));
        assert_eq!(text_of(b"gr\xFC\xDF").as_deref(), Some("grüß"));
    }

    #[test]
    fn folder_size_counts_recursively() {
        let dir = temp_dir("size");
        fs::write(dir.join("a"), "12345").unwrap();
        fs::create_dir(dir.join("sub")).unwrap();
        fs::write(dir.join("sub").join("b"), "123").unwrap();
        let mut total = FolderSize::default();
        walk(&dir, &mut total);
        assert_eq!((total.size, total.files, total.dirs, total.errors), (8, 2, 1, 0));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn drive_letter_means_root() {
        assert_eq!(normalize("C:"), PathBuf::from("C:\\"));
        assert_eq!(normalize(" C:\\x "), PathBuf::from("C:\\x"));
    }

    #[test]
    fn environment_variables_are_expanded() {
        std::env::set_var("MRFILESYS_TEST_DIR", "C:\\Users\\me");
        assert_eq!(expand_env("%mrfilesys_test_dir%\\x"), "C:\\Users\\me\\x");
        assert_eq!(expand_env("100%%MRFILESYS_TEST_DIR%"), "100%C:\\Users\\me");
        assert_eq!(expand_env("%NOT_SET_12345%\\a"), "%NOT_SET_12345%\\a");
        assert_eq!(expand_env("a%b"), "a%b");
        assert_eq!(normalize("%MRFILESYS_TEST_DIR%"), PathBuf::from("C:\\Users\\me"));
    }
}
