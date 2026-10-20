//! Regieberichte (Stundenlohn-/Tagesberichte) und ihre Abrechnung.

use crate::belege::{self, BelegVoll};
use crate::db::{err, Datensatz, Res};
use crate::modelle::{Mitarbeiter, Position, Regie, RegieZeile};
use crate::state::AppState;
use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RegieVoll {
    pub regie: Regie,
    pub zeilen: Vec<RegieZeile>,
}

#[tauri::command]
pub fn regie_liste(state: State<AppState>, projekt_id: i64) -> Res<Vec<RegieVoll>> {
    state.mit_db(|db| {
        let mut out = vec![];
        for regie in Regie::alle(db, "WHERE projekt_id = ?1 ORDER BY datum DESC, id DESC", &[&projekt_id])? {
            let zeilen = RegieZeile::alle(db, "WHERE regie_id = ?1 ORDER BY pos", &[&regie.id])?;
            out.push(RegieVoll { regie, zeilen });
        }
        Ok(out)
    })
}

pub fn speichern(db: &Connection, mut r: RegieVoll) -> Res<i64> {
    if r.regie.datum.is_empty() {
        r.regie.datum = crate::db::heute();
    }
    let tx = db.unchecked_transaction().map_err(err)?;
    let id = r.regie.speichern(&tx)?;
    tx.execute("DELETE FROM regie_zeile WHERE regie_id = ?1", [id]).map_err(err)?;
    for (i, mut z) in r.zeilen.into_iter().enumerate() {
        z.id = 0;
        z.regie_id = id;
        z.pos = i as i64;
        z.speichern(&tx)?;
    }
    tx.commit().map_err(err)?;
    Ok(id)
}

#[tauri::command]
pub fn regie_speichern(state: State<AppState>, regie: RegieVoll) -> Res<i64> {
    state.mit_db(|db| speichern(db, regie))
}

#[tauri::command]
pub fn regie_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| Regie::loeschen(db, id))
}

/// Erstellt aus Regieberichten eine Rechnung: Lohn, Material und Gerät als Positionen,
/// Mengen zu LV-Positionen (Art P) mit Text und Preis der Position aus dem zugehörigen Beleg.
pub fn abrechnen(db: &Connection, projekt_id: i64, ids: &[i64]) -> Res<i64> {
    let mut berichte = Regie::alle(db, "WHERE projekt_id = ?1 ORDER BY datum, id", &[&projekt_id])?;
    berichte.retain(|r| ids.contains(&r.id));
    if berichte.is_empty() {
        return Err("Keine Regieberichte ausgewählt.".into());
    }
    let saetze = crate::einstellungen::allgemein(db)?.kalkulation;
    // Gleiche Leistungen zusammenfassen: (Art, PosNr/Text, Einheit, EP) → (Menge, Artikel, EP, Text)
    type Schluessel = (String, String, String, String);
    type Summe = (Decimal, Option<i64>, Decimal, String);
    let mut summe: BTreeMap<Schluessel, Summe> = BTreeMap::new();
    let mut zeitraum: Vec<String> = vec![];
    for r in &berichte {
        zeitraum.push(r.datum.clone());
        let lv: Vec<Position> = match r.beleg_id {
            Some(b) => Position::alle(db, "WHERE beleg_id = ?1", &[&b])?,
            None => vec![],
        };
        for z in RegieZeile::alle(db, "WHERE regie_id = ?1 ORDER BY pos", &[&r.id])? {
            if z.art == "T" || z.menge.is_zero() {
                continue;
            }
            let (text, einheit, ep) = match z.art.as_str() {
                "P" => match lv.iter().find(|p| p.posnr == z.posnr && !z.posnr.is_empty()) {
                    Some(p) => (p.text.clone(), p.einheit.clone(), p.ep),
                    None => (z.text.clone(), z.einheit.clone(), z.ep),
                },
                "L" => {
                    let m = z.mitarbeiter_id.and_then(|id| Mitarbeiter::laden(db, id).ok());
                    let satz = if !z.ep.is_zero() { z.ep } else { m.as_ref().map(|m| m.stundensatz).filter(|s| !s.is_zero()).unwrap_or(saetze.stundensatz) };
                    let text = match m {
                        Some(m) if !m.qualifikation.is_empty() => format!("Stundenlohnarbeiten {}", m.qualifikation),
                        _ => if z.text.is_empty() { "Stundenlohnarbeiten".into() } else { z.text.clone() },
                    };
                    (text, if z.einheit.is_empty() { "Std.".into() } else { z.einheit.clone() }, satz)
                }
                _ => (z.text.clone(), z.einheit.clone(), z.ep),
            };
            let schluessel = (z.art.clone(), if z.art == "P" { z.posnr.clone() } else { text.clone() }, einheit.clone(), ep.to_string());
            let e = summe.entry(schluessel).or_insert((Decimal::ZERO, z.artikel_id, ep, text));
            e.0 += z.menge;
        }
    }
    let mut beleg = belege::neu(db, projekt_id, "rechnung")?;
    beleg.titel = "Regierechnung".into();
    zeitraum.sort();
    if let (Some(a), Some(b)) = (zeitraum.first(), zeitraum.last()) {
        beleg.leistung = if a == b { belege::datum_de(a) } else { format!("{} bis {}", belege::datum_de(a), belege::datum_de(b)) };
    }
    beleg.vortext = format!("<p>für Arbeiten laut Regiebericht{} vom {}.</p>", if berichte.len() > 1 { "en" } else { "" }, berichte.iter().map(|r| belege::datum_de(&r.datum)).collect::<Vec<_>>().join(", "));
    let mut v = BelegVoll { beleg, ..Default::default() };
    for ((art, schluessel, einheit, _), (menge, artikel_id, ep, text)) in summe {
        v.positionen.push(Position {
            art: "N".into(),
            posnr: if art == "P" { schluessel } else { String::new() },
            text,
            menge,
            einheit,
            ep,
            lohn: if art == "L" { ep } else { Decimal::ZERO },
            artikel_id,
            ..Default::default()
        });
    }
    let id = belege::schreiben(db, &mut v)?;
    for r in &berichte {
        db.execute("UPDATE regie SET abgerechnet_id = ?1 WHERE id = ?2", [id, r.id]).map_err(err)?;
    }
    Ok(id)
}

