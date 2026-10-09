// Kontakte importieren/exportieren – wie src/csvimport.js, src/csvexport.js und src/outlook.js.
use serde_json::{json, Value};

// --- CSV-Import: Dateien, wie sie Outlook (klassisch, neu, Outlook.com) exportiert ---
// Spaltennamen deutsch oder englisch; unbekannte Telefon-Spalten werden mit ihrem Namen übernommen.
const PHONE_LABELS: [(&str, &str); 20] = [
    ("business phone", "Geschäftlich"),
    ("telefon geschäftlich", "Geschäftlich"),
    ("business phone 2", "Geschäftlich 2"),
    ("telefon geschäftlich 2", "Geschäftlich 2"),
    ("company main phone", "Firma"),
    ("telefon firma", "Firma"),
    ("mobile phone", "Mobil"),
    ("mobiltelefon", "Mobil"),
    ("home phone", "Privat"),
    ("telefon privat", "Privat"),
    ("home phone 2", "Privat 2"),
    ("telefon privat 2", "Privat 2"),
    ("primary phone", "Haupt"),
    ("haupttelefon", "Haupt"),
    ("other phone", "Weitere"),
    ("weiteres telefon", "Weitere"),
    ("car phone", "Auto"),
    ("autotelefon", "Auto"),
    ("assistant's phone", "Assistenz"),
    ("telefon assistent", "Assistenz"),
];
const NAME: [&str; 4] = ["name", "full name", "display name", "anzeigename"];
const FIRST: [&str; 2] = ["first name", "vorname"];
const MIDDLE: [&str; 2] = ["middle name", "weitere vornamen"];
const LAST: [&str; 2] = ["last name", "nachname"];
const COMPANY: [&str; 2] = ["company", "firma"];

fn is_phone_column(name: &str) -> bool {
    ["phone", "telefon", "mobil", "handy"]
        .iter()
        .any(|k| name.contains(k))
}

