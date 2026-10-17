//! Rechnungsbuch: offene Posten, Zahlungseingänge, Mahnungen und Umsatz.

use crate::belege::{self, arten};
use crate::db::{self, err, Datensatz, Res};
use crate::einstellungen;
use crate::modelle::{Beleg, Mahnung, Zahlung};
use crate::rechnen::cent;
use crate::state::AppState;
use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::State;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Filter {
    /// Jahr (leer = alle)
    pub jahr: String,
    /// Belegart (leer = alle Rechnungsarten, "alle" = auch Angebote usw.)
    pub art: String,
    /// alle, offen, bezahlt, faellig
    pub status: String,
    pub suche: String,
    pub kunde_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct Zeile {
    pub id: i64,
    pub projekt_id: i64,
    pub kunde_id: i64,
    pub art: String,
    pub titel: String,
    pub nummer: String,
    pub datum: String,
    pub faellig: String,
    pub kunde: String,
    pub projekt: String,
    pub netto: Decimal,
    pub mwst: Decimal,
    pub betrag: Decimal,
    pub bezahlt: Decimal,
    pub offen: Decimal,
    pub bezahlt_am: String,
    pub mahnstufe: i64,
    pub status: String,
    pub waehrung: String,
    pub ueberfaellig: bool,
}

#[derive(Debug, Serialize, Default)]
pub struct Liste {
    pub zeilen: Vec<Zeile>,
    pub netto: Decimal,
    pub mwst: Decimal,
    pub betrag: Decimal,
    pub offen: Decimal,
    pub jahre: Vec<String>,
}

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_default()
}

