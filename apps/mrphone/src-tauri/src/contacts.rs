// Telefonbuch in contacts.json: [{ id, name, company, numbers: [{ label, number }], source }] – wie src/contacts.js.
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

// Rufnummer vergleichbar/wählbar machen: "+49 (30) 123-45" -> "03012345", "+43 1 234" -> "00431234".
pub fn normalize_number(value: &str) -> String {
    let lower = value.to_lowercase();
    let mut s = value;
    for scheme in ["sips:", "sip:", "tel:"] {
        if lower.starts_with(scheme) {
            s = &value[scheme.len()..];
            break;
        }
    }
    let s = s.split('@').next().unwrap_or_default();
    let n: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || matches!(c, '+' | '*' | '#'))
        .collect();
    if let Some(rest) = n.strip_prefix("+49") {
        format!("0{rest}")
    } else if let Some(rest) = n.strip_prefix("0049") {
        format!("0{rest}")
    } else if let Some(rest) = n.strip_prefix('+') {
        format!("00{rest}")
    } else {
        n
    }
}

// Gleich, oder bei längeren Nummern gleiche Endung (Amtsholung davor oder ohne führende 0).
pub fn numbers_match(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let core = short.trim_start_matches('0');
    core.len() >= 7 && long.ends_with(core)
}

// Sortierschlüssel wie localeCompare(…, 'de', { sensitivity: 'base' }): ohne Groß/klein und Umlaute.
fn sort_key(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .flat_map(|c| match c {
            'ä' | 'à' | 'á' | 'â' => vec!['a'],
            'ö' | 'ò' | 'ó' | 'ô' => vec!['o'],
            'ü' | 'ù' | 'ú' | 'û' => vec!['u'],
            'é' | 'è' | 'ê' => vec!['e'],
            'ß' => vec!['s', 's'],
            other => vec![other],
        })
        .collect()
}

fn text(v: &Value) -> String {
    v.as_str().map(str::to_string).unwrap_or_default()
}

pub struct Contacts {
    file: PathBuf,
    pub entries: Vec<Value>,
    index: Vec<(String, String)>, // (normalisierte Nummer, Name)
}

impl Contacts {
    pub fn load(dir: &Path) -> Self {
        let file = dir.join("contacts.json");
        let entries = fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|c| c["name"].is_string() && c["numbers"].is_array())
            .collect();
        let mut c = Self {
            file,
            entries,
            index: Vec::new(),
        };
        c.reindex();
        c
    }

    fn reindex(&mut self) {
        self.entries.sort_by_key(|c| sort_key(&text(&c["name"])));
        self.index = self
            .entries
            .iter()
            .flat_map(|c| {
                let name = text(&c["name"]);
                c["numbers"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(move |n| (normalize_number(&text(&n["number"])), name.clone()))
            })
            .collect();
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        self.reindex();
        fs::write(&self.file, crate::json_indent1(&json!(self.entries)))
    }

    pub fn lookup(&self, uri_or_number: &str) -> Option<String> {
        let norm = normalize_number(uri_or_number);
        if norm.is_empty() {
            return None;
        }
        self.index
            .iter()
            .find(|(n, _)| *n == norm)
            .or_else(|| self.index.iter().find(|(n, _)| numbers_match(n, &norm)))
            .map(|(_, name)| name.clone())
    }

    pub fn upsert(&mut self, data: &Value) -> Result<Value, String> {
        let name = text(&data["name"]).trim().to_string();
        let numbers: Vec<Value> = data["numbers"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|n| json!({ "label": text(&n["label"]).trim(), "number": text(&n["number"]).trim() }))
            .filter(|n| !normalize_number(n["number"].as_str().unwrap_or_default()).is_empty())
            .collect();
        if name.is_empty() {
            return Err("Bitte einen Namen eingeben.".into());
        }
        if numbers.is_empty() {
            return Err("Bitte mindestens eine Telefonnummer eingeben.".into());
        }
        let id = text(&data["id"]);
        let existing = (!id.is_empty())
            .then(|| self.entries.iter().position(|c| text(&c["id"]) == id))
            .flatten();
        let contact = json!({
            "id": existing.map(|i| text(&self.entries[i]["id"])).unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            "name": name,
            "company": text(&data["company"]).trim(),
            "numbers": numbers,
            "source": existing.map(|i| text(&self.entries[i]["source"])).unwrap_or_else(|| "manuell".into()),
        });
        match existing {
            Some(i) => self.entries[i] = contact.clone(),
            None => self.entries.push(contact.clone()),
        }
        self.save().map_err(|e| e.to_string())?;
        Ok(contact)
    }

    pub fn remove(&mut self, id: &str) -> std::io::Result<()> {
        self.entries.retain(|c| text(&c["id"]) != id);
        self.save()
    }

    // Fürs Fenster: je Nummer zusätzlich die wählbare Form ("dial").
    pub fn view(&self) -> Value {
        json!(self
            .entries
            .iter()
            .map(|c| {
                let mut c = c.clone();
                let numbers: Vec<Value> = c["numbers"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|mut n| {
                        n["dial"] = json!(normalize_number(&text(&n["number"])));
                        n
                    })
                    .collect();
                c["numbers"] = json!(numbers);
                c
            })
            .collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_like_electron() {
        assert_eq!(normalize_number("+49 (30) 123-45"), "03012345");
        assert_eq!(normalize_number("+43 1 234"), "00431234");
        assert_eq!(normalize_number("0049 171 5550123"), "01715550123");
        assert_eq!(normalize_number("sip:742@sip.crossbase.net"), "742");
        assert_eq!(normalize_number("SIP:*76742@x"), "*76742");
    }

    #[test]
    fn match_with_trunk_prefix() {
        assert!(numbers_match("01715550123", "001715550123"));
        assert!(numbers_match("1715550123", "01715550123"));
        assert!(!numbers_match("742", "0742"));
    }

    #[test]
    fn upsert_lookup_sort() {
        let dir = std::env::temp_dir().join(format!("mrphone-contacts-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let mut c = Contacts::load(&dir);
        c.upsert(&json!({ "name": "Zoe", "numbers": [{ "label": "Mobil", "number": "+49 171 5550123" }] })).unwrap();
        c.upsert(
            &json!({ "name": "Ärger", "numbers": [{ "label": "Mobil", "number": "030 1234567" }] }),
        )
        .unwrap();
        assert!(c.upsert(&json!({ "name": "", "numbers": [] })).is_err());
        assert_eq!(c.entries[0]["name"], "Ärger"); // Ä wie A einsortiert
        assert_eq!(c.lookup("sip:01715550123@x").as_deref(), Some("Zoe"));
        let again = Contacts::load(&dir);
        assert_eq!(again.entries.len(), 2);
        fs::remove_dir_all(&dir).unwrap();
    }
}
