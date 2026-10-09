//! File system helpers for projects: watched folders and default apps.

use serde::Serialize;
use std::fs;
use std::path::Path;

/// Returns the files in `directory` that match one of `patterns`. A pattern is
/// either a file name (`All.groupproj`) or a suffix wildcard (`*.sln`).
pub fn find_files(directory: &Path, patterns: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for pattern in patterns.iter().map(|p| p.trim()).filter(|p| !p.is_empty()) {
        if let Some(suffix) = pattern.strip_prefix('*') {
            let suffix = suffix.to_lowercase();
            let Ok(entries) = fs::read_dir(directory) else {
                continue;
            };
            let mut matches: Vec<String> = entries
                .flatten()
                .filter(|e| e.path().is_file())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .to_lowercase()
                        .ends_with(&suffix)
                })
                .map(|e| e.path().to_string_lossy().into_owned())
                .collect();
            matches.sort();
            found.extend(matches);
        } else {
            let file = directory.join(pattern);
            if file.is_file() {
                found.push(file.to_string_lossy().into_owned());
            }
        }
    }
    found.dedup();
    found
}

#[tauri::command]
pub fn existing_files(directory: String, patterns: Vec<String>) -> Vec<String> {
    find_files(Path::new(&directory), &patterns)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedEntry {
    name: String,
    path: String,
    apps: Vec<String>,
    /// The watched directory this entry was found in, exactly as passed in.
    root: String,
}

/// Lists the direct subfolders of every watched directory.
#[tauri::command]
pub async fn watched_projects(
    directories: Vec<String>,
    default_apps: Vec<String>,
) -> Vec<WatchedEntry> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut entries: Vec<WatchedEntry> = directories
            .iter()
            .filter_map(|dir| Some((dir, fs::read_dir(dir).ok()?)))
            .flat_map(|(dir, read)| read.flatten().map(move |e| (dir, e)))
            .filter(|(_, e)| e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.'))
            .map(|(dir, e)| WatchedEntry {
                name: e.file_name().to_string_lossy().into_owned(),
                path: e.path().to_string_lossy().into_owned(),
                apps: find_files(&e.path(), &default_apps),
                root: dir.clone(),
            })
            .collect();
        entries.sort_by_key(|e| e.name.to_lowercase());
        entries
    })
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_exact_names_and_wildcards() {
        let dir = std::env::temp_dir().join(format!("mrstart-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for name in ["App.sln", "Tools.SLN", "build.cmd", "readme.md"] {
            fs::write(dir.join(name), "").unwrap();
        }

        let patterns = vec![
            "*.sln".to_string(),
            "build.cmd".to_string(),
            "missing.exe".to_string(),
        ];
        let names: Vec<String> = find_files(&dir, &patterns)
            .iter()
            .map(|p| {
                Path::new(p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(names, ["App.sln", "Tools.SLN", "build.cmd"]);
    }
}