pub fn faellig_am(datum: &str, tage: i64) -> String {
    chrono::NaiveDate::parse_from_str(datum, "%Y-%m-%d")
        .map(|t| (t + chrono::Duration::days(tage.max(0))).format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

pub fn liste(db: &Connection, f: &Filter) -> Res<Liste> {
    let mut params: Vec<String> = vec![];
    let mut bed = vec!["b.status <> 'entwurf'".to_string()];
    match f.art.as_str() {
        "" => bed.push("b.art IN ('abschlag','schluss','rechnung','gutschrift')".into()),
        "alle" => {}
        a => {
            params.push(a.to_string());
            bed.push(format!("b.art = ?{}", params.len()));
        }
    }
    if !f.jahr.is_empty() {
        params.push(f.jahr.clone());
        bed.push(format!("substr(b.datum, 1, 4) = ?{}", params.len()));
    }
    if let Some(k) = f.kunde_id {
        params.push(k.to_string());
        bed.push(format!("b.kunde_id = ?{}", params.len()));
    }
    if !f.suche.trim().is_empty() {
        bed.push(crate::kunden::suchbedingung(
            &f.suche,
            &["b.nummer", "b.titel", "b.adresse", "b.betreff", "k.suchname", "k.name1", "p.nummer", "p.bezeichnung"],
            &mut params,
        ));
    }
    let sql = format!(
        "SELECT b.id, b.projekt_id, b.kunde_id, b.art, b.titel, b.nummer, b.datum, b.zahlungsziel_tage,
                CASE WHEN k.name1 = '' THEN k.suchname ELSE k.name1 END, p.nummer || ' ' || p.bezeichnung,
                b.netto, b.mwst, b.zahlbetrag, b.bezahlt, b.bezahlt_am, b.mahnstufe, b.status, b.waehrung
         FROM beleg b JOIN kunde k ON k.id = b.kunde_id JOIN projekt p ON p.id = b.projekt_id
         WHERE {} ORDER BY b.datum DESC, b.id DESC",
        bed.join(" AND ")
    );
    let heute = db::heute();
    let mut stmt = db.prepare(&sql).map_err(err)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |r| {
            let art: String = r.get(3)?;
            let datum: String = r.get(6)?;
            let status: String = r.get(16)?;
            let betrag = d(&r.get::<_, String>(12)?);
            let bezahlt = d(&r.get::<_, String>(13)?);
            let forderung = arten::art(&art).forderung && status != "storniert";
            let offen = if forderung { betrag - bezahlt } else { Decimal::ZERO };
            let faellig = faellig_am(&datum, r.get(7)?);
            Ok(Zeile {
                id: r.get(0)?,
                projekt_id: r.get(1)?,
                kunde_id: r.get(2)?,
                ueberfaellig: !offen.is_zero() && offen > Decimal::ZERO && !faellig.is_empty() && faellig < heute,
                art,
                titel: r.get(4)?,
                nummer: r.get(5)?,
                datum,
                faellig,
                kunde: r.get(8)?,
                projekt: r.get(9)?,
                netto: d(&r.get::<_, String>(10)?),
                mwst: d(&r.get::<_, String>(11)?),
                betrag,
                bezahlt,
                offen,
                bezahlt_am: r.get(14)?,
                mahnstufe: r.get(15)?,
                status,
                waehrung: r.get(17)?,
            })
        })
        .map_err(err)?;
    let mut l = Liste::default();
    for z in rows {
        let z = z.map_err(err)?;
        let ok = match f.status.as_str() {
            "offen" => !z.offen.is_zero(),
            "bezahlt" => z.offen.is_zero(),
            "faellig" => z.ueberfaellig,
            _ => true,
        };
        if !ok {
            continue;
        }
        // Gutschriften mindern den Umsatz.
        let vz = if z.art == "gutschrift" { Decimal::NEGATIVE_ONE } else { Decimal::ONE };
        if z.waehrung != "DM" && z.status != "storniert" {
            l.netto += vz * z.netto;
            l.mwst += vz * z.mwst;
            l.betrag += vz * z.betrag;
        }
        l.offen += vz * z.offen;
        l.zeilen.push(z);
    }
    let mut stmt = db
        .prepare("SELECT DISTINCT substr(datum, 1, 4) FROM beleg WHERE datum <> '' ORDER BY 1 DESC")
        .map_err(err)?;
    l.jahre = stmt.query_map([], |r| r.get(0)).map_err(err)?.collect::<Result<Vec<_>, _>>().map_err(err)?;
    Ok(l)
}

#[tauri::command]
pub fn rechnungsbuch(state: State<AppState>, filter: Filter) -> Res<Liste> {
    state.mit_db(|db| liste(db, &filter))
}

/// Zahlungsstand des Belegs aus seinen Zahlungen neu berechnen.
pub fn zahlstand(db: &Connection, beleg_id: i64) -> Res<()> {
    let zahlungen = Zahlung::alle(db, "WHERE beleg_id = ?1 ORDER BY datum", &[&beleg_id])?;
    let mut b = Beleg::laden(db, beleg_id)?;
    b.bezahlt = zahlungen.iter().map(|z| z.betrag + z.abzug).sum();
    b.bezahlt_am = zahlungen.last().map(|z| z.datum.clone()).unwrap_or_default();
    if b.bezahlt >= b.zahlbetrag && !b.zahlbetrag.is_zero() {
        b.mahnstufe = 0;
    }
    b.speichern(db)?;
    Ok(())
}

#[tauri::command]
pub fn zahlung_buchen(state: State<AppState>, mut zahlung: Zahlung) -> Res<()> {
    state.mit_db(|db| {
        let b = Beleg::laden(db, zahlung.beleg_id)?;
        if b.status == "entwurf" {
            return Err("Zahlungen werden erst für gedruckte bzw. festgeschriebene Rechnungen erfasst.".into());
        }
        if zahlung.datum.is_empty() {
            zahlung.datum = db::heute();
        }
        zahlung.betrag = cent(zahlung.betrag);
        zahlung.abzug = cent(zahlung.abzug);
        zahlung.speichern(db)?;
        zahlstand(db, zahlung.beleg_id)
    })
}

#[tauri::command]
pub fn zahlung_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| {
        let z = Zahlung::laden(db, id)?;
        Zahlung::loeschen(db, id)?;
        zahlstand(db, z.beleg_id)
    })
}

/// Mahnung der nächsten Stufe (oder `stufe`) anlegen. Stufe 0 = Zahlungserinnerung.
pub fn mahnen(db: &Connection, beleg_id: i64, stufe: Option<i64>) -> Res<Mahnung> {
    let mut b = Beleg::laden(db, beleg_id)?;
    if !arten::art(&b.art).forderung || b.status == "entwurf" || b.status == "storniert" {
        return Err("Gemahnt werden nur gedruckte, nicht stornierte Rechnungen.".into());
    }
    if b.zahlbetrag - b.bezahlt <= Decimal::ZERO {
        return Err("Die Rechnung ist bereits bezahlt.".into());
    }
    let bisher = Mahnung::alle(db, "WHERE beleg_id = ?1 ORDER BY stufe DESC", &[&beleg_id])?;
    let stufe = stufe.unwrap_or_else(|| bisher.first().map(|m| (m.stufe + 1).min(3)).unwrap_or(0));
    let allg = einstellungen::allgemein(db)?;
    let vorgabe = allg.mahnstufen.get(stufe as usize).cloned().unwrap_or_default();
    let heute = db::heute();
    let mut m = Mahnung {
        beleg_id,
        stufe,
        datum: heute.clone(),
        frist: faellig_am(&heute, if vorgabe.frist_tage > 0 { vorgabe.frist_tage } else { 7 }),
        gebuehr: vorgabe.gebuehr,
        text: String::new(),
        ..Default::default()
    };
    m.speichern(db)?;
    b.mahnstufe = stufe.max(1).max(b.mahnstufe);
    if stufe == 0 {
        b.mahnstufe = b.mahnstufe.max(0);
    }
    b.speichern(db)?;
    Ok(m)
}

