//! PDF-Ausgabe mit Typst. Die Vorlagen (`vorlagen/*.typ`) bekommen ihre Daten als `daten.json`.

pub mod druck;

use crate::db::Res;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst_layout::PagedDocument;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;

const VORLAGEN: &[(&str, &str)] = &[
    ("briefpapier.typ", include_str!("vorlagen/briefpapier.typ")),
    ("beleg.typ", include_str!("vorlagen/beleg.typ")),
    ("brief.typ", include_str!("vorlagen/brief.typ")),
    ("mahnung.typ", include_str!("vorlagen/mahnung.typ")),
    ("aufmass.typ", include_str!("vorlagen/aufmass.typ")),
    ("regie.typ", include_str!("vorlagen/regie.typ")),
    ("etiketten.typ", include_str!("vorlagen/etiketten.typ")),
];

fn schriften() -> &'static FontStore {
    static FONTS: OnceLock<FontStore> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut store = FontStore::new();
        store.extend(typst_kit::fonts::system());
        store.extend(typst_kit::fonts::embedded());
        store
    })
}

/// Schriften im Hintergrund laden, damit das erste PDF schnell kommt.
pub fn vorladen() {
    std::thread::spawn(|| {
        schriften();
    });
}

struct Welt {
    library: LazyHash<Library>,
    main: FileId,
    quellen: HashMap<FileId, Source>,
    dateien: HashMap<FileId, Bytes>,
}

fn id(name: &str) -> FileId {
    RootedPath::new(VirtualRoot::Project, VirtualPath::new(name).expect("gültiger Dateiname")).intern()
}

fn fehlt(id: FileId) -> FileError {
    FileError::NotFound(std::path::PathBuf::from(format!("{id:?}")))
}

impl Welt {
    fn neu(haupt: &str, dateien: HashMap<String, Vec<u8>>) -> Self {
        let mut quellen = HashMap::new();
        for (name, text) in VORLAGEN {
            quellen.insert(id(name), Source::new(id(name), text.to_string()));
        }
        Self {
            library: LazyHash::new(Library::default()),
            main: id(haupt),
            quellen,
            dateien: dateien.into_iter().map(|(n, d)| (id(&n), Bytes::new(d))).collect(),
        }
    }
}

impl World for Welt {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        schriften().book()
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        self.quellen.get(&id).cloned().ok_or_else(|| fehlt(id))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(d) = self.dateien.get(&id) {
            return Ok(d.clone());
        }
        if let Some(s) = self.quellen.get(&id) {
            return Ok(Bytes::from_string(s.text().to_string()));
        }
        Err(fehlt(id))
    }
    fn font(&self, index: usize) -> Option<Font> {
        schriften().font(index)
    }
    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        use chrono::Datelike;
        let t = chrono::Local::now();
        Datetime::from_ymd(t.year(), t.month() as u8, t.day() as u8)
    }
}

/// Setzt `vorlage` mit `daten` (als `daten.json`) und weiteren Dateien (Bilder, XML) zu einem PDF.
pub fn setzen<T: Serialize>(vorlage: &str, daten: &T, mut dateien: HashMap<String, Vec<u8>>, pdf_a3: bool) -> Res<Vec<u8>> {
    dateien.insert("daten.json".into(), serde_json::to_vec(daten).map_err(|e| e.to_string())?);
    let welt = Welt::neu(vorlage, dateien);
    let ergebnis = typst::compile::<PagedDocument>(&welt);
    let dokument = ergebnis.output.map_err(|fehler| {
        fehler.iter().map(|f| f.message.to_string()).collect::<Vec<_>>().join("; ")
    })?;
    let jetzt = chrono::Local::now();
    let zeit = {
        use chrono::{Datelike, Timelike};
        Datetime::from_ymd_hms(jetzt.year(), jetzt.month() as u8, jetzt.day() as u8, jetzt.hour() as u8, jetzt.minute() as u8, jetzt.second() as u8)
            .and_then(|d| typst_pdf::Timestamp::new_local(d, jetzt.offset().local_minus_utc() / 60))
    };
    let mut optionen = typst_pdf::PdfOptions {
        creator: typst::foundations::Smart::Custom(Some("mrfakt".into())),
        timestamp: zeit,
        ..Default::default()
    };
    if pdf_a3 {
        optionen.standards = typst_pdf::PdfStandards::new(&[typst_pdf::PdfStandard::A_3b]).map_err(|e| e.message().to_string())?;
    }
    typst_pdf::pdf(&dokument, &optionen).map_err(|fehler| fehler.iter().map(|f| f.message.to_string()).collect::<Vec<_>>().join("; "))
}

