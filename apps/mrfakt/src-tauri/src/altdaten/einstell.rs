//! Einstellungen aus `EINSTELL\` und dem Programmordner von GHFakt.

use super::texte::{dekodieren, zahl};
use crate::einstellungen::{Allgemein, Firma, Mahnstufe, Zahlungsbedingung};
use crate::modelle::{Formel, Textbaustein};
use std::path::Path;

pub fn lesen(pfad: &Path) -> Option<String> {
    std::fs::read(pfad).ok().map(|b| dekodieren(&b))
}

fn zeilen(text: &str) -> Vec<String> {
    text.split('\n').map(|l| l.trim_end().to_string()).collect()
}

/// Absender, Briefkopf (RECHNKOX.STA) und Steuernummer (REKOPF01.DAT).
pub fn firma(ordner: &Path) -> Firma {
    let mut f = Firma::default();
    let e = ordner.join("EINSTELL");
    if let Some(t) = lesen(&e.join("Absender.Dat")) {
        let l = zeilen(&t);
        let g = |i: usize| l.get(i).map(|s| s.trim().to_string()).unwrap_or_default();
        f.name = g(0);
        f.inhaber = g(1);
        f.unterschrift = g(2);
        f.strasse = g(3);
        let ort = g(4);
        if let Some((plz, o)) = ort.split_once(' ') {
            f.plz = plz.into();
            f.ort = o.into();
        } else {
            f.ort = ort;
        }
        f.telefon = g(5);
        f.telefax = g(6);
        f.email = g(7);
        f.briefort = f.ort.clone();
    }
    if let Some(t) = lesen(&e.join("RECHNKOX.STA")) {
        let bloecke: Vec<String> = t
            .split("*Ende*")
            .enumerate()
            .map(|(i, b)| {
                // Vor jedem Text stehen Layoutzeilen (Position, Schrift, Größe, Farbe), vor dem ersten nur eine.
                let l: Vec<&str> = b.trim_start_matches('\n').split('\n').collect();
                let skip = if i == 0 { 1 } else { 9 };
                l.iter().skip(skip).map(|s| s.trim()).collect::<Vec<_>>().join("\n").trim().to_string()
            })
            .collect();
        let b = |i: usize| bloecke.get(i).cloned().unwrap_or_default();
        f.zusatz = b(1);
        f.fusszeile = b(2);
        f.absenderzeile = b(3);
        f.seitenleiste = b(4).lines().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n");
        for zeile in b(2).lines() {
            for teil in zeile.split('·') {
                let teil = teil.trim();
                if let Some(w) = teil.strip_prefix("internet:") {
                    f.internet = w.trim().into();
                }
            }
        }
    }
    if let Some(t) = lesen(&e.join("REKOPF01.DAT")) {
        if let Some(st) = zeilen(&t).iter().map(|s| s.trim()).find(|s| s.contains('/') && s.chars().any(|c| c.is_ascii_digit())) {
            f.steuernummer = st.to_string();
        }
    }
    f
}

pub fn allgemein(ordner: &Path) -> Allgemein {
    let mut a = Allgemein::default();
    let e = ordner.join("EINSTELL");
    if let Some(t) = lesen(&e.join("Einheite.sta")) {
        let einheiten: Vec<String> = zeilen(&t).into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if !einheiten.is_empty() {
            for x in einheiten {
                if !a.einheiten.contains(&x) {
                    a.einheiten.push(x);
                }
            }
        }
    }
    // ZAHLBE03…10: je Bedingung aus ZBBezeic.DAT zwei Texte (mit Skonto, ohne Skonto).
    let namen: Vec<String> = lesen(&e.join("ZBBezeic.DAT")).map(|t| zeilen(&t)).unwrap_or_default();
    let mut bedingungen = vec![];
    for i in 0..8 {
        let Some(text) = lesen(&e.join(format!("ZAHLBE{:02}.DAT", i + 3))) else { continue };
        let text = text.lines().map(|l| l.trim_end()).filter(|l| !l.chars().all(|c| c == '_' || c == ' ')).collect::<Vec<_>>().join("\n");
        let text = text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        let name = namen.get(i / 2).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or(format!("Bedingung {}", i / 2 + 1));
        let mit_skonto = text.contains("#SK#");
        bedingungen.push(Zahlungsbedingung {
            name: if mit_skonto { format!("{name} mit Skonto") } else { name },
            skonto: if mit_skonto { 2.into() } else { 0.into() },
            skonto_tage: if mit_skonto { 8 } else { 0 },
            ziel_tage: if text.contains("#ZN#") || mit_skonto { 16 } else { 0 },
            text,
        });
    }
    if !bedingungen.is_empty() {
        a.zahlungsbedingung_standard = bedingungen.iter().position(|b| !b.text.contains("#SK#")).unwrap_or(0);
        a.zahlungsbedingungen = bedingungen;
    }
    let titel = ["Zahlungserinnerung", "1. Mahnung", "2. Mahnung", "3. Mahnung"];
    let mut stufen = vec![];
    for (i, t) in titel.iter().enumerate() {
        let Some(text) = lesen(&e.join(format!("MAHNNEU{i}.STA"))) else { continue };
        let l = zeilen(&text);
        let rest = l.iter().skip(4).cloned().collect::<Vec<_>>().join("\n");
        let teile: Vec<&str> = rest.split("*Ende*").collect();
        let bereinigen = |s: &str| s.lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n").trim().to_string();
        stufen.push(Mahnstufe {
            titel: t.to_string(),
            gebuehr: zahl(l.get(1).map(String::as_str).unwrap_or("")),
            frist_tage: l.get(3).and_then(|s| s.trim().parse().ok()).unwrap_or(7),
            einleitung: bereinigen(teile.first().copied().unwrap_or("")),
            schluss: bereinigen(teile.get(1).copied().unwrap_or("")),
        });
    }
    if stufen.len() == 4 {
        a.mahnstufen = stufen;
    }
    a
}

