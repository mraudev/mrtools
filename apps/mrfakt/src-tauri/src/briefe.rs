//! Briefe, Textbausteine und die globale Suche.

use crate::db::{self, err, Datensatz, Res};
use crate::modelle::{Brief, Kunde, Textbaustein};
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[tauri::command]
pub fn briefe_liste(state: State<AppState>, kunde_id: i64) -> Res<Vec<Brief>> {
    state.mit_db(|db| Brief::alle(db, "WHERE kunde_id = ?1 ORDER BY datum DESC, id DESC", &[&kunde_id]))
}

#[tauri::command]
pub fn brief_laden(state: State<AppState>, id: i64) -> Res<Brief> {
    state.mit_db(|db| Brief::laden(db, id))
}

#[tauri::command]
pub fn brief_neu(state: State<AppState>, kunde_id: i64, projekt_id: Option<i64>) -> Res<Brief> {
    state.mit_db(|db| {
        let k = Kunde::laden(db, kunde_id)?;
        let anrede = match (k.geschlecht.as_str(), k.anrede.as_str()) {
            ("Herr", _) | (_, "Herrn") | (_, "Herr") => format!("Sehr geehrter Herr {},", k.name1.split_whitespace().last().unwrap_or("")),
            ("Frau", _) | (_, "Frau") => format!("Sehr geehrte Frau {},", k.name1.split_whitespace().last().unwrap_or("")),
            _ => "Sehr geehrte Damen und Herren,".into(),
        };
        let mut b = Brief {
            kunde_id,
            projekt_id,
            datum: db::heute(),
            adresse: k.anschrift(),
            text: format!("<p>{anrede}</p><p></p>"),
            angelegt: db::jetzt(),
            ..Default::default()
        };
        b.speichern(db)?;
        Ok(b)
    })
}

#[tauri::command]
pub fn brief_speichern(state: State<AppState>, brief: Brief) -> Res<i64> {
    let mut b = brief;
    state.mit_db(|db| b.speichern(db))
}

#[tauri::command]
pub fn brief_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| Brief::loeschen(db, id))
}

#[tauri::command]
pub fn textbausteine(state: State<AppState>) -> Res<Vec<Textbaustein>> {
    state.mit_db(|db| Textbaustein::alle(db, "ORDER BY kategorie, name COLLATE NOCASE", &[]))
}

#[tauri::command]
pub fn textbaustein_speichern(state: State<AppState>, baustein: Textbaustein) -> Res<i64> {
    let mut t = baustein;
    state.mit_db(|db| t.speichern(db))
}

#[tauri::command]
pub fn textbaustein_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| Textbaustein::loeschen(db, id))
}

#[derive(Debug, Serialize)]
pub struct Treffer {
    /// kunde, projekt, beleg, artikel
    pub art: String,
    pub id: i64,
    pub kunde_id: i64,
    pub projekt_id: i64,
    pub titel: String,
    pub info: String,
}

/// Sucht in Kunden, Projekten, Belegen (Nummer/Betreff) und Artikeln.
#[tauri::command]
pub fn suche(state: State<AppState>, text: String) -> Res<Vec<Treffer>> {
    if text.trim().len() < 2 {
        return Ok(vec![]);
    }
    state.mit_db(|db| {
        let mut out = vec![];
        for k in crate::kunden::kunden_suchen(db, &text, "", 8)? {
            out.push(Treffer { art: "kunde".into(), id: k.id, kunde_id: k.id, projekt_id: 0, titel: if k.name.is_empty() { k.suchname } else { k.name }, info: k.ort });
        }
        let mut params = vec![];
        let bed = crate::kunden::suchbedingung(&text, &["p.nummer", "p.bezeichnung", "p.baustelle"], &mut params);
        let mut stmt = db
            .prepare(&format!(
                "SELECT p.id, p.kunde_id, p.nummer, p.bezeichnung, CASE WHEN k.name1 = '' THEN k.suchname ELSE k.name1 END
                 FROM projekt p JOIN kunde k ON k.id = p.kunde_id WHERE {bed} ORDER BY p.datum DESC LIMIT 8"
            ))
            .map_err(err)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |r| {
                Ok(Treffer { art: "projekt".into(), id: r.get(0)?, kunde_id: r.get(1)?, projekt_id: r.get(0)?, titel: format!("{} {}", r.get::<_, String>(2)?, r.get::<_, String>(3)?), info: r.get(4)? })
            })
            .map_err(err)?;
        for t in rows {
            out.push(t.map_err(err)?);
        }
        let mut params = vec![];
        let bed = crate::kunden::suchbedingung(&text, &["b.nummer", "b.betreff", "b.titel"], &mut params);
        let mut stmt = db
            .prepare(&format!(
                "SELECT b.id, b.kunde_id, b.projekt_id, b.titel, b.nummer, b.datum, CASE WHEN k.name1 = '' THEN k.suchname ELSE k.name1 END
                 FROM beleg b JOIN kunde k ON k.id = b.kunde_id WHERE {bed} AND b.nummer <> '' ORDER BY b.datum DESC LIMIT 10"
            ))
            .map_err(err)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |r| {
                Ok(Treffer {
                    art: "beleg".into(),
                    id: r.get(0)?,
                    kunde_id: r.get(1)?,
                    projekt_id: r.get(2)?,
                    titel: format!("{} {}", r.get::<_, String>(3)?, r.get::<_, String>(4)?),
                    info: format!("{} · {}", crate::belege::datum_de(&r.get::<_, String>(5)?), r.get::<_, String>(6)?),
                })
            })
            .map_err(err)?;
        for t in rows {
            out.push(t.map_err(err)?);
        }
        let mut params = vec![];
        let bed = crate::kunden::suchbedingung(&text, &["nummer", "suchtext", "text"], &mut params);
        let mut stmt = db.prepare(&format!("SELECT id, nummer, suchtext, vk1, einheit FROM artikel WHERE {bed} LIMIT 6")).map_err(err)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |r| {
                Ok(Treffer { art: "artikel".into(), id: r.get(0)?, kunde_id: 0, projekt_id: 0, titel: r.get(2)?, info: format!("{} · {} € / {}", r.get::<_, String>(1)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?) })
            })
            .map_err(err)?;
        for t in rows {
            out.push(t.map_err(err)?);
        }
        Ok(out)
    })
}
