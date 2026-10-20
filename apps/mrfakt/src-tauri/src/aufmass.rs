//! Aufmaß: Maßzeilen je Position, Formelkatalog, Übernahme der Mengen in den Beleg.

use crate::ausdruck;
use crate::belege;
use crate::db::{err, Datensatz, Res};
use crate::modelle::{Aufmass, Beleg, Formel, Position};
use crate::state::AppState;
use rusqlite::Connection;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct AufmassDaten {
    pub zeilen: Vec<Aufmass>,
    pub formeln: Vec<Formel>,
    pub gesperrt: bool,
}

/// Ergebnis einer Maßzeile: Anzahl × Formel(Maß 1–3); Formel 0 = Produkt der angegebenen Maße.
pub fn ergebnis(z: &Aufmass, formeln: &HashMap<i64, Formel>) -> Result<Decimal, String> {
    if z.art != "M" {
        return Ok(Decimal::ZERO);
    }
    let masse = [ausdruck::feld(&z.m1)?, ausdruck::feld(&z.m2)?, ausdruck::feld(&z.m3)?];
    let wert = match formeln.get(&z.formel) {
        Some(f) if z.formel != 0 && !f.ausdruck.trim().is_empty() => {
            let vars = [('a', masse[0].unwrap_or_default()), ('b', masse[1].unwrap_or_default()), ('c', masse[2].unwrap_or_default())];
            ausdruck::auswerten(&f.ausdruck, &vars)?
        }
        _ => {
            let vorhanden: Vec<Decimal> = masse.iter().flatten().copied().collect();
            if vorhanden.is_empty() { Decimal::ZERO } else { vorhanden.iter().product() }
        }
    };
    let anzahl = if z.anzahl.is_zero() { Decimal::ONE } else { z.anzahl };
    let e = (anzahl * wert).round_dp_with_strategy(3, RoundingStrategy::MidpointAwayFromZero);
    Ok(if z.abzug { -e.abs() } else { e })
}

/// Summe der Maßzeilen je Positionsnummer.
pub fn summen(zeilen: &[Aufmass]) -> HashMap<String, Decimal> {
    let mut m: HashMap<String, Decimal> = HashMap::new();
    for z in zeilen.iter().filter(|z| z.art == "M") {
        *m.entry(z.posnr.clone()).or_default() += z.ergebnis;
    }
    m
}

pub fn speichern(db: &Connection, beleg_id: i64, mut zeilen: Vec<Aufmass>, uebernehmen: bool) -> Res<Vec<String>> {
    let beleg = Beleg::laden(db, beleg_id)?;
    let formeln: HashMap<i64, Formel> = Formel::alle(db, "", &[])?.into_iter().map(|f| (f.nummer, f)).collect();
    let mut fehler = vec![];
    for (i, z) in zeilen.iter_mut().enumerate() {
        z.id = 0;
        z.beleg_id = beleg_id;
        z.pos = i as i64;
        match ergebnis(z, &formeln) {
            Ok(e) => z.ergebnis = e,
            Err(e) => fehler.push(format!("Zeile {} ({}): {e}", i + 1, z.text)),
        }
    }
    let tx = db.unchecked_transaction().map_err(err)?;
    tx.execute("DELETE FROM aufmass WHERE beleg_id = ?1", [beleg_id]).map_err(err)?;
    for z in zeilen.iter_mut() {
        z.speichern(&tx)?;
    }
    tx.commit().map_err(err)?;
    if uebernehmen {
        if belege::ist_gesperrt(&beleg) {
            return Err("Der Beleg ist festgeschrieben – das Aufmaß wurde gespeichert, die Mengen aber nicht übernommen.".into());
        }
        let s = summen(&zeilen);
        let mut v = belege::voll_laden(db, beleg_id)?;
        for p in v.positionen.iter_mut() {
            if let Some(m) = s.get(&p.posnr).filter(|_| !p.posnr.is_empty()) {
                p.menge = m.round_dp_with_strategy(3, RoundingStrategy::MidpointAwayFromZero);
                p.menge_aus_aufmass = true;
            }
        }
        belege::schreiben(db, &mut v)?;
    }
    Ok(fehler)
}

