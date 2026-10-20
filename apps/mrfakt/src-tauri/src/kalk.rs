//! Kalkulation: Mitarbeiter, Positionskalkulation (Lohn, Material, Gerät, Fremdleistung) und Nachkalkulation.

use crate::belege;
use crate::db::{err, Datensatz, Res};
use crate::einstellungen::{self, Kalkulationssaetze};
use crate::modelle::{Artikel, Beleg, Kalkulation, Mitarbeiter, Position, Zeit};
use crate::rechnen::cent;
use crate::state::AppState;
use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::Serialize;
use std::str::FromStr;
use tauri::State;

#[tauri::command]
pub fn mitarbeiter_liste(state: State<AppState>) -> Res<Vec<Mitarbeiter>> {
    state.mit_db(|db| Mitarbeiter::alle(db, "ORDER BY aktiv DESC, name COLLATE NOCASE", &[]))
}

#[tauri::command]
pub fn mitarbeiter_speichern(state: State<AppState>, mitarbeiter: Mitarbeiter) -> Res<i64> {
    let mut m = mitarbeiter;
    state.mit_db(|db| m.speichern(db))
}

#[tauri::command]
pub fn mitarbeiter_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| Mitarbeiter::loeschen(db, id))
}

fn zuschlag(z: &Kalkulation, s: &Kalkulationssaetze) -> Decimal {
    if !z.zuschlag.trim().is_empty() {
        return Decimal::from_str(&z.zuschlag.trim().replace(',', ".")).unwrap_or_default();
    }
    match z.art.as_str() {
        "L" => s.zuschlag_lohn,
        "M" => s.zuschlag_material,
        "G" => s.zuschlag_geraet,
        "F" => s.zuschlag_fremd,
        _ => s.zuschlag_sonstig,
    }
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct KalkErgebnis {
    pub ep: Decimal,
    pub lohn: Decimal,
    pub minuten: Decimal,
    /// Je Zeile: Kosten und Preis (mit Zuschlag) je Einheit der Position.
    pub zeilen: Vec<(Decimal, Decimal)>,
}

/// Einheitspreis aus den Kalkulationszeilen. Lohn: Menge in Minuten × Stundensatz / 60.
pub fn rechnen(zeilen: &[Kalkulation], s: &Kalkulationssaetze) -> KalkErgebnis {
    let mut e = KalkErgebnis::default();
    for z in zeilen {
        let kosten = if z.art == "L" {
            let satz = if z.preis.is_zero() { s.stundensatz } else { z.preis };
            e.minuten += z.menge;
            z.menge * satz / Decimal::from(60)
        } else {
            z.menge * z.preis
        };
        let preis = kosten * (Decimal::ONE_HUNDRED + zuschlag(z, s)) / Decimal::ONE_HUNDRED;
        if z.art == "L" {
            e.lohn += preis;
        }
        e.ep += preis;
        e.zeilen.push((cent(kosten), cent(preis)));
    }
    e.ep = cent(e.ep);
    e.lohn = cent(e.lohn);
    e
}

#[derive(Debug, Serialize)]
pub struct KalkDaten {
    pub zeilen: Vec<Kalkulation>,
    pub saetze: Kalkulationssaetze,
    pub ergebnis: KalkErgebnis,
    pub gesperrt: bool,
}

#[tauri::command]
pub fn kalkulation_laden(state: State<AppState>, position_id: i64) -> Res<KalkDaten> {
    state.mit_db(|db| {
        let zeilen = Kalkulation::alle(db, "WHERE position_id = ?1 ORDER BY pos", &[&position_id])?;
        let saetze = einstellungen::allgemein(db)?.kalkulation;
        let pos = Position::laden(db, position_id)?;
        let gesperrt = belege::ist_gesperrt(&Beleg::laden(db, pos.beleg_id)?);
        Ok(KalkDaten { ergebnis: rechnen(&zeilen, &saetze), zeilen, saetze, gesperrt })
    })
}

#[tauri::command]
pub fn kalkulation_rechnen(state: State<AppState>, zeilen: Vec<Kalkulation>) -> Res<KalkErgebnis> {
    state.mit_db(|db| Ok(rechnen(&zeilen, &einstellungen::allgemein(db)?.kalkulation)))
}

/// Speichert die Kalkulation einer Position; mit `uebernehmen` werden EP und Lohnanteil gesetzt.
pub fn speichern(db: &Connection, position_id: i64, zeilen: Vec<Kalkulation>, uebernehmen: bool) -> Res<KalkErgebnis> {
    let saetze = einstellungen::allgemein(db)?.kalkulation;
    let e = rechnen(&zeilen, &saetze);
    let tx = db.unchecked_transaction().map_err(err)?;
    tx.execute("DELETE FROM kalkulation WHERE position_id = ?1", [position_id]).map_err(err)?;
    for (i, mut z) in zeilen.into_iter().enumerate() {
        z.id = 0;
        z.position_id = position_id;
        z.pos = i as i64;
        z.speichern(&tx)?;
    }
    tx.commit().map_err(err)?;
    if uebernehmen {
        let pos = Position::laden(db, position_id)?;
        let mut v = belege::voll_laden(db, pos.beleg_id)?;
        if let Some(p) = v.positionen.iter_mut().find(|p| p.id == position_id) {
            p.ep = e.ep;
            p.lohn = e.lohn;
        }
        belege::schreiben(db, &mut v)?;
    }
    Ok(e)
}

#[tauri::command]
pub fn kalkulation_speichern(state: State<AppState>, position_id: i64, zeilen: Vec<Kalkulation>, uebernehmen: bool) -> Res<KalkErgebnis> {
    state.mit_db(|db| speichern(db, position_id, zeilen, uebernehmen))
}

/// Material aus dem Artikelstamm für eine Kalkulationszeile (EK, sonst VK).
#[tauri::command]
pub fn kalkulation_artikel(state: State<AppState>, artikel_id: i64) -> Res<Kalkulation> {
    state.mit_db(|db| {
        let a = Artikel::laden(db, artikel_id)?;
        Ok(Kalkulation {
            art: "M".into(),
            text: a.text.lines().next().unwrap_or("").to_string(),
            menge: Decimal::ONE,
            einheit: a.einheit,
            preis: if a.ek.is_zero() { a.vk1 } else { a.ek },
            artikel_id: Some(a.id),
            ..Default::default()
        })
    })
}

// ------------------------------------------------------------- Nachkalkulation

#[tauri::command]
pub fn zeiten_liste(state: State<AppState>, projekt_id: i64) -> Res<Vec<Zeit>> {
    state.mit_db(|db| Zeit::alle(db, "WHERE projekt_id = ?1 ORDER BY datum DESC, id DESC", &[&projekt_id]))
}

#[tauri::command]
pub fn zeit_speichern(state: State<AppState>, zeit: Zeit) -> Res<i64> {
    let mut z = zeit;
    state.mit_db(|db| {
        if z.datum.is_empty() {
            z.datum = crate::db::heute();
        }
        z.speichern(db)
    })
}

#[tauri::command]
pub fn zeit_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| Zeit::loeschen(db, id))
}