// RFC 4180: Felder in "..." dürfen Trennzeichen, Zeilenumbrüche (Notizen!) und "" enthalten.
fn parse_csv(text: &str, sep: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            if ch != '"' {
                field.push(ch);
            } else if chars.peek() == Some(&'"') {
                field.push('"');
                chars.next();
            } else {
                quoted = false;
            }
        } else if ch == '"' {
            quoted = true;
        } else if ch == sep {
            row.push(std::mem::take(&mut field));
        } else if ch == '\n' || ch == '\r' {
            if ch == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
        } else {
            field.push(ch);
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

// Klassisches Outlook exportiert ANSI (Windows-1252), neuere Exporte UTF-8.
pub fn read_contacts_csv(bytes: &[u8]) -> Result<Vec<Value>, String> {
    let decoded = crate::sip::msg::decode_text(bytes);
    let text = decoded.strip_prefix('\u{feff}').unwrap_or(&decoded); // BOM (Excel/UTF-8) verwerfen
    let head = text.lines().next().unwrap_or_default();
    let sep = [';', '\t'].into_iter().fold(',', |best, s| {
        if head.split(s).count() > head.split(best).count() {
            s
        } else {
            best
        }
    });
    let mut rows = parse_csv(text, sep).into_iter();
    let Some(header) = rows.next() else {
        return Ok(Vec::new());
    };
    let cols: Vec<String> = header.iter().map(|h| h.trim().to_lowercase()).collect();
    let find = |names: &[&str]| cols.iter().position(|c| names.contains(&c.as_str()));
    let (i_name, i_first, i_middle, i_last, i_company) = (
        find(&NAME),
        find(&FIRST),
        find(&MIDDLE),
        find(&LAST),
        find(&COMPANY),
    );
    let phones: Vec<(usize, String)> = cols
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let known = PHONE_LABELS
                .iter()
                .find(|(k, _)| k == c)
                .map(|(_, l)| l.to_string());
            known
                .or_else(|| {
                    (is_phone_column(c) && !c.contains("fax")).then(|| header[i].trim().to_string())
                })
                .map(|label| (i, label))
        })
        .collect();
    if phones.is_empty() {
        return Err(
            "Die Datei enthält keine Telefon-Spalten (z. B. „Mobile Phone“ oder „Mobiltelefon“)."
                .into(),
        );
    }
    let get = |row: &[String], i: Option<usize>| {
        i.and_then(|i| row.get(i))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    Ok(rows
        .filter_map(|row| {
            let company = get(&row, i_company);
            let mut name = get(&row, i_name);
            if name.is_empty() {
                name = [i_first, i_middle, i_last]
                    .iter()
                    .map(|&i| get(&row, i))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            if name.is_empty() {
                name = company.clone();
            }
            let numbers: Vec<Value> = phones
                .iter()
                .map(|(i, label)| json!({ "label": label, "number": get(&row, Some(*i)) }))
                .filter(|n| n["number"] != "")
                .collect();
            (!name.is_empty() && !numbers.is_empty())
                .then(|| json!({ "name": name, "company": company, "numbers": numbers }))
        })
        .collect())
}

// --- CSV-Export: Spaltennamen so, dass unser Import und das klassische Outlook sie wieder einlesen ---
const LABEL_HEADERS: [(&str, &str); 10] = [
    ("Geschäftlich", "Telefon geschäftlich"),
    ("Geschäftlich 2", "Telefon geschäftlich 2"),
    ("Mobil", "Mobiltelefon"),
    ("Privat", "Telefon privat"),
    ("Privat 2", "Telefon privat 2"),
    ("Firma", "Telefon Firma"),
    ("Weitere", "Weiteres Telefon"),
    ("Haupt", "Haupttelefon"),
    ("Auto", "Autotelefon"),
    ("Assistenz", "Telefon Assistent"),
];
const STANDARD: [&str; 5] = [
    "Telefon geschäftlich",
    "Mobiltelefon",
    "Telefon privat",
    "Telefon Firma",
    "Weiteres Telefon",
];

// Unbekannte Bezeichnung ohne Telefon-Stichwort bekommt „Telefon “ davor (sonst ginge die Spalte
// beim Wiederimport verloren).
fn header_for(label: &str) -> String {
    if let Some((_, h)) = LABEL_HEADERS.iter().find(|(l, _)| *l == label) {
        return h.to_string();
    }
    if label.is_empty() {
        return "Telefon".into();
    }
    if is_phone_column(&label.to_lowercase()) {
        label.into()
    } else {
        format!("Telefon {label}")
    }
}

fn quote(value: &str) -> String {
    if value.contains(['"', ';', ',', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

// Name, Firma, (Spalte, Nummer)
type Row = (String, String, Vec<(String, String)>);

// Trennzeichen ';' (deutsches Excel öffnet damit direkt in Spalten), Zeilenende CRLF.
pub fn contacts_to_csv(entries: &[Value]) -> String {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let rows: Vec<Row> = entries
        .iter()
        .map(|c| {
            let mut seen: Vec<(String, usize)> = Vec::new();
            let mut phones: Vec<(String, String)> = Vec::new();
            for n in c["numbers"].as_array().cloned().unwrap_or_default() {
                let number = text(&n["number"]);
                if number.is_empty() {
                    continue;
                }
                let base = header_for(&text(&n["label"]));
                let k = match seen.iter_mut().find(|(b, _)| *b == base) {
                    Some((_, k)) => {
                        *k += 1;
                        *k
                    }
                    None => {
                        seen.push((base.clone(), 1));
                        1
                    }
                };
                let column = if k > 1 { format!("{base} {k}") } else { base };
                match phones.iter_mut().find(|(h, _)| *h == column) {
                    Some(p) => p.1 = number,
                    None => phones.push((column, number)),
                }
            }
            (text(&c["name"]), text(&c["company"]), phones)
        })
        .collect();
    // Spalten: benutzte Standardspalten in fester Reihenfolge, dann übrige nach Auftreten.
    let mut columns: Vec<String> = Vec::new();
    let has = |r: &Row, h: &str| r.2.iter().any(|(c, _)| c == h);
    for h in STANDARD {
        if rows.iter().any(|r| has(r, h)) {
            columns.push(h.into());
        }
    }
    for r in &rows {
        for (h, _) in &r.2 {
            if !columns.contains(h) {
                columns.push(h.clone());
            }
        }
    }
    let mut lines = vec![["Name".to_string(), "Firma".to_string()]
        .into_iter()
        .chain(columns.iter().cloned())
        .collect::<Vec<_>>()];
    for (name, company, phones) in &rows {
        let mut line = vec![name.clone(), company.clone()];
        line.extend(columns.iter().map(|h| {
            phones
                .iter()
                .find(|(c, _)| c == h)
                .map(|(_, n)| n.clone())
                .unwrap_or_default()
        }));
        lines.push(line);
    }
    lines
        .iter()
        .map(|l| l.iter().map(|f| quote(f)).collect::<Vec<_>>().join(";"))
        .collect::<Vec<_>>()
        .join("\r\n")
        + "\r\n"
}

// --- Outlook: per COM alle Kontaktordner aller Konten im klassischen Outlook lesen ---
const OUTLOOK_FIELDS: [(&str, &str); 10] = [
    ("BusinessTelephoneNumber", "Geschäftlich"),
    ("Business2TelephoneNumber", "Geschäftlich 2"),
    ("CompanyMainTelephoneNumber", "Firma"),
    ("MobileTelephoneNumber", "Mobil"),
    ("HomeTelephoneNumber", "Privat"),
    ("Home2TelephoneNumber", "Privat 2"),
    ("PrimaryTelephoneNumber", "Haupt"),
    ("AssistantTelephoneNumber", "Assistenz"),
    ("CarTelephoneNumber", "Auto"),
    ("OtherTelephoneNumber", "Weitere"),
];

const OUTLOOK_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$fields = @(FIELDS)
$ol = New-Object -ComObject Outlook.Application
$ns = $ol.GetNamespace('MAPI')
$result = New-Object System.Collections.ArrayList
function Read-Folder($folder) {
  if ($folder.DefaultItemType -eq 2) {
    foreach ($item in $folder.Items) {
      if ($item.Class -ne 40) { continue }
      $numbers = @(foreach ($f in $fields) { $v = $item.$f; if ($v) { @{ field = $f; number = [string]$v } } })
      $name = if ($item.FullName) { $item.FullName } else { $item.CompanyName }
      if ($name -and $numbers.Count) { [void]$result.Add(@{ name = [string]$name; company = [string]$item.CompanyName; numbers = $numbers }) }
    }
  }
  foreach ($sub in $folder.Folders) { Read-Folder $sub }
}
foreach ($store in $ns.Stores) { try { Read-Folder $store.GetRootFolder() } catch { } }
ConvertTo-Json -InputObject @($result) -Depth 5 -Compress
"#;

pub async fn outlook_contacts() -> Result<Vec<Value>, String> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    let fields = OUTLOOK_FIELDS
        .iter()
        .map(|(f, _)| format!("'{f}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let script = OUTLOOK_SCRIPT.replace("FIELDS", &fields);
    // -EncodedCommand (UTF-16LE, Base64) umgeht jedes Quoting-Problem der Kommandozeile
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut cmd = tokio::process::Command::new("powershell.exe");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
    ])
    .arg(B64.encode(utf16))
    .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let out = tokio::time::timeout(std::time::Duration::from_secs(180), cmd.output())
        .await
        .map_err(|_| "Outlook-Import fehlgeschlagen: keine Antwort von Outlook".to_string())?
        .map_err(|e| format!("Outlook-Import fehlgeschlagen: {e}"))?;
    if !out.status.success() {
        let detail = String::from_utf8_lossy(&out.stderr).to_string();
        if ["COM class factory", "80040154", "Outlook.Application"]
            .iter()
            .any(|k| detail.to_lowercase().contains(&k.to_lowercase()))
        {
            return Err("Das klassische Outlook wurde nicht gefunden. Das neue Outlook bietet keinen direkten Zugriff.".into());
        }
        let first = detail
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("unbekannter Fehler");
        return Err(format!("Outlook-Import fehlgeschlagen: {first}"));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stdout = stdout.trim().trim_start_matches('\u{feff}');
    let items: Vec<Value> = serde_json::from_str(if stdout.is_empty() { "[]" } else { stdout })
        .map_err(|e| format!("Outlook-Antwort nicht lesbar: {e}"))?;
    Ok(items
        .iter()
        .map(|c| {
            let numbers: Vec<Value> = c["numbers"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|n| {
                    let field = n["field"].as_str().unwrap_or_default();
                    let label = OUTLOOK_FIELDS
                        .iter()
                        .find(|(f, _)| *f == field)
                        .map_or(field, |(_, l)| l);
                    json!({ "label": label, "number": n["number"] })
                })
                .collect();
            json!({ "name": c["name"], "company": c["company"].as_str().unwrap_or_default(), "numbers": numbers })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_outlook_german_ansi() {
        // Windows-1252, Semikolon, Notiz mit Zeilenumbruch, Fax-Spalte wird ignoriert
        let mut data =
            b"Vorname;Nachname;Firma;Mobiltelefon;Fax gesch\xe4ftlich;Notizen;Telefon Kantine\r\n"
                .to_vec();
        data.extend_from_slice(
            b"J\xf6rg;M\xfcller;ACME;0171 555;0301;\"Zeile 1\r\nZeile \"\"2\"\"\";42\r\n",
        );
        data.extend_from_slice(b";;Nur Firma;;;;99\r\n;;;;;;\r\n");
        let found = read_contacts_csv(&data).unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0]["name"], "Jörg Müller");
        assert_eq!(
            found[0]["numbers"],
            json!([{ "label": "Mobil", "number": "0171 555" }, { "label": "Telefon Kantine", "number": "42" }])
        );
        assert_eq!(found[1]["name"], "Nur Firma");
        assert!(read_contacts_csv(b"Name,Email\nA,a@x\n").is_err());
    }

    #[test]
    fn csv_roundtrip() {
        let entries = vec![
            json!({ "name": "Müller; Jörg", "company": "A \"B\"", "numbers": [
                { "label": "Mobil", "number": "0171" }, { "label": "Mobil", "number": "0172" },
                { "label": "Labor", "number": "12" }, { "label": "Handy privat", "number": "13" }] }),
            json!({ "name": "Zoe", "company": "", "numbers": [{ "label": "Geschäftlich", "number": "+49 30 1" }] }),
        ];
        let csv = contacts_to_csv(&entries);
        assert!(csv.starts_with("Name;Firma;Telefon geschäftlich;Mobiltelefon;Mobiltelefon 2;Telefon Labor;Handy privat\r\n"));
        let back = read_contacts_csv(csv.as_bytes()).unwrap();
        assert_eq!(back[0]["name"], "Müller; Jörg");
        assert_eq!(back[0]["company"], "A \"B\"");
        let numbers: Vec<&str> = back[0]["numbers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["number"].as_str().unwrap())
            .collect();
        assert_eq!(numbers, ["0171", "0172", "12", "13"]);
        assert_eq!(back[1]["numbers"][0]["label"], "Geschäftlich");
    }
}