#[tauri::command]
pub fn mahnung_erstellen(state: State<AppState>, beleg_id: i64, stufe: Option<i64>) -> Res<Mahnung> {
    state.mit_db(|db| mahnen(db, beleg_id, stufe))
}

#[tauri::command]
pub fn mahnung_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| {
        let m = Mahnung::laden(db, id)?;
        Mahnung::loeschen(db, id)?;
        let rest = Mahnung::alle(db, "WHERE beleg_id = ?1 ORDER BY stufe DESC", &[&m.beleg_id])?;
        let mut b = Beleg::laden(db, m.beleg_id)?;
        b.mahnstufe = rest.first().map(|m| m.stufe).unwrap_or(0);
        b.speichern(db)?;
        Ok(())
    })
}

/// Überfällige Rechnungen als Mahnvorschlag.
#[tauri::command]
pub fn mahnvorschlag(state: State<AppState>) -> Res<Vec<Zeile>> {
    state.mit_db(|db| {
        let l = liste(db, &Filter { status: "faellig".into(), ..Default::default() })?;
        Ok(l.zeilen.into_iter().filter(|z| z.waehrung != "DM" && z.datum.as_str() >= "2010-01-01").collect())
    })
}

#[derive(Debug, Serialize, Default)]
pub struct Umsatz {
    pub jahr: String,
    pub monate: Vec<(u32, Decimal, Decimal)>,
    pub kunden: Vec<(String, Decimal)>,
    pub netto: Decimal,
    pub mwst: Decimal,
    pub offen: Decimal,
    pub angebote: Decimal,
    pub angebote_anzahl: i64,
    pub jahre: Vec<(String, Decimal)>,
}

/// Umsatz eines Jahres nach Monaten und Kunden (Rechnungen und Schlussrechnungen abzüglich Gutschriften;
/// Abschlagsrechnungen zählen nicht extra, weil die Schlussrechnung den Gesamtbetrag enthält).
#[tauri::command]
pub fn umsatz(state: State<AppState>, jahr: String) -> Res<Umsatz> {
    state.mit_db(|db| {
        let mut u = Umsatz { jahr: jahr.clone(), ..Default::default() };
        let vz = "CASE WHEN art = 'gutschrift' THEN -1 ELSE 1 END";
        let basis = "art IN ('rechnung','schluss','gutschrift') AND status IN ('festgeschrieben','gedruckt') AND waehrung <> 'DM'";
        let mut stmt = db
            .prepare(&format!(
                "SELECT CAST(substr(datum, 6, 2) AS INTEGER), SUM({vz} * CAST(netto AS REAL)), SUM({vz} * CAST(mwst AS REAL))
                 FROM beleg WHERE {basis} AND substr(datum, 1, 4) = ?1 GROUP BY 1 ORDER BY 1"
            ))
            .map_err(err)?;
        let f = |x: f64| cent(Decimal::try_from(x).unwrap_or_default());
        u.monate = stmt
            .query_map([&jahr], |r| Ok((r.get::<_, u32>(0)?, f(r.get(1)?), f(r.get(2)?))))
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        u.netto = u.monate.iter().map(|m| m.1).sum();
        u.mwst = u.monate.iter().map(|m| m.2).sum();
        let mut stmt = db
            .prepare(
                "SELECT CASE WHEN k.name1 = '' THEN k.suchname ELSE k.name1 END,
                        SUM(CASE WHEN b.art = 'gutschrift' THEN -1 ELSE 1 END * CAST(b.netto AS REAL)) AS s
                 FROM beleg b JOIN kunde k ON k.id = b.kunde_id
                 WHERE b.art IN ('rechnung','schluss','gutschrift') AND b.status IN ('festgeschrieben','gedruckt')
                   AND b.waehrung <> 'DM' AND substr(b.datum, 1, 4) = ?1
                 GROUP BY b.kunde_id ORDER BY s DESC LIMIT 15",
            )
            .map_err(err)?;
        u.kunden = stmt
            .query_map([&jahr], |r| Ok((r.get::<_, String>(0)?, f(r.get(1)?))))
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        let (angebote, anzahl): (f64, i64) = db
            .query_row(
                "SELECT COALESCE(SUM(CAST(netto AS REAL)), 0), COUNT(*) FROM beleg WHERE art = 'angebot' AND substr(datum, 1, 4) = ?1",
                [&jahr],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(err)?;
        u.angebote = f(angebote);
        u.angebote_anzahl = anzahl;
        let offen = liste(db, &Filter { jahr: jahr.clone(), status: "offen".into(), ..Default::default() })?;
        u.offen = offen.offen;
        let mut stmt = db
            .prepare(&format!("SELECT substr(datum, 1, 4), SUM({vz} * CAST(netto AS REAL)) FROM beleg WHERE {basis} AND datum <> '' GROUP BY 1 ORDER BY 1"))
            .map_err(err)?;
        u.jahre = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, f(r.get(1)?))))
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        Ok(u)
    })
}