#[derive(Debug, Default, Serialize)]
pub struct Nachkalkulation {
    /// Beleg, gegen den verglichen wird (Schlussrechnung/Rechnung, sonst Auftrag/Angebot).
    pub beleg: String,
    pub erloes: Decimal,
    pub soll_stunden: Decimal,
    pub ist_stunden: Decimal,
    pub lohnkosten: Decimal,
    pub soll_lohn: Decimal,
    pub ergebnis: Decimal,
    pub je_mitarbeiter: Vec<(String, Decimal, Decimal)>,
}

#[tauri::command]
pub fn nachkalkulation(state: State<AppState>, projekt_id: i64) -> Res<Nachkalkulation> {
    state.mit_db(|db| {
        let saetze = einstellungen::allgemein(db)?.kalkulation;
        let mut n = Nachkalkulation::default();
        let rangfolge = ["schluss", "rechnung", "auftrag", "angebot"];
        let mut beleg = None;
        for art in rangfolge {
            if let Some(b) = Beleg::alle(db, "WHERE projekt_id = ?1 AND art = ?2 AND status <> 'storniert' ORDER BY datum DESC LIMIT 1", &[&projekt_id, &art])?.into_iter().next() {
                beleg = Some(b);
                break;
            }
        }
        // Erlös: alle Rechnungen/Schlussrechnungen netto (Abschläge stecken in der Schlussrechnung)
        let erloes: f64 = db
            .query_row(
                "SELECT COALESCE(SUM(CAST(netto AS REAL)), 0) FROM beleg WHERE projekt_id = ?1 AND art IN ('rechnung','schluss') AND status <> 'storniert'",
                [projekt_id],
                |r| r.get(0),
            )
            .map_err(err)?;
        n.erloes = cent(Decimal::try_from(erloes).unwrap_or_default());
        if let Some(b) = &beleg {
            n.beleg = format!("{} {} vom {}", b.titel, b.nummer, belege::datum_de(&b.datum));
            if n.erloes.is_zero() {
                n.erloes = b.netto;
            }
            for p in Position::alle(db, "WHERE beleg_id = ?1 AND art = 'N'", &[&b.id])? {
                let z = Kalkulation::alle(db, "WHERE position_id = ?1", &[&p.id])?;
                let e = rechnen(&z, &saetze);
                n.soll_stunden += e.minuten * p.menge / Decimal::from(60);
                n.soll_lohn += cent(e.lohn * p.menge);
                if z.is_empty() && p.einheit.to_lowercase().starts_with("std") {
                    n.soll_stunden += p.menge;
                }
            }
        }
        let mitarbeiter: std::collections::HashMap<i64, Mitarbeiter> = Mitarbeiter::alle(db, "", &[])?.into_iter().map(|m| (m.id, m)).collect();
        let mut je: std::collections::BTreeMap<String, (Decimal, Decimal)> = Default::default();
        for z in Zeit::alle(db, "WHERE projekt_id = ?1", &[&projekt_id])? {
            let m = z.mitarbeiter_id.and_then(|id| mitarbeiter.get(&id));
            let kosten = m.map(|m| m.kosten).filter(|k| !k.is_zero()).unwrap_or(saetze.stundensatz);
            n.ist_stunden += z.stunden;
            n.lohnkosten += cent(z.stunden * kosten);
            let e = je.entry(m.map(|m| m.name.clone()).unwrap_or("ohne Zuordnung".into())).or_default();
            e.0 += z.stunden;
            e.1 += cent(z.stunden * kosten);
        }
        n.je_mitarbeiter = je.into_iter().map(|(k, (s, c))| (k, s, c)).collect();
        n.soll_stunden = n.soll_stunden.round_dp(2);
        n.ergebnis = n.erloes - n.lohnkosten;
        Ok(n)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep_aus_kalkulation() {
        let s = Kalkulationssaetze { stundensatz: Decimal::from(60), zuschlag_lohn: Decimal::ZERO, zuschlag_material: Decimal::from(20), ..Default::default() };
        let zeilen = vec![
            Kalkulation { art: "L".into(), menge: Decimal::from(12), ..Default::default() },
            Kalkulation { art: "M".into(), menge: Decimal::new(25, 2), preis: Decimal::from(8), ..Default::default() },
            Kalkulation { art: "F".into(), menge: Decimal::ONE, preis: Decimal::from(10), zuschlag: "0".into(), ..Default::default() },
        ];
        let e = rechnen(&zeilen, &s);
        assert_eq!(e.lohn, Decimal::from(12));
        assert_eq!(e.ep, Decimal::new(2440, 2));
        assert_eq!(e.minuten, Decimal::from(12));
    }
}
