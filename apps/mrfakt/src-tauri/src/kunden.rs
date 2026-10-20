//! Kunden, Lieferanten und ihre Projekte.

use crate::db::{self, err, Datensatz, Res};
use crate::modelle::{Kunde, Plz, Projekt};
use crate::state::AppState;
use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct KundeZeile {
    pub id: i64,
    pub nummer: String,
    pub typ: String,
    pub suchname: String,
    pub name: String,
    pub strasse: String,
    pub ort: String,
    pub telefon: String,
    pub offen: Decimal,
    pub letzter_beleg: String,
}

/// Bedingung für eine Suche über mehrere Spalten: jedes Wort muss in einer der Spalten vorkommen.
pub fn suchbedingung(suche: &str, spalten: &[&str], params: &mut Vec<String>) -> String {
    let mut teile = vec![];
    for wort in suche.split_whitespace() {
        params.push(format!("%{}%", wort.replace('%', "")));
        let n = params.len();
        let oder: Vec<String> = spalten.iter().map(|s| format!("{s} LIKE ?{n}")).collect();
        teile.push(format!("({})", oder.join(" OR ")));
    }
    if teile.is_empty() { "1".into() } else { teile.join(" AND ") }
}

#[tauri::command]
pub fn kunden_liste(state: State<AppState>, suche: String, typ: String) -> Res<Vec<KundeZeile>> {
    state.mit_db(|db| kunden_suchen(db, &suche, &typ, 2000))
}

