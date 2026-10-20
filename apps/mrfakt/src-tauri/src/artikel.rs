//! Artikelstamm, Lager und DATANORM-Import.

use crate::db::{self, err, Datensatz, Res};
use crate::modelle::{Artikel, Beleg, LagerBewegung, Position};
use crate::state::AppState;
use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;
use tauri::State;

#[tauri::command]
pub fn artikel_liste(state: State<AppState>, suche: String, nur_lager: bool) -> Res<Vec<Artikel>> {
    state.mit_db(|db| {
        let mut params = vec![];
        let bed = crate::kunden::suchbedingung(&suche, &["nummer", "suchtext", "text", "lieferant", "lief_nr", "ean", "warengruppe"], &mut params);
        let lager = if nur_lager { " AND lager_aktiv = 1" } else { "" };
        let p: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        Artikel::alle(db, &format!("WHERE {bed}{lager} ORDER BY suchtext COLLATE NOCASE, nummer LIMIT 3000"), &p)
    })
}

#[tauri::command]
pub fn artikel_speichern(state: State<AppState>, mut artikel: Artikel) -> Res<i64> {
    state.mit_db(|db| {
        if artikel.suchtext.trim().is_empty() {
            artikel.suchtext = artikel.text.lines().next().unwrap_or("").chars().take(40).collect();
        }
        if artikel.quelle.is_empty() {
            artikel.quelle = "manuell".into();
        }
        if artikel.preiseinheit.is_zero() {
            artikel.preiseinheit = Decimal::ONE;
        }
        artikel.speichern(db)
    })
}

#[tauri::command]
pub fn artikel_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| Artikel::loeschen(db, id))
}

pub fn buchen(db: &Connection, artikel_id: i64, menge: Decimal, text: &str, beleg_id: Option<i64>) -> Res<()> {
    let mut a = Artikel::laden(db, artikel_id)?;
    a.bestand += menge;
    a.speichern(db)?;
    LagerBewegung { artikel_id, datum: db::heute(), menge, text: text.into(), beleg_id, ..Default::default() }.speichern(db)?;
    Ok(())
}

#[tauri::command]
pub fn lager_buchen(state: State<AppState>, artikel_id: i64, menge: Decimal, text: String) -> Res<()> {
    state.mit_db(|db| buchen(db, artikel_id, menge, &text, None))
}

#[tauri::command]
pub fn lager_bewegungen(state: State<AppState>, artikel_id: i64) -> Res<Vec<LagerBewegung>> {
    state.mit_db(|db| LagerBewegung::alle(db, "WHERE artikel_id = ?1 ORDER BY datum DESC, id DESC LIMIT 500", &[&artikel_id]))
}

/// Lagerartikel unter Mindestbestand.
#[tauri::command]
pub fn bestellvorschlag(state: State<AppState>) -> Res<Vec<Artikel>> {
    state.mit_db(|db| {
        let alle = Artikel::alle(db, "WHERE lager_aktiv = 1 ORDER BY lieferant, suchtext", &[])?;
        Ok(alle.into_iter().filter(|a| a.bestand < a.mindestbestand).collect())
    })
}