/// Zähler aus GHFakt: (Nummernkreis, nächste Nummer).
pub fn nummernkreise(ordner: &Path) -> Vec<(&'static str, i64)> {
    let e = ordner.join("EINSTELL");
    let wert = |datei: &str| lesen(&e.join(datei)).and_then(|t| t.trim().parse::<i64>().ok());
    let mut out = vec![];
    for (kreis, datei) in [("angebot", "FAKTNUM0.DAT"), ("auftrag", "FAKTNUM1.DAT"), ("lieferschein", "FAKTNUM2.DAT"), ("rechnung", "FAKTNUM3.DAT"), ("gutschrift", "FAKTNUM4.DAT"), ("kunde", "KDSTNUM0.DAT")] {
        if let Some(n) = wert(datei) {
            out.push((kreis, n));
        }
    }
    // AktAuftr.DAT enthält die zuletzt vergebene Auftragsnummer.
    if let Some(n) = wert("AktAuftr.DAT") {
        out.push(("projekt", n + 1));
    }
    out
}

pub fn textbausteine(ordner: &Path) -> Vec<Textbaustein> {
    let e = ordner.join("EINSTELL");
    let namen: Vec<String> = lesen(&e.join("TEXTMUSN.DAT")).map(|t| zeilen(&t)).unwrap_or_default();
    let mut out = vec![];
    for i in 0..100 {
        let Some(t) = lesen(&e.join(format!("TextMust.{i:03}"))) else { continue };
        let text: String = t.split('\n').skip(1).collect::<Vec<_>>().join("\n").replace('•', "");
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let name = namen.get(i).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or(format!("Textmuster {}", i + 1));
        out.push(Textbaustein { kategorie: "nachtext".into(), name, text: super::rtf::text_zu_html(text), ..Default::default() });
    }
    out
}

/// Formelkatalog des Aufmaßes (NFORMEL.STA, Blöcke zu 12 Zeilen). `ma1`…`ma3` werden zu `a`…`c`.
pub fn formeln(ordner: &Path) -> Vec<Formel> {
    let Some(t) = lesen(&ordner.join("EINSTELL").join("NFORMEL.STA")) else { return vec![] };
    let l = zeilen(&t);
    let mut out = vec![];
    for (nummer, block) in l.chunks(12).enumerate() {
        let name = block.first().map(|s| s.trim()).unwrap_or("");
        let ausdruck = block.get(9).map(|s| s.trim()).unwrap_or("");
        if name.is_empty() || ausdruck.is_empty() {
            continue;
        }
        let masse: Vec<&str> = (1..4).map(|i| block.get(i).map(|s| s.trim()).unwrap_or("")).filter(|s| !s.is_empty()).collect();
        let ausdruck = ausdruck.replace("ma1", "a").replace("ma2", "b").replace("ma3", "c").replace(',', ".");
        out.push(Formel { nummer: nummer as i64, name: name.into(), ausdruck, hinweis: masse.join(" / "), ..Default::default() });
    }
    out
}