pub fn kunden_suchen(db: &Connection, suche: &str, typ: &str, limit: i64) -> Res<Vec<KundeZeile>> {
    let mut params: Vec<String> = vec![];
    let bed = suchbedingung(
        suche,
        &["k.suchname", "k.name1", "k.name2", "k.anrede", "k.ort", "k.ortsteil", "k.strasse", "k.nummer", "k.telefon", "k.mobil", "k.email", "k.plz"],
        &mut params,
    );
    let typ_bed = if typ.is_empty() {
        String::new()
    } else {
        params.push(typ.to_string());
        format!(" AND k.typ = ?{}", params.len())
    };
    let sql = format!(
        "SELECT k.id, k.nummer, k.typ, k.suchname, k.name1, k.name2, k.strasse, k.plz, k.ort, k.telefon, k.mobil,
            COALESCE((SELECT SUM(CAST(b.zahlbetrag AS REAL) - CAST(b.bezahlt AS REAL)) FROM beleg b
              WHERE b.kunde_id = k.id AND b.art IN ('abschlag','schluss','rechnung') AND b.status IN ('festgeschrieben','gedruckt')), 0),
            COALESCE((SELECT MAX(b.datum) FROM beleg b WHERE b.kunde_id = k.id), '')
         FROM kunde k WHERE {bed}{typ_bed}
         ORDER BY CASE WHEN k.suchname = '' THEN k.name1 ELSE k.suchname END COLLATE NOCASE LIMIT {limit}"
    );
    let mut stmt = db.prepare(&sql).map_err(err)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |r| {
            let name1: String = r.get(4)?;
            let name2: String = r.get(5)?;
            let tel: String = r.get(9)?;
            let mobil: String = r.get(10)?;
            let offen: f64 = r.get(11)?;
            Ok(KundeZeile {
                id: r.get(0)?,
                nummer: r.get(1)?,
                typ: r.get(2)?,
                suchname: r.get(3)?,
                name: [name1.trim(), name2.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(", "),
                strasse: r.get(6)?,
                ort: format!("{} {}", r.get::<_, String>(7)?, r.get::<_, String>(8)?).trim().to_string(),
                telefon: if tel.is_empty() { mobil } else { tel },
                offen: crate::rechnen::cent(Decimal::try_from(offen).unwrap_or_default()),
                letzter_beleg: r.get(12)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

#[derive(Debug, Serialize)]
pub struct ProjektZeile {
    #[serde(flatten)]
    pub projekt: Projekt,
    pub belege: i64,
    pub letzter: String,
    pub summe: Decimal,
}

#[derive(Debug, Serialize)]
pub struct KundeDetail {
    pub kunde: Kunde,
    pub projekte: Vec<ProjektZeile>,
    pub umsatz: Vec<(String, Decimal)>,
    pub offen: Decimal,
    pub briefe: i64,
}

pub fn projekte_von(db: &Connection, kunde_id: i64) -> Res<Vec<ProjektZeile>> {
    let projekte = Projekt::alle(db, "WHERE kunde_id = ?1 ORDER BY datum DESC, CAST(nummer AS INTEGER) DESC", &[&kunde_id])?;
    let mut out = vec![];
    let mut stmt = db
        .prepare_cached(
            "SELECT COUNT(*), COALESCE(MAX(datum), ''),
                COALESCE(SUM(CASE WHEN art IN ('abschlag','schluss','rechnung') AND status <> 'storniert' THEN CAST(netto AS REAL) ELSE 0 END), 0)
             FROM beleg WHERE projekt_id = ?1",
        )
        .map_err(err)?;
    for p in projekte {
        let (belege, letzter, summe): (i64, String, f64) = stmt.query_row([p.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(err)?;
        out.push(ProjektZeile { belege, letzter, summe: crate::rechnen::cent(Decimal::try_from(summe).unwrap_or_default()), projekt: p });
    }
    // Neueste Aktivität zuerst
    out.sort_by(|a, b| {
        let neu = |x: &ProjektZeile| x.letzter.clone().max(x.projekt.datum.clone());
        neu(b).cmp(&neu(a))
    });
    Ok(out)
}

#[tauri::command]
pub fn kunde_laden(state: State<AppState>, id: i64) -> Res<KundeDetail> {
    state.mit_db(|db| {
        let kunde = Kunde::laden(db, id)?;
        db.execute("UPDATE kunde SET zugriff = ?1 WHERE id = ?2", rusqlite::params![db::jetzt(), id]).map_err(err)?;
        let projekte = projekte_von(db, id)?;
        let mut stmt = db
            .prepare(
                "SELECT substr(datum, 1, 4) AS jahr, SUM(CASE WHEN art = 'gutschrift' THEN -CAST(netto AS REAL) ELSE CAST(netto AS REAL) END)
                 FROM beleg WHERE kunde_id = ?1 AND art IN ('rechnung','schluss','gutschrift') AND status <> 'storniert' AND datum <> ''
                 GROUP BY jahr ORDER BY jahr DESC",
            )
            .map_err(err)?;
        let umsatz = stmt
            .query_map([id], |r| Ok((r.get::<_, String>(0)?, crate::rechnen::cent(Decimal::try_from(r.get::<_, f64>(1)?).unwrap_or_default()))))
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        let offen: f64 = db
            .query_row(
                "SELECT COALESCE(SUM(CAST(zahlbetrag AS REAL) - CAST(bezahlt AS REAL)), 0) FROM beleg
                 WHERE kunde_id = ?1 AND art IN ('abschlag','schluss','rechnung') AND status IN ('festgeschrieben','gedruckt')",
                [id],
                |r| r.get(0),
            )
            .map_err(err)?;
        let briefe = db.query_row("SELECT COUNT(*) FROM brief WHERE kunde_id = ?1", [id], |r| r.get(0)).map_err(err)?;
        Ok(KundeDetail { kunde, projekte, umsatz, offen: crate::rechnen::cent(Decimal::try_from(offen).unwrap_or_default()), briefe })
    })
}

#[tauri::command]
pub fn kunde_speichern(state: State<AppState>, mut kunde: Kunde) -> Res<i64> {
    state.mit_db(|db| {
        let jetzt = db::jetzt();
        if kunde.id == 0 {
            kunde.angelegt = jetzt.clone();
            if kunde.nummer.trim().is_empty() {
                kunde.nummer = db::naechste_nummer(db, "kunde")?;
            }
        }
        if kunde.suchname.trim().is_empty() {
            kunde.suchname = kunde.name1.split_whitespace().last().unwrap_or("").to_string();
        }
        kunde.geaendert = jetzt;
        kunde.speichern(db)
    })
}

#[tauri::command]
pub fn kunde_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| {
        let belege: i64 = db.query_row("SELECT COUNT(*) FROM beleg WHERE kunde_id = ?1", [id], |r| r.get(0)).map_err(err)?;
        if belege > 0 {
            return Err(format!("Der Kunde hat {belege} Belege und kann nicht gelöscht werden."));
        }
        Kunde::loeschen(db, id)
    })
}

#[tauri::command]
pub fn plz_suche(state: State<AppState>, plz: String, ort: String) -> Res<Vec<Plz>> {
    state.mit_db(|db| {
        if !plz.trim().is_empty() {
            Plz::alle(db, "WHERE plz LIKE ?1 ORDER BY plz, ort LIMIT 30", &[&format!("{}%", plz.trim())])
        } else if ort.trim().len() >= 2 {
            Plz::alle(db, "WHERE ort LIKE ?1 ORDER BY ort, plz LIMIT 30", &[&format!("{}%", ort.trim())])
        } else {
            Ok(vec![])
        }
    })
}

#[tauri::command]
pub fn projekt_speichern(state: State<AppState>, projekt: Projekt) -> Res<i64> {
    let mut p = projekt;
    state.mit_db(|db| {
        if p.id == 0 {
            if p.nummer.trim().is_empty() {
                p.nummer = db::naechste_nummer(db, "projekt")?;
            }
            if p.datum.is_empty() {
                p.datum = db::heute();
            }
            if p.status.is_empty() {
                p.status = "offen".into();
            }
        }
        p.speichern(db)
    })
}

#[tauri::command]
pub fn projekt_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| {
        let fest: i64 = db
            .query_row("SELECT COUNT(*) FROM beleg WHERE projekt_id = ?1 AND status = 'festgeschrieben'", [id], |r| r.get(0))
            .map_err(err)?;
        if fest > 0 {
            return Err("Das Projekt enthält festgeschriebene Rechnungen und kann nicht gelöscht werden.".into());
        }
        Projekt::loeschen(db, id)
    })
}

#[derive(Debug, Serialize)]
pub struct ProjektSuche {
    pub id: i64,
    pub kunde_id: i64,
    pub nummer: String,
    pub bezeichnung: String,
    pub kunde: String,
    pub datum: String,
}

/// Projekte über alle Kunden suchen (Nummer, Bezeichnung, Kunde).
#[tauri::command]
pub fn projekte_suchen(state: State<AppState>, suche: String) -> Res<Vec<ProjektSuche>> {
    state.mit_db(|db| {
        let mut params = vec![];
        let bed = suchbedingung(&suche, &["p.nummer", "p.bezeichnung", "p.baustelle", "k.suchname", "k.name1", "k.ort"], &mut params);
        let sql = format!(
            "SELECT p.id, p.kunde_id, p.nummer, p.bezeichnung, CASE WHEN k.name1 = '' THEN k.suchname ELSE k.name1 END, p.datum
             FROM projekt p JOIN kunde k ON k.id = p.kunde_id WHERE {bed} ORDER BY CAST(p.nummer AS INTEGER) DESC LIMIT 200"
        );
        let mut stmt = db.prepare(&sql).map_err(err)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |r| {
                Ok(ProjektSuche { id: r.get(0)?, kunde_id: r.get(1)?, nummer: r.get(2)?, bezeichnung: r.get(3)?, kunde: r.get(4)?, datum: r.get(5)? })
            })
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suche_ueber_mehrere_woerter() {
        let db = db::test_db();
        for (s, n, o) in [("Glinka", "Manfred Glinka", "Neubulach"), ("Rau", "Hartmut Rau", "Calw")] {
            Kunde { suchname: s.into(), name1: n.into(), ort: o.into(), ..Default::default() }.speichern(&db).unwrap();
        }
        assert_eq!(kunden_suchen(&db, "manfred neub", "", 10).unwrap().len(), 1);
        assert_eq!(kunden_suchen(&db, "", "", 10).unwrap().len(), 2);
        assert_eq!(kunden_suchen(&db, "xyz", "", 10).unwrap().len(), 0);
    }

    #[test]
    fn anschrift_ohne_leerzeilen() {
        let k = Kunde { anrede: "Herrn".into(), name1: "Manfred Glinka".into(), strasse: "Hölderlinstr. 3".into(), plz: "75387".into(), ort: "Neubulach".into(), ..Default::default() };
        assert_eq!(k.anschrift(), "Herrn\nManfred Glinka\nHölderlinstr. 3\n75387 Neubulach");
    }
}
