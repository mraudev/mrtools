// Gesprächsverlauf in history.json, neueste Einträge zuerst – wie src/history.js. Neue Einträge kommen
// mit den Gesprächen (Stufe 3); hier nur Lesen und Löschen.
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct History {
    file: PathBuf,
    pub entries: Vec<Value>,
}

impl History {
    pub fn load(dir: &Path) -> Self {
        let file = dir.join("history.json");
        let entries = fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok())
            .unwrap_or_default();
        Self { file, entries }
    }

    pub fn clear(&mut self) -> std::io::Result<()> {
        self.entries.clear();
        fs::write(&self.file, crate::json_indent1(&json!(self.entries)))
    }
}
