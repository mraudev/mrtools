// Gesprächsverlauf in history.json, neueste Einträge zuerst – wie src/history.js.
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_ENTRIES: usize = 200;

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

    // Neuer Eintrag nach Gesprächsende; Rückgabe: der Eintrag.
    pub fn add(&mut self, reason: &str, call: &Value) -> std::io::Result<Value> {
        let started = call["startedAt"].as_u64();
        let incoming = call["direction"] == "in";
        let status = match (started, incoming) {
            (Some(_), _) => "answered",
            (None, true) if call["rejected"] == true => "rejected",
            (None, true) => "missed",
            (None, false) => "unanswered",
        };
        // Ausgehende Anrufe kennen keinen Namen -> aus früheren Einträgen derselben Nummer übernehmen.
        let uri = call["remoteUri"].as_str().unwrap_or_default();
        let name = call["remoteName"]
            .as_str()
            .filter(|n| !n.is_empty())
            .map(String::from)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|e| {
                        e["remoteUri"] == uri
                            && e["remoteName"].as_str().is_some_and(|n| !n.is_empty())
                    })
                    .map(|e| e["remoteName"].as_str().unwrap_or_default().to_string())
            });
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let entry = json!({
            "direction": call["direction"],
            "remoteUri": uri,
            "remoteName": name.unwrap_or_default(),
            "at": call["createdAt"],
            "duration": started.map(|s| ((now.saturating_sub(s)) as f64 / 1000.0).round() as u64),
            "status": status,
            "reason": reason,
            "accountId": call["accountId"],
            "accountLabel": call["accountLabel"],
        });
        self.entries.insert(0, entry.clone());
        self.entries.truncate(MAX_ENTRIES);
        fs::write(&self.file, crate::json_indent1(&json!(self.entries)))?;
        Ok(entry)
    }

    pub fn clear(&mut self) -> std::io::Result<()> {
        self.replace(Vec::new())
    }

    // Aus einer Sicherung: nur Einträge mit Gegenstelle, höchstens MAX_ENTRIES.
    pub fn replace(&mut self, entries: Vec<Value>) -> std::io::Result<()> {
        self.entries = entries
            .into_iter()
            .filter(|e| e["remoteUri"].is_string())
            .take(MAX_ENTRIES)
            .collect();
        fs::write(&self.file, crate::json_indent1(&json!(self.entries)))
    }
}