/// Rechnungsbuch als CSV (Semikolon, für Excel und den Steuerberater).
#[tauri::command]
pub fn rechnungsbuch_csv(state: State<AppState>, filter: Filter, pfad: String) -> Res<()> {
    let l = state.mit_db(|db| liste(db, &filter))?;
    let zahl = |d: Decimal| format!("{:.2}", d).replace('.', ",");
    let mut out = String::from("\u{feff}Datum;Nummer;Art;Kunde;Projekt;Netto;MwSt;Betrag;Bezahlt;Offen;Bezahlt am;Mahnstufe;Status;Währung\r\n");
    for z in &l.zeilen {
        let feld = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{};{};{};{};{};{};{}\r\n",
            belege::datum_de(&z.datum),
            feld(&z.nummer),
            feld(&z.titel),
            feld(&z.kunde),
            feld(&z.projekt),
            zahl(z.netto),
            zahl(z.mwst),
            zahl(z.betrag),
            zahl(z.bezahlt),
            zahl(z.offen),
            belege::datum_de(&z.bezahlt_am),
            z.mahnstufe,
            z.status,
            z.waehrung
        ));
    }
    std::fs::write(&pfad, out).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belege::{festschreiben, neu, schreiben, BelegVoll};
    use crate::modelle::{Kunde, Position, Projekt};

    #[test]
    fn zahlung_und_mahnung() {
        let db = db::test_db();
        let k = Kunde { name1: "Test".into(), ..Default::default() }.speichern(&db).unwrap();
        let p = Projekt { kunde_id: k, nummer: "1".into(), ..Default::default() }.speichern(&db).unwrap();
        let mut v = BelegVoll { beleg: neu(&db, p, "rechnung").unwrap(), ..Default::default() };
        v.beleg.datum = "2024-01-01".into();
        v.beleg.zahlungsziel_tage = 14;
        v.positionen.push(Position { art: "N".into(), menge: Decimal::ONE, ep: Decimal::from(100), ..Default::default() });
        let id = schreiben(&db, &mut v).unwrap();
        assert!(mahnen(&db, id, None).is_err(), "Entwurf");
        festschreiben(&db, id).unwrap();
        let l = liste(&db, &Filter { status: "faellig".into(), ..Default::default() }).unwrap();
        assert_eq!(l.zeilen.len(), 1);
        assert_eq!(l.zeilen[0].faellig, "2024-01-15");
        assert_eq!(l.offen, Decimal::from(119));
        let m = mahnen(&db, id, None).unwrap();
        assert_eq!(m.stufe, 0);
        assert_eq!(mahnen(&db, id, None).unwrap().stufe, 1);
        Zahlung { beleg_id: id, datum: "2024-02-01".into(), betrag: Decimal::from(116), abzug: Decimal::from(3), ..Default::default() }.speichern(&db).unwrap();
        zahlstand(&db, id).unwrap();
        let l = liste(&db, &Filter { status: "offen".into(), ..Default::default() }).unwrap();
        assert!(l.zeilen.is_empty());
        assert_eq!(Beleg::laden(&db, id).unwrap().mahnstufe, 0);
    }
}