/// Bucht die Artikel eines Lieferscheins (oder einer Rechnung ohne Lieferschein) vom Lager ab – einmalig.
pub fn beleg_abbuchen(db: &Connection, beleg: &Beleg) -> Res<()> {
    let schon: i64 = db.query_row("SELECT COUNT(*) FROM lager_bewegung WHERE beleg_id = ?1", [beleg.id], |r| r.get(0)).map_err(err)?;
    if schon > 0 {
        return Ok(());
    }
    for p in Position::alle(db, "WHERE beleg_id = ?1 AND artikel_id IS NOT NULL AND art = 'N'", &[&beleg.id])? {
        let Some(aid) = p.artikel_id else { continue };
        if let Ok(a) = Artikel::laden(db, aid) {
            if a.lager_aktiv && !p.menge.is_zero() {
                buchen(db, aid, -p.menge, &format!("{} {}", beleg.titel, beleg.nummer), Some(beleg.id))?;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- DATANORM

#[derive(Debug, Default, Serialize)]
pub struct DatanormErgebnis {
    pub neu: i64,
    pub geaendert: i64,
    pub preise: i64,
    pub texte: i64,
    pub version: String,
}

fn feld<'a>(f: &'a [&str], i: usize) -> &'a str {
    f.get(i).map(|s| s.trim()).unwrap_or("")
}

/// Preis in Cent (ohne Komma) → Euro.
fn preis(s: &str) -> Decimal {
    let s = s.trim().replace(',', ".");
    if s.contains('.') {
        return Decimal::from_str(&s).unwrap_or_default();
    }
    Decimal::from_str(&s).map(|d| d / Decimal::ONE_HUNDRED).unwrap_or_default()
}

fn preiseinheit(code: &str) -> Decimal {
    match code.trim() {
        "1" => Decimal::from(10),
        "2" => Decimal::from(100),
        "3" => Decimal::from(1000),
        _ => Decimal::ONE,
    }
}

/// DATANORM 4/5 (Semikolon-getrennt): A = Artikel, B = Zusatzdaten, P = Preise, T = Langtexte.
pub fn datanorm(db: &Connection, text: &str, lieferant: &str, aufschlag: Decimal) -> Res<DatanormErgebnis> {
    let mut e = DatanormErgebnis::default();
    let mut artikel: HashMap<String, Artikel> = HashMap::new();
    let mut langtexte: HashMap<String, Vec<(i64, String)>> = HashMap::new();
    let mut langtext_von: HashMap<String, String> = HashMap::new();
    for zeile in text.lines() {
        let f: Vec<&str> = zeile.split(';').collect();
        match feld(&f, 0) {
            "V" => e.version = format!("DATANORM {}", feld(&f, 1).chars().filter(|c| c.is_ascii_digit()).collect::<String>()),
            "A" => {
                let nr = feld(&f, 2).to_string();
                if nr.is_empty() {
                    continue;
                }
                let a = artikel.entry(nr.clone()).or_default();
                a.nummer = nr.clone();
                a.lief_nr = nr.clone();
                let kurz = [feld(&f, 4), feld(&f, 5)].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
                a.text = kurz.clone();
                a.suchtext = kurz.chars().take(40).collect();
                a.preiseinheit = preiseinheit(feld(&f, 7));
                a.einheit = feld(&f, 8).to_string();
                let p = preis(feld(&f, 9)) / a.preiseinheit;
                if feld(&f, 6) == "2" { a.ek = p } else { a.vk1 = p }
                a.rabattgruppe = feld(&f, 10).to_string();
                a.warengruppe = feld(&f, 11).to_string();
                let lt = feld(&f, 12);
                if !lt.is_empty() {
                    langtext_von.insert(nr, lt.to_string());
                }
            }
            "B" => {
                let nr = feld(&f, 2);
                if let Some(a) = artikel.get_mut(nr) {
                    if !feld(&f, 3).is_empty() {
                        a.suchtext = feld(&f, 3).to_string();
                    }
                    let ean = f.iter().map(|s| s.trim()).find(|s| (s.len() == 13 || s.len() == 8) && s.chars().all(|c| c.is_ascii_digit()));
                    if let Some(ean) = ean {
                        a.ean = ean.to_string();
                    }
                }
            }
            "P" => {
                // P;A;Artikelnr;Preiskennz.;Preis;… (bis zu drei Artikel je Satz)
                let mut i = 2;
                while i + 2 < f.len() {
                    let nr = feld(&f, i);
                    if nr.is_empty() {
                        break;
                    }
                    let a = artikel.entry(nr.to_string()).or_insert_with(|| Artikel { nummer: nr.into(), lief_nr: nr.into(), ..Default::default() });
                    let p = preis(feld(&f, i + 2)) / if a.preiseinheit.is_zero() { Decimal::ONE } else { a.preiseinheit };
                    if feld(&f, i + 1) == "2" { a.ek = p } else { a.vk1 = p }
                    e.preise += 1;
                    i += 9;
                }
            }
            "T" => {
                // T;N;Textschlüssel;…;Zeilennummer;Text
                let schluessel = feld(&f, 2).to_string();
                let nr = f.iter().skip(3).find_map(|s| s.trim().parse::<i64>().ok()).unwrap_or(0);
                let text = f.iter().rev().find(|s| !s.trim().is_empty() && s.trim().parse::<i64>().is_err()).map(|s| s.trim().to_string()).unwrap_or_default();
                if !schluessel.is_empty() && !text.is_empty() {
                    langtexte.entry(schluessel).or_default().push((nr, text));
                    e.texte += 1;
                }
            }
            _ => {}
        }
    }
    let lieferant_id: Option<i64> = db
        .query_row("SELECT id FROM kunde WHERE typ = 'Lieferant' AND (suchname = ?1 OR name1 = ?1) LIMIT 1", [lieferant], |r| r.get(0))
        .ok();
    let tx = db.unchecked_transaction().map_err(err)?;
    for (nr, mut a) in artikel {
        if let Some(lt) = langtext_von.get(&nr).and_then(|k| langtexte.get_mut(k)) {
            lt.sort();
            let lang = lt.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>().join("\n");
            if !lang.is_empty() {
                a.text = format!("{}\n{}", a.text, lang);
            }
        }
        if a.vk1.is_zero() && !a.ek.is_zero() {
            a.vk1 = crate::rechnen::cent(a.ek * (Decimal::ONE_HUNDRED + aufschlag) / Decimal::ONE_HUNDRED);
        }
        a.lieferant = lieferant.into();
        a.lieferant_id = lieferant_id;
        a.quelle = "datanorm".into();
        a.datum_preis = db::heute();
        let vorhanden = Artikel::alle(&tx, "WHERE lief_nr = ?1 AND lieferant = ?2 LIMIT 1", &[&nr, &lieferant.to_string()])?;
        match vorhanden.into_iter().next() {
            Some(mut alt) => {
                if !a.text.is_empty() { alt.text = a.text; }
                if !a.suchtext.is_empty() { alt.suchtext = a.suchtext; }
                if !a.einheit.is_empty() { alt.einheit = a.einheit; }
                if !a.ek.is_zero() { alt.ek = a.ek; }
                if !a.vk1.is_zero() { alt.vk1 = a.vk1; }
                if !a.ean.is_empty() { alt.ean = a.ean; }
                alt.datum_preis = a.datum_preis;
                alt.speichern(&tx)?;
                e.geaendert += 1;
            }
            None => {
                if a.text.is_empty() {
                    continue;
                }
                a.speichern(&tx)?;
                e.neu += 1;
            }
        }
    }
    tx.commit().map_err(err)?;
    Ok(e)
}

#[tauri::command]
pub fn datanorm_import(state: State<AppState>, pfade: Vec<String>, lieferant: String, aufschlag: Decimal) -> Res<DatanormErgebnis> {
    let mut text = String::new();
    for p in &pfade {
        let bytes = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
        let t = match String::from_utf8(bytes.clone()) {
            Ok(t) => t,
            Err(_) => {
                // DATANORM ist meist Codepage 437/850 oder Windows-1252
                let (t, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
                t.into_owned()
            }
        };
        text.push_str(&t);
        text.push('\n');
    }
    state.mit_db(|db| datanorm(db, &text, lieferant.trim(), aufschlag))
}

/// Etiketten für Artikel (Bezeichnung, Nummer, Preis, Lagerplatz) als PDF.
#[tauri::command]
pub fn artikel_etiketten(state: State<AppState>, ids: Vec<i64>, pfad: String, spalten: u32, hoehe: f64) -> Res<()> {
    let etiketten = state.mit_db(|db| {
        let mut out = vec![];
        for id in ids {
            let a = Artikel::laden(db, id)?;
            let mut e = vec![a.text.lines().next().unwrap_or("").to_string()];
            if !a.nummer.is_empty() { e.push(format!("Art.-Nr. {}", a.nummer)); }
            if !a.vk1.is_zero() { e.push(format!("{} € / {}", crate::pdf::betrag(a.vk1), a.einheit)); }
            if !a.lagerplatz.is_empty() { e.push(format!("Lagerplatz {}", a.lagerplatz)); }
            if !a.ean.is_empty() { e.push(format!("EAN {}", a.ean)); }
            out.push(e);
        }
        Ok(out)
    })?;
    let pdf = crate::pdf::druck::etiketten_pdf(etiketten, spalten, hoehe)?;
    std::fs::write(&pfad, pdf).map_err(err)
}

/// Adressetiketten für Kunden.
#[tauri::command]
pub fn kunden_etiketten(state: State<AppState>, ids: Vec<i64>, pfad: String, spalten: u32, hoehe: f64) -> Res<()> {
    let etiketten = state.mit_db(|db| {
        let mut out = vec![];
        for id in ids {
            let k = crate::modelle::Kunde::laden(db, id)?;
            out.push(k.anschrift().lines().map(|s| s.to_string()).collect());
        }
        Ok(out)
    })?;
    let pdf = crate::pdf::druck::etiketten_pdf(etiketten, spalten, hoehe)?;
    std::fs::write(&pfad, pdf).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DN4: &str = "V 050101Großhandel Muster;04;EUR\nA;N;4711;00;Dispersionsfarbe weiß;Caparol Indeko;1;0;l;0000001250;R1;WG1;LT1\nB;N;4711;DISPERSION;;;;;;;;4012345678901\nT;N;LT1;;;1;Hochdeckend, scheuerbeständig\nT;N;LT1;;;2;Klasse 2\nP;A;4711;2;0000000890;;;;;;;\n";

    #[test]
    fn datanorm_einlesen_und_aktualisieren() {
        let db = db::test_db();
        let e = datanorm(&db, DN4, "Muster", Decimal::from(30)).unwrap();
        assert_eq!((e.neu, e.preise, e.texte), (1, 1, 2));
        let a = &Artikel::alle(&db, "", &[]).unwrap()[0];
        assert_eq!(a.vk1, Decimal::new(1250, 2));
        assert_eq!(a.ek, Decimal::new(890, 2));
        assert_eq!(a.ean, "4012345678901");
        assert_eq!(a.suchtext, "DISPERSION");
        assert!(a.text.ends_with("Hochdeckend, scheuerbeständig\nKlasse 2"));
        let e = datanorm(&db, "P;A;4711;2;0000000990;;;;;;;\n", "Muster", Decimal::from(30)).unwrap();
        assert_eq!(e.geaendert, 1);
        assert_eq!(Artikel::alle(&db, "", &[]).unwrap()[0].ek, Decimal::new(990, 2));
    }

    #[test]
    fn lager_buchung() {
        let db = db::test_db();
        let id = Artikel { text: "Lack".into(), lager_aktiv: true, bestand: Decimal::from(5), mindestbestand: Decimal::from(3), ..Default::default() }.speichern(&db).unwrap();
        buchen(&db, id, Decimal::from(-3), "Entnahme", None).unwrap();
        assert_eq!(Artikel::laden(&db, id).unwrap().bestand, Decimal::from(2));
        assert_eq!(LagerBewegung::alle(&db, "", &[]).unwrap().len(), 1);
    }
}
