//! Einstellungen (als JSON in der Tabelle `einstellung`).

use crate::db::{self, err, Res};
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Bank {
    pub name: String,
    pub iban: String,
    pub bic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Firma {
    pub name: String,
    pub inhaber: String,
    pub zusatz: String,
    pub strasse: String,
    pub plz: String,
    pub ort: String,
    pub land: String,
    pub telefon: String,
    pub telefax: String,
    pub mobil: String,
    pub email: String,
    pub internet: String,
    pub steuernummer: String,
    pub ustid: String,
    pub registergericht: String,
    pub banken: Vec<Bank>,
    /// Kleine Absenderzeile über dem Adressfeld.
    pub absenderzeile: String,
    /// Fußzeile (mehrzeilig), wenn kein Fußzeilenbild hinterlegt ist.
    pub fusszeile: String,
    /// Leistungen am rechten Rand (eine je Zeile).
    pub seitenleiste: String,
    pub waehrung: String,
    pub mwst_satz: Decimal,
    pub mwst_ermaessigt: Decimal,
    /// Ort für „Neubulach, den …“ in Briefen.
    pub briefort: String,
    pub gruss: String,
    pub unterschrift: String,
}

impl Default for Firma {
    fn default() -> Self {
        Self {
            name: String::new(),
            inhaber: String::new(),
            zusatz: String::new(),
            strasse: String::new(),
            plz: String::new(),
            ort: String::new(),
            land: "DE".into(),
            telefon: String::new(),
            telefax: String::new(),
            mobil: String::new(),
            email: String::new(),
            internet: String::new(),
            steuernummer: String::new(),
            ustid: String::new(),
            registergericht: String::new(),
            banken: vec![],
            absenderzeile: String::new(),
            fusszeile: String::new(),
            seitenleiste: String::new(),
            waehrung: "EUR".into(),
            mwst_satz: Decimal::from(19),
            mwst_ermaessigt: Decimal::from(7),
            briefort: String::new(),
            gruss: "Mit freundlichen Grüßen".into(),
            unterschrift: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Zahlungsbedingung {
    pub name: String,
    /// Text mit Platzhaltern: #ZN# Zahlungsziel, #ZS# Skontofrist, #SK# Skontosatz, #WE# Währung,
    /// #BN# Betrag, #SB# Skontobetrag, #BS# Betrag nach Skonto
    pub text: String,
    pub skonto: Decimal,
    pub skonto_tage: i64,
    pub ziel_tage: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Mahnstufe {
    pub titel: String,
    pub gebuehr: Decimal,
    pub frist_tage: i64,
    pub einleitung: String,
    pub schluss: String,
}

/// Vorgaben je Belegart (Schlüssel = `belege::ARTEN`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Belegvorgabe {
    pub vortext: String,
    pub nachtext: String,
    pub zahlungsbedingung: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Kalkulationssaetze {
    /// Verrechnungslohn je Stunde
    pub stundensatz: Decimal,
    pub zuschlag_lohn: Decimal,
    pub zuschlag_material: Decimal,
    pub zuschlag_geraet: Decimal,
    pub zuschlag_fremd: Decimal,
    pub zuschlag_sonstig: Decimal,
}

impl Default for Kalkulationssaetze {
    fn default() -> Self {
        Self {
            stundensatz: Decimal::from(55),
            zuschlag_lohn: Decimal::ZERO,
            zuschlag_material: Decimal::from(20),
            zuschlag_geraet: Decimal::from(10),
            zuschlag_fremd: Decimal::from(15),
            zuschlag_sonstig: Decimal::from(10),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Allgemein {
    pub einheiten: Vec<String>,
    pub zahlungsbedingungen: Vec<Zahlungsbedingung>,
    /// Index in `zahlungsbedingungen` für neue Rechnungen.
    pub zahlungsbedingung_standard: usize,
    /// Stufe 0 = Zahlungserinnerung, 1–3 = Mahnungen.
    pub mahnstufen: Vec<Mahnstufe>,
    pub belegvorgaben: std::collections::BTreeMap<String, Belegvorgabe>,
    pub kalkulation: Kalkulationssaetze,
    /// Ordner für gespeicherte PDFs (leer = Dokumente\mrfakt).
    pub pdf_ordner: String,
    /// Anzahl automatischer Sicherungen, die aufbewahrt werden.
    pub sicherungen: usize,
    /// E-Rechnung (ZUGFeRD) in Rechnungen an Firmen einbetten.
    pub zugferd: bool,
    /// Ränder des Briefpapiers in mm.
    pub rand_links: Decimal,
    pub rand_rechts: Decimal,
    pub rand_oben: Decimal,
    pub rand_unten: Decimal,
}

impl Default for Allgemein {
    fn default() -> Self {
        let einheiten = ["m²", "m", "lfm", "m³", "Std.", "St.", "Stück", "kg", "l", "pauschal", "psch.", "Sack", "t"];
        Self {
            einheiten: einheiten.iter().map(|s| s.to_string()).collect(),
            zahlungsbedingungen: vec![Zahlungsbedingung {
                name: "Standard".into(),
                text: "Zahlbar bis zum #ZN# ohne Abzug.".into(),
                ziel_tage: 14,
                ..Default::default()
            }],
            zahlungsbedingung_standard: 0,
            mahnstufen: vec![
                Mahnstufe { titel: "Zahlungserinnerung".into(), frist_tage: 8, ..Default::default() },
                Mahnstufe { titel: "1. Mahnung".into(), frist_tage: 7, gebuehr: Decimal::from(5), ..Default::default() },
                Mahnstufe { titel: "2. Mahnung".into(), frist_tage: 6, gebuehr: Decimal::from(10), ..Default::default() },
                Mahnstufe { titel: "3. Mahnung".into(), frist_tage: 5, gebuehr: Decimal::from(15), ..Default::default() },
            ],
            belegvorgaben: Default::default(),
            kalkulation: Kalkulationssaetze::default(),
            pdf_ordner: String::new(),
            sicherungen: 30,
            zugferd: true,
            rand_links: Decimal::from(20),
            rand_rechts: Decimal::from(15),
            rand_oben: Decimal::from(15),
            rand_unten: Decimal::from(15),
        }
    }
}

pub fn firma(db: &rusqlite::Connection) -> Res<Firma> {
    Ok(db::einstellung(db, "firma")?.unwrap_or_default())
}

pub fn allgemein(db: &rusqlite::Connection) -> Res<Allgemein> {
    Ok(db::einstellung(db, "allgemein")?.unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Einstellungen {
    pub firma: Firma,
    pub allgemein: Allgemein,
    pub nummernkreise: Vec<Nummernkreis>,
    pub logo: bool,
    pub fussbild: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nummernkreis {
    pub name: String,
    pub naechste: i64,
    pub format: String,
}

pub const KREISE: &[&str] = &["kunde", "projekt", "angebot", "auftrag", "lieferschein", "rechnung", "gutschrift"];

#[tauri::command]
pub fn einstellungen_laden(state: State<AppState>) -> Res<Einstellungen> {
    state.mit_db(|db| {
        let mut nummernkreise = vec![];
        for k in KREISE {
            let (naechste, format) = db
                .query_row("SELECT naechste, format FROM nummernkreis WHERE name = ?1", [k], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap_or((1, "{n}".to_string()));
            nummernkreise.push(Nummernkreis { name: k.to_string(), naechste, format });
        }
        let hat = |n: &str| db.query_row("SELECT COUNT(*) FROM datei WHERE name = ?1", [n], |r| r.get::<_, i64>(0)).unwrap_or(0) > 0;
        Ok(Einstellungen { firma: firma(db)?, allgemein: allgemein(db)?, nummernkreise, logo: hat("logo"), fussbild: hat("fusszeile") })
    })
}

#[tauri::command]
pub fn einstellungen_speichern(state: State<AppState>, firma: Firma, allgemein: Allgemein, nummernkreise: Vec<Nummernkreis>) -> Res<()> {
    state.mit_db(|db| {
        let tx = db.unchecked_transaction().map_err(err)?;
        db::einstellung_setzen(&tx, "firma", &firma)?;
        db::einstellung_setzen(&tx, "allgemein", &allgemein)?;
        for k in nummernkreise {
            tx.execute(
                "INSERT INTO nummernkreis (name, naechste, format) VALUES (?1, ?2, ?3)
                 ON CONFLICT(name) DO UPDATE SET naechste = excluded.naechste, format = excluded.format",
                rusqlite::params![k.name, k.naechste, k.format],
            )
            .map_err(err)?;
        }
        tx.commit().map_err(err)
    })
}

/// Bild für Briefkopf (`logo`) oder Fußzeile (`fusszeile`) setzen; leerer Pfad entfernt es.
#[tauri::command]
pub fn bild_setzen(state: State<AppState>, name: String, pfad: String) -> Res<()> {
    if name != "logo" && name != "fusszeile" {
        return Err("Unbekanntes Bild".into());
    }
    state.mit_db(|db| {
        if pfad.is_empty() {
            db.execute("DELETE FROM datei WHERE name = ?1", [&name]).map_err(err)?;
            return Ok(());
        }
        let daten = std::fs::read(&pfad).map_err(err)?;
        let mime = bild_mime(&pfad).ok_or("Bitte ein PNG- oder JPG-Bild wählen.")?;
        db.execute(
            "INSERT INTO datei (name, mime, daten) VALUES (?1, ?2, ?3) ON CONFLICT(name) DO UPDATE SET mime = excluded.mime, daten = excluded.daten",
            rusqlite::params![name, mime, daten],
        )
        .map_err(err)?;
        Ok(())
    })
}

pub fn bild_mime(pfad: &str) -> Option<&'static str> {
    let p = pfad.to_ascii_lowercase();
    if p.ends_with(".png") {
        Some("image/png")
    } else if p.ends_with(".jpg") || p.ends_with(".jpeg") {
        Some("image/jpeg")
    } else {
        None
    }
}

/// Bild als data:-URL für die Vorschau in den Einstellungen.
#[tauri::command]
pub fn bild_laden(state: State<AppState>, name: String) -> Res<Option<String>> {
    state.mit_db(|db| {
        let r: Option<(String, Vec<u8>)> = rusqlite::OptionalExtension::optional(
            db.query_row("SELECT mime, daten FROM datei WHERE name = ?1", [&name], |r| Ok((r.get(0)?, r.get(1)?))),
        )
        .map_err(err)?;
        Ok(r.map(|(mime, daten)| format!("data:{mime};base64,{}", base64(&daten))))
    })
}

pub fn base64(daten: &[u8]) -> String {
    const ZEICHEN: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(daten.len().div_ceil(3) * 4);
    for block in daten.chunks(3) {
        let b = [block[0], *block.get(1).unwrap_or(&0), *block.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= block.len() {
                out.push(ZEICHEN[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_wie_rfc4648() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }
}
