//! The apps of the mrtools repository, embedded at build time (see `build.rs`),
//! together with their installation state.

use crate::installed::{self, Installed};
use serde::{Deserialize, Serialize};

const CATALOG: &str = include_str!(concat!(env!("OUT_DIR"), "/catalog.json"));

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Folder under `apps/`, also the prefix of the release tags (`<folder>-v`).
    pub folder: String,
    /// Product name, as shown in Windows' installed apps.
    pub name: String,
    pub description: Option<String>,
    /// Logo as SVG source.
    pub icon: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct App {
    #[serde(flatten)]
    entry: Entry,
    installed: Option<Installed>,
}

pub fn entries() -> Vec<Entry> {
    serde_json::from_str(CATALOG).expect("catalog.json from build.rs")
}

#[tauri::command]
pub async fn apps() -> Result<Vec<App>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let installed = installed::all();
        entries()
            .into_iter()
            .map(|entry| App {
                installed: installed.iter().find(|i| i.name.eq_ignore_ascii_case(&entry.name)).cloned(),
                entry,
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::entries;

    #[test]
    fn catalog_lists_the_other_apps() {
        let folders: Vec<_> = entries().into_iter().map(|e| e.folder).collect();
        assert!(folders.contains(&"mrstart".to_string()));
        assert!(!folders.contains(&"mrtools".to_string()));
    }
}