#[tauri::command]
pub fn regie_abrechnen(state: State<AppState>, projekt_id: i64, ids: Vec<i64>) -> Res<i64> {
    state.mit_db(|db| abrechnen(db, projekt_id, &ids))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modelle::{Kunde, Projekt};

    #[test]
    fn regie_wird_rechnung() {
        let db = crate::db::test_db();
        let k = Kunde { name1: "Test".into(), ..Default::default() }.speichern(&db).unwrap();
        let p = Projekt { kunde_id: k, nummer: "1".into(), ..Default::default() }.speichern(&db).unwrap();
        let m = Mitarbeiter { name: "Matt".into(), qualifikation: "Facharbeiter".into(), stundensatz: Decimal::from(50), aktiv: true, ..Default::default() }.speichern(&db).unwrap();
        let mut ids = vec![];
        for tag in ["2024-06-01", "2024-06-02"] {
            ids.push(speichern(&db, RegieVoll {
                regie: Regie { projekt_id: p, datum: tag.into(), ..Default::default() },
                zeilen: vec![
                    RegieZeile { art: "T".into(), text: "Fenster gestrichen".into(), ..Default::default() },
                    RegieZeile { art: "L".into(), menge: Decimal::from(4), mitarbeiter_id: Some(m), ..Default::default() },
                    RegieZeile { art: "M".into(), text: "Lack".into(), menge: Decimal::ONE, einheit: "l".into(), ep: Decimal::from(12), ..Default::default() },
                ],
            }).unwrap());
        }
        let r = abrechnen(&db, p, &ids).unwrap();
        let v = belege::voll_laden(&db, r).unwrap();
        assert_eq!(v.positionen.len(), 2);
        assert_eq!(v.summen.netto, Decimal::from(424));
        assert_eq!(v.beleg.leistung, "01.06.2024 bis 02.06.2024");
        assert_eq!(Regie::laden(&db, ids[0]).unwrap().abgerechnet_id, Some(r));
    }
}