/// Betrag deutsch formatiert: 1.234,56
pub fn betrag(d: Decimal) -> String {
    zahl_de(crate::rechnen::cent(d), 2, 2)
}

/// Menge mit 2–3 Nachkommastellen: 12,50 / 1,125
pub fn menge(d: Decimal) -> String {
    zahl_de(d, 2, 3)
}

pub fn zahl_de(d: Decimal, min: u32, max: u32) -> String {
    let d = d.round_dp_with_strategy(max, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
    let neg = d.is_sign_negative() && !d.is_zero();
    let s = format!("{:.*}", max as usize, d.abs());
    let (ganz, mut nach) = s.split_once('.').map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((s.clone(), String::new()));
    while nach.len() > min as usize && nach.ends_with('0') {
        nach.pop();
    }
    let mut g = String::new();
    for (i, c) in ganz.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            g.push('.');
        }
        g.push(c);
    }
    let ganz: String = g.chars().rev().collect();
    let text = if nach.is_empty() { ganz } else { format!("{ganz},{nach}") };
    if neg { format!("-{text}") } else { text }
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct Teil {
    pub t: String,
    pub b: bool,
    pub i: bool,
    pub u: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct Absatz {
    /// "", "ul" oder "ol"
    pub liste: String,
    pub nr: usize,
    pub teile: Vec<Teil>,
}

/// Das einfache HTML des Editors (`p`, `br`, `strong`, `em`, `u`, `ul`/`ol`/`li`) in Absätze für die Vorlagen.
pub fn html_absaetze(html: &str) -> Vec<Absatz> {
    let mut out: Vec<Absatz> = vec![];
    let mut aktuell = Absatz::default();
    let (mut b, mut i, mut u) = (0i32, 0i32, 0i32);
    let mut liste: Vec<(String, usize)> = vec![];
    let mut text = String::new();
    let chars: Vec<char> = html.chars().collect();
    let mut k = 0;

    fn schieben(aktuell: &mut Absatz, text: &mut String, b: i32, i: i32, u: i32) {
        if text.is_empty() {
            return;
        }
        let neu = Teil { t: std::mem::take(text), b: b > 0, i: i > 0, u: u > 0 };
        match aktuell.teile.last_mut() {
            Some(t) if (t.b, t.i, t.u) == (neu.b, neu.i, neu.u) => t.t.push_str(&neu.t),
            _ => aktuell.teile.push(neu),
        }
    }
    fn absatz_ende(out: &mut Vec<Absatz>, aktuell: &mut Absatz, liste: &[(String, usize)]) {
        let mut a = std::mem::take(aktuell);
        if let Some(l) = liste.last() {
            aktuell.liste = l.0.clone();
        }
        if !a.teile.is_empty() || a.liste.is_empty() {
            if a.teile.iter().all(|t| t.t.trim().is_empty()) && a.liste.is_empty() {
                a.teile.clear();
            }
            out.push(a);
        }
    }

    while k < chars.len() {
        let c = chars[k];
        if c == '<' {
            let ende = chars[k..].iter().position(|&x| x == '>').map(|p| k + p).unwrap_or(chars.len() - 1);
            let tag: String = chars[k + 1..ende].iter().collect::<String>().to_lowercase();
            let schliessend = tag.starts_with('/');
            let name: String = tag.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            schieben(&mut aktuell, &mut text, b, i, u);
            match (name.as_str(), schliessend) {
                ("strong" | "b", false) => b += 1,
                ("strong" | "b", true) => b -= 1,
                ("em" | "i", false) => i += 1,
                ("em" | "i", true) => i -= 1,
                ("u", false) => u += 1,
                ("u", true) => u -= 1,
                ("br", _) => text.push('\n'),
                ("p" | "div" | "h1" | "h2" | "h3", true) => absatz_ende(&mut out, &mut aktuell, &liste),
                ("ul" | "ol", false) => liste.push((name.clone(), 0)),
                ("ul" | "ol", true) => {
                    liste.pop();
                    aktuell.liste = liste.last().map(|l| l.0.clone()).unwrap_or_default();
                }
                ("li", false) => {
                    if let Some(l) = liste.last_mut() {
                        l.1 += 1;
                        aktuell.liste = l.0.clone();
                        aktuell.nr = l.1;
                    }
                }
                ("li", true) => {
                    let nr = aktuell.nr;
                    absatz_ende(&mut out, &mut aktuell, &liste);
                    aktuell.nr = nr;
                }
                _ => {}
            }
            k = ende + 1;
            continue;
        }
        if c == '&' {
            let ende = chars[k..].iter().take(10).position(|&x| x == ';').map(|p| k + p);
            if let Some(e) = ende {
                let ent: String = chars[k + 1..e].iter().collect();
                let ersatz = match ent.as_str() {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" | "#39" => Some('\''),
                    "nbsp" | "#160" => Some('\u{a0}'),
                    _ => None,
                };
                if let Some(z) = ersatz {
                    text.push(z);
                    k = e + 1;
                    continue;
                }
            }
        }
        if c != '\n' && c != '\r' {
            text.push(c);
        }
        k += 1;
    }
    schieben(&mut aktuell, &mut text, b, i, u);
    if !aktuell.teile.is_empty() {
        out.push(aktuell);
    }
    while out.last().is_some_and(|a| a.teile.is_empty()) {
        out.pop();
    }
    out
}

/// Klartext (z. B. Positionstexte) in Absätze.
pub fn text_absaetze(text: &str) -> Vec<Absatz> {
    if text.trim().is_empty() {
        return vec![];
    }
    vec![Absatz { teile: vec![Teil { t: text.trim_end().to_string(), ..Default::default() }], ..Default::default() }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn zahlen_deutsch() {
        let d = |s: &str| Decimal::from_str(s).unwrap();
        assert_eq!(betrag(d("1234567.5")), "1.234.567,50");
        assert_eq!(betrag(d("-0.5")), "-0,50");
        assert_eq!(menge(d("12.5")), "12,50");
        assert_eq!(menge(d("1.125")), "1,125");
        assert_eq!(menge(d("265")), "265,00");
        assert_eq!(betrag(Decimal::ZERO), "0,00");
    }

    #[test]
    fn html_zu_absaetzen() {
        let a = html_absaetze("<p>Für <strong>Maler</strong>arbeiten &amp; mehr</p><p></p><ul><li><p>eins</p></li><li><p>zwei</p></li></ul><p>A<br>B</p>");
        assert_eq!(a.len(), 5);
        assert_eq!(a[0].teile.len(), 3);
        assert!(a[0].teile[1].b);
        assert_eq!(a[0].teile[2].t, "arbeiten & mehr");
        assert!(a[1].teile.is_empty());
        assert_eq!((a[2].liste.as_str(), a[2].nr), ("ul", 1));
        assert_eq!((a[3].liste.as_str(), a[3].nr), ("ul", 2));
        assert_eq!(a[4].teile[0].t, "A\nB");
        assert_eq!(a[4].liste, "");
    }

    #[test]
    fn einfaches_pdf() {
        let pdf = setzen("etiketten.typ", &serde_json::json!({"etiketten": [["Hallo", "Welt"]], "spalten": 3, "hoehe": 37}), HashMap::new(), false).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }
}