#[tauri::command]
pub fn aufmass_laden(state: State<AppState>, beleg_id: i64) -> Res<AufmassDaten> {
    state.mit_db(|db| {
        let beleg = Beleg::laden(db, beleg_id)?;
        Ok(AufmassDaten {
            zeilen: Aufmass::alle(db, "WHERE beleg_id = ?1 ORDER BY pos", &[&beleg_id])?,
            formeln: Formel::alle(db, "ORDER BY nummer", &[])?,
            gesperrt: belege::ist_gesperrt(&beleg),
        })
    })
}

/// Speichert das Aufmaß; mit `uebernehmen` werden die Summen als Mengen in die Positionen geschrieben.
/// Gibt Hinweise zu Zeilen zurück, deren Maße nicht berechnet werden konnten.
#[tauri::command]
pub fn aufmass_speichern(state: State<AppState>, beleg_id: i64, zeilen: Vec<Aufmass>, uebernehmen: bool) -> Res<Vec<String>> {
    state.mit_db(|db| speichern(db, beleg_id, zeilen, uebernehmen))
}

/// Ergebnis einer Zeile berechnen (Vorschau beim Tippen).
#[tauri::command]
pub fn aufmass_rechnen(state: State<AppState>, zeile: Aufmass) -> Res<Decimal> {
    state.mit_db(|db| {
        let formeln: HashMap<i64, Formel> = Formel::alle(db, "", &[])?.into_iter().map(|f| (f.nummer, f)).collect();
        ergebnis(&zeile, &formeln)
    })
}

#[tauri::command]
pub fn formeln_liste(state: State<AppState>) -> Res<Vec<Formel>> {
    state.mit_db(|db| Formel::alle(db, "ORDER BY nummer", &[]))
}

#[tauri::command]
pub fn formeln_speichern(state: State<AppState>, formeln: Vec<Formel>) -> Res<()> {
    state.mit_db(|db| {
        for f in &formeln {
            if f.nummer != 0 {
                ausdruck::auswerten(&f.ausdruck, &[('a', Decimal::ONE), ('b', Decimal::ONE), ('c', Decimal::ONE)])
                    .map_err(|e| format!("Formel „{}“: {e}", f.name))?;
            }
        }
        let tx = db.unchecked_transaction().map_err(err)?;
        tx.execute("DELETE FROM formel", []).map_err(err)?;
        for mut f in formeln {
            f.id = 0;
            f.speichern(&tx)?;
        }
        tx.commit().map_err(err)
    })
}

/// Positionen eines Belegs für die Zuordnung im Aufmaß.
#[tauri::command]
pub fn aufmass_positionen(state: State<AppState>, beleg_id: i64) -> Res<Vec<Position>> {
    state.mit_db(|db| Position::alle(db, "WHERE beleg_id = ?1 AND art IN ('N','A','B') ORDER BY pos", &[&beleg_id]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn maszeilen() {
        let d = |s: &str| Decimal::from_str(s).unwrap();
        let mut formeln = HashMap::new();
        formeln.insert(2, Formel { nummer: 2, name: "Trapez".into(), ausdruck: "(a+b)/2*c".into(), ..Default::default() });
        let z = Aufmass { art: "M".into(), anzahl: d("1"), m1: "9,85".into(), m2: "6,02".into(), ..Default::default() };
        assert_eq!(ergebnis(&z, &formeln).unwrap(), d("59.297"));
        let z = Aufmass { art: "M".into(), anzahl: d("2"), formel: 2, m1: "2".into(), m2: "4".into(), m3: "3".into(), ..Default::default() };
        assert_eq!(ergebnis(&z, &formeln).unwrap(), d("18"));
        let z = Aufmass { art: "M".into(), anzahl: d("1"), m1: "12+12+9".into(), abzug: true, ..Default::default() };
        assert_eq!(ergebnis(&z, &formeln).unwrap(), d("-33"));
        let z = Aufmass { art: "M".into(), m1: "abc".into(), ..Default::default() };
        assert!(ergebnis(&z, &formeln).is_err());
    }
}
