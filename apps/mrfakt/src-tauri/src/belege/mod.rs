//! Belege: Angebote, Auftragsbestätigungen, Lieferscheine, Rechnungen, Gutschriften.

pub mod arten;

use crate::db::{self, err, Datensatz, Res};
use crate::einstellungen;
use crate::modelle::*;
use crate::rechnen::{self, Summen};
use crate::state::AppState;
use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BelegVoll {
    pub beleg: Beleg,
    pub positionen: Vec<Position>,
    pub zuschlaege: Vec<Zuschlag>,
    pub abzuege: Vec<AbschlagAbzug>,
    #[serde(skip_deserializing)]
    pub summen: Summen,
    #[serde(skip_deserializing)]
    pub kunde: Option<Kunde>,
    #[serde(skip_deserializing)]
    pub projekt: Option<Projekt>,
    #[serde(skip_deserializing)]
    pub zahlungen: Vec<Zahlung>,
    #[serde(skip_deserializing)]
    pub mahnungen: Vec<Mahnung>,
    #[serde(skip_deserializing)]
    pub aufmass: i64,
    #[serde(skip_deserializing)]
    pub nachfolger: Vec<BelegKurz>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BelegKurz {
    pub id: i64,
    pub art: String,
    pub titel: String,
    pub nummer: String,
    pub datum: String,
    pub status: String,
    pub netto: Decimal,
    pub zahlbetrag: Decimal,
    pub bezahlt: Decimal,
    pub waehrung: String,
}

impl From<&Beleg> for BelegKurz {
    fn from(b: &Beleg) -> Self {
        Self {
            id: b.id,
            art: b.art.clone(),
            titel: b.titel.clone(),
            nummer: b.nummer.clone(),
            datum: b.datum.clone(),
            status: b.status.clone(),
            netto: b.netto,
            zahlbetrag: b.zahlbetrag,
            bezahlt: b.bezahlt,
            waehrung: b.waehrung.clone(),
        }
    }
}

pub fn voll_laden(db: &Connection, id: i64) -> Res<BelegVoll> {
    let beleg = Beleg::laden(db, id)?;
    let positionen = Position::alle(db, "WHERE beleg_id = ?1 ORDER BY pos", &[&id])?;
    let zuschlaege = Zuschlag::alle(db, "WHERE beleg_id = ?1 ORDER BY pos", &[&id])?;
    let abzuege = AbschlagAbzug::alle(db, "WHERE beleg_id = ?1 ORDER BY pos", &[&id])?;
    let summen = rechnen::summen(&beleg, &positionen, &zuschlaege, &abzuege);
    let kunde = Kunde::laden(db, beleg.kunde_id).ok();
    let projekt = Projekt::laden(db, beleg.projekt_id).ok();
    let zahlungen = Zahlung::alle(db, "WHERE beleg_id = ?1 ORDER BY datum", &[&id])?;
    let mahnungen = Mahnung::alle(db, "WHERE beleg_id = ?1 ORDER BY datum", &[&id])?;
    let aufmass = db.query_row("SELECT COUNT(*) FROM aufmass WHERE beleg_id = ?1", [id], |r| r.get(0)).map_err(err)?;
    let nachfolger = Beleg::alle(db, "WHERE vorgaenger_id = ?1 ORDER BY datum", &[&id])?.iter().map(BelegKurz::from).collect();
    Ok(BelegVoll { beleg, positionen, zuschlaege, abzuege, summen, kunde, projekt, zahlungen, mahnungen, aufmass, nachfolger })
}

pub fn ist_gesperrt(b: &Beleg) -> bool {
    b.status == "festgeschrieben" || b.status == "storniert"
}

/// Schreibt Beleg und Unterlisten; berechnet GP und Summen neu.
pub fn schreiben(db: &Connection, v: &mut BelegVoll) -> Res<i64> {
    if v.beleg.id != 0 {
        let alt = Beleg::laden(db, v.beleg.id)?;
        if ist_gesperrt(&alt) {
            return Err("Der Beleg ist festgeschrieben und kann nicht mehr geändert werden. Für Korrekturen eine Gutschrift oder ein Storno erstellen.".into());
        }
        // Zahlungsstand und Status gehören nicht der Bearbeitung.
        v.beleg.bezahlt = alt.bezahlt;
        v.beleg.bezahlt_am = alt.bezahlt_am;
        v.beleg.mahnstufe = alt.mahnstufe;
        v.beleg.alt_betrag = alt.alt_betrag;
        v.beleg.alt_slot = alt.alt_slot;
        v.beleg.angelegt = alt.angelegt;
    } else {
        v.beleg.angelegt = db::jetzt();
    }
    for (i, p) in v.positionen.iter_mut().enumerate() {
        p.pos = i as i64;
        p.gp = if matches!(p.art.as_str(), "N" | "A" | "B") { rechnen::gp(p) } else { Decimal::ZERO };
    }
    let summen = rechnen::summen(&v.beleg, &v.positionen, &v.zuschlaege, &v.abzuege);
    v.beleg.netto = summen.netto;
    v.beleg.mwst = summen.mwst;
    v.beleg.brutto = summen.gesamt;
    v.beleg.abzug = summen.abzug;
    v.beleg.zahlbetrag = summen.zahlbetrag;
    v.beleg.geaendert = db::jetzt();
    let tx = db.unchecked_transaction().map_err(err)?;
    let id = v.beleg.speichern(&tx)?;
    // Unterlisten ersetzen; Positionen behalten ihre Id (Kalkulation hängt daran).
    let behalten: Vec<String> = v.positionen.iter().filter(|p| p.id != 0).map(|p| p.id.to_string()).collect();
    tx.execute(
        &format!("DELETE FROM position WHERE beleg_id = ?1{}", if behalten.is_empty() { String::new() } else { format!(" AND id NOT IN ({})", behalten.join(",")) }),
        [id],
    )
    .map_err(err)?;
    for p in v.positionen.iter_mut() {
        p.beleg_id = id;
        p.speichern(&tx)?;
    }
    tx.execute("DELETE FROM zuschlag WHERE beleg_id = ?1", [id]).map_err(err)?;
    for (i, z) in v.zuschlaege.iter_mut().enumerate() {
        z.id = 0;
        z.beleg_id = id;
        z.pos = i as i64;
        z.speichern(&tx)?;
    }
    tx.execute("DELETE FROM abschlag_abzug WHERE beleg_id = ?1", [id]).map_err(err)?;
    for (i, a) in v.abzuege.iter_mut().enumerate() {
        a.id = 0;
        a.beleg_id = id;
        a.pos = i as i64;
        a.speichern(&tx)?;
    }
    tx.commit().map_err(err)?;
    Ok(id)
}

#[tauri::command]
pub fn beleg_laden(state: State<AppState>, id: i64) -> Res<BelegVoll> {
    state.mit_db(|db| voll_laden(db, id))
}

#[tauri::command]
pub fn beleg_speichern(state: State<AppState>, mut beleg: BelegVoll) -> Res<BelegVoll> {
    state.mit_db(|db| {
        let id = schreiben(db, &mut beleg)?;
        voll_laden(db, id)
    })
}

/// Vorschau der Summen ohne zu speichern (beim Bearbeiten).
#[tauri::command]
pub fn beleg_summen(mut beleg: BelegVoll) -> Summen {
    for p in beleg.positionen.iter_mut() {
        p.gp = if matches!(p.art.as_str(), "N" | "A" | "B") { rechnen::gp(p) } else { Decimal::ZERO };
    }
    rechnen::summen(&beleg.beleg, &beleg.positionen, &beleg.zuschlaege, &beleg.abzuege)
}

/// Neuer, leerer Beleg mit den Vorgaben aus den Einstellungen.
pub fn neu(db: &Connection, projekt_id: i64, art: &str) -> Res<Beleg> {
    let projekt = Projekt::laden(db, projekt_id)?;
    let kunde = Kunde::laden(db, projekt.kunde_id)?;
    let firma = einstellungen::firma(db)?;
    let allg = einstellungen::allgemein(db)?;
    let a = arten::art(art);
    let mut titel = a.titel.to_string();
    if art == "abschlag" {
        let n: i64 = db
            .query_row("SELECT COUNT(*) FROM beleg WHERE projekt_id = ?1 AND art = 'abschlag' AND status <> 'storniert'", [projekt_id], |r| r.get(0))
            .map_err(err)?;
        titel = format!("{} {}", a.titel, n + 1);
    }
    let vorgabe = allg.belegvorgaben.get(art).cloned().unwrap_or_default();
    let mut b = Beleg {
        projekt_id,
        kunde_id: kunde.id,
        art: art.into(),
        titel,
        datum: db::heute(),
        adresse: kunde.anschrift(),
        betreff: projekt.bezeichnung.clone(),
        vortext: vorgabe.vortext,
        nachtext: vorgabe.nachtext,
        mwst_satz: firma.mwst_satz,
        rabatt_text: "Rabatt".into(),
        netto_rechnung: kunde.ust_art == "13b",
        status: "entwurf".into(),
        waehrung: if firma.waehrung.is_empty() { "EUR".into() } else { firma.waehrung.clone() },
        ..Default::default()
    };
    if a.forderung {
        let zb = allg.zahlungsbedingungen.iter().find(|z| !kunde.zahlungsbedingung.is_empty() && z.name == kunde.zahlungsbedingung)
            .or_else(|| allg.zahlungsbedingungen.get(allg.zahlungsbedingung_standard))
            .cloned()
            .unwrap_or_default();
        b.zahlungsbedingung = if vorgabe.zahlungsbedingung.is_empty() { zb.text } else { vorgabe.zahlungsbedingung };
        b.skonto_prozent = zb.skonto;
        b.skonto_tage = zb.skonto_tage;
        b.zahlungsziel_tage = zb.ziel_tage;
    }
    // Nicht-Rechnungen bekommen ihre Nummer sofort, Rechnungen erst beim Festschreiben (lückenlos).
    if !a.forderung && !a.kreis.is_empty() {
        b.nummer = db::naechste_nummer(db, a.kreis)?;
    }
    Ok(b)
}

/// Bisherige Abschlagsrechnungen eines Projekts als Abzüge für eine Schluss-/Abschlagsrechnung.
pub fn abzuege_aus_projekt(db: &Connection, projekt_id: i64, ausser: i64) -> Res<Vec<AbschlagAbzug>> {
    let abschlaege = Beleg::alle(
        db,
        "WHERE projekt_id = ?1 AND art = 'abschlag' AND status IN ('festgeschrieben','gedruckt') AND id <> ?2 ORDER BY datum, id",
        &[&projekt_id, &ausser],
    )?;
    Ok(abschlaege
        .iter()
        .map(|a| AbschlagAbzug {
            text: a.titel.clone(),
            nummer: a.nummer.clone(),
            datum: a.datum.clone(),
            netto: a.netto,
            mwst: a.mwst,
            brutto: a.netto + a.mwst,
            bezahlt: a.bezahlt,
            quelle_id: Some(a.id),
            ..Default::default()
        })
        .collect())
}

#[tauri::command]
pub fn beleg_neu(state: State<AppState>, projekt_id: i64, art: String) -> Res<BelegVoll> {
    state.mit_db(|db| {
        let beleg = neu(db, projekt_id, &art)?;
        let mut v = BelegVoll { beleg, ..Default::default() };
        if art == "schluss" {
            v.abzuege = abzuege_aus_projekt(db, projekt_id, 0)?;
        }
        let id = schreiben(db, &mut v)?;
        voll_laden(db, id)
    })
}

/// Neuen Beleg aus einem bestehenden erstellen (Angebot → Auftragsbestätigung → Rechnung …).
pub fn umwandeln(db: &Connection, id: i64, art: &str, projekt_id: Option<i64>) -> Res<i64> {
    let quelle = voll_laden(db, id)?;
    let ziel_projekt = projekt_id.unwrap_or(quelle.beleg.projekt_id);
    let mut b = neu(db, ziel_projekt, art)?;
    let q = &quelle.beleg;
    b.betreff = q.betreff.clone();
    if projekt_id.is_none() {
        b.adresse = q.adresse.clone();
    }
    b.vortext = q.vortext.clone();
    b.nachtext = if arten::art(art).forderung == arten::art(&q.art).forderung { q.nachtext.clone() } else { b.nachtext };
    b.leistung = q.leistung.clone();
    b.mwst_satz = q.mwst_satz;
    b.rabatt_text = q.rabatt_text.clone();
    b.rabatt_prozent = q.rabatt_prozent;
    b.netto_rechnung = q.netto_rechnung;
    b.lohn_ausweisen = q.lohn_ausweisen;
    b.waehrung = q.waehrung.clone();
    b.vorgaenger_id = Some(q.id);
    let mut v = BelegVoll {
        beleg: b,
        positionen: quelle.positionen.iter().cloned().map(|mut p| { p.id = 0; p }).collect(),
        zuschlaege: quelle.zuschlaege.clone(),
        ..Default::default()
    };
    if art == "schluss" {
        v.abzuege = abzuege_aus_projekt(db, ziel_projekt, 0)?;
    }
    let neu_id = schreiben(db, &mut v)?;
    // Aufmaß mitnehmen
    db.execute(
        "INSERT INTO aufmass (beleg_id, pos, posnr, art, text, anzahl, formel, m1, m2, m3, ergebnis, abzug)
         SELECT ?1, pos, posnr, art, text, anzahl, formel, m1, m2, m3, ergebnis, abzug FROM aufmass WHERE beleg_id = ?2",
        [neu_id, id],
    )
    .map_err(err)?;
    // Kalkulation mitnehmen (Positionen in gleicher Reihenfolge)
    let alt_pos = &quelle.positionen;
    let neu_pos = Position::alle(db, "WHERE beleg_id = ?1 ORDER BY pos", &[&neu_id])?;
    for (a, n) in alt_pos.iter().zip(neu_pos.iter()) {
        db.execute(
            "INSERT INTO kalkulation (position_id, pos, art, text, menge, einheit, preis, zuschlag, artikel_id)
             SELECT ?1, pos, art, text, menge, einheit, preis, zuschlag, artikel_id FROM kalkulation WHERE position_id = ?2",
            [n.id, a.id],
        )
        .map_err(err)?;
    }
    Ok(neu_id)
}

#[tauri::command]
pub fn beleg_umwandeln(state: State<AppState>, id: i64, art: String, projekt_id: Option<i64>) -> Res<BelegVoll> {
    state.mit_db(|db| {
        let neu_id = umwandeln(db, id, &art, projekt_id)?;
        voll_laden(db, neu_id)
    })
}

#[tauri::command]
pub fn beleg_loeschen(state: State<AppState>, id: i64) -> Res<()> {
    state.mit_db(|db| {
        let b = Beleg::laden(db, id)?;
        if ist_gesperrt(&b) {
            return Err("Festgeschriebene Belege können nicht gelöscht werden – bitte stornieren.".into());
        }
        if arten::art(&b.art).forderung && !b.nummer.is_empty() && b.status != "entwurf" {
            return Err("Gedruckte Rechnungen können nicht gelöscht werden – bitte stornieren.".into());
        }
        Beleg::loeschen(db, id)
    })
}

/// Vergibt die Nummer (falls noch keine) und schreibt Rechnungen fest. Gibt den aktuellen Stand zurück.
pub fn festschreiben(db: &Connection, id: i64) -> Res<Beleg> {
    let mut b = Beleg::laden(db, id)?;
    if ist_gesperrt(&b) {
        return Ok(b);
    }
    let a = arten::art(&b.art);
    if b.nummer.trim().is_empty() && !a.kreis.is_empty() {
        b.nummer = db::naechste_nummer(db, a.kreis)?;
    }
    if b.datum.is_empty() {
        b.datum = db::heute();
    }
    b.status = if a.forderung { "festgeschrieben".into() } else { "gedruckt".into() };
    b.gedruckt_am = db::heute();
    b.speichern(db)?;
    // Lager: Lieferscheine buchen ab; Rechnungen nur, wenn sie nicht aus einem Lieferschein entstanden sind.
    let aus_lieferschein = b.vorgaenger_id.and_then(|id| Beleg::laden(db, id).ok()).is_some_and(|v| v.art == "lieferschein");
    if b.art == "lieferschein" || (matches!(b.art.as_str(), "rechnung" | "schluss") && !aus_lieferschein) {
        crate::artikel::beleg_abbuchen(db, &b)?;
    }
    Ok(b)
}

#[tauri::command]
pub fn beleg_festschreiben(state: State<AppState>, id: i64) -> Res<BelegVoll> {
    state.mit_db(|db| {
        festschreiben(db, id)?;
        voll_laden(db, id)
    })
}

/// Storno: erzeugt eine Stornorechnung (Rechnung mit negativen Mengen) und markiert das Original.
pub fn stornieren(db: &Connection, id: i64) -> Res<i64> {
    let q = voll_laden(db, id)?;
    if !arten::art(&q.beleg.art).forderung {
        return Err("Nur Rechnungen werden storniert; andere Belege können gelöscht werden.".into());
    }
    if q.beleg.status == "storniert" {
        return Err("Der Beleg ist bereits storniert.".into());
    }
    if q.beleg.status != "festgeschrieben" {
        return Err("Nicht festgeschriebene Belege bitte einfach bearbeiten oder löschen.".into());
    }
    let mut b = neu(db, q.beleg.projekt_id, "rechnung")?;
    b.titel = format!("Stornorechnung zu {} {}", q.beleg.titel, q.beleg.nummer);
    b.adresse = q.beleg.adresse.clone();
    b.betreff = q.beleg.betreff.clone();
    b.mwst_satz = q.beleg.mwst_satz;
    b.netto_rechnung = q.beleg.netto_rechnung;
    b.rabatt_prozent = q.beleg.rabatt_prozent;
    b.rabatt_text = q.beleg.rabatt_text.clone();
    b.waehrung = q.beleg.waehrung.clone();
    b.vortext = format!("<p>Hiermit stornieren wir {} Nr. {} vom {}.</p>", q.beleg.titel, q.beleg.nummer, datum_de(&q.beleg.datum));
    b.nachtext.clear();
    b.zahlungsbedingung.clear();
    b.vorgaenger_id = Some(id);
    let mut v = BelegVoll {
        beleg: b,
        positionen: q.positionen.iter().cloned().map(|mut p| { p.id = 0; p.menge = -p.menge; p }).collect(),
        zuschlaege: q.zuschlaege.iter().cloned().map(|mut z| { if z.art.starts_with("betrag") { z.wert = -z.wert; } z }).collect(),
        abzuege: q.abzuege.iter().cloned().map(|mut a| { a.netto = -a.netto; a.mwst = -a.mwst; a.brutto = -a.brutto; a }).collect(),
        ..Default::default()
    };
    let neu_id = schreiben(db, &mut v)?;
    let mut storno = festschreiben(db, neu_id)?;
    // Original und Storno gleichen sich aus.
    let mut orig = Beleg::laden(db, id)?;
    orig.status = "storniert".into();
    orig.bezahlt = orig.zahlbetrag;
    orig.speichern(db)?;
    storno.bezahlt = storno.zahlbetrag;
    storno.speichern(db)?;
    Ok(neu_id)
}

#[tauri::command]
pub fn beleg_stornieren(state: State<AppState>, id: i64) -> Res<BelegVoll> {
    state.mit_db(|db| {
        let neu_id = stornieren(db, id)?;
        voll_laden(db, neu_id)
    })
}

pub fn datum_de(iso: &str) -> String {
    let t: Vec<&str> = iso.split('-').collect();
    if t.len() == 3 { format!("{}.{}.{}", t[2], t[1], t[0]) } else { iso.to_string() }
}

/// Belege eines Projekts.
#[tauri::command]
pub fn belege_projekt(state: State<AppState>, projekt_id: i64) -> Res<Vec<BelegKurz>> {
    state.mit_db(|db| Ok(Beleg::alle(db, "WHERE projekt_id = ?1 ORDER BY datum, id", &[&projekt_id])?.iter().map(BelegKurz::from).collect()))
}

/// Abschlagsrechnungen des Projekts erneut als Abzüge übernehmen.
#[tauri::command]
pub fn abzuege_vorschlagen(state: State<AppState>, projekt_id: i64, beleg_id: i64) -> Res<Vec<AbschlagAbzug>> {
    state.mit_db(|db| abzuege_aus_projekt(db, projekt_id, beleg_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aufbau() -> (Connection, i64) {
        let db = db::test_db();
        let k = Kunde { name1: "Test".into(), ..Default::default() }.speichern(&db).unwrap();
        let p = Projekt { kunde_id: k, nummer: "1".into(), bezeichnung: "Fassade".into(), ..Default::default() }.speichern(&db).unwrap();
        db.execute("INSERT INTO nummernkreis (name, naechste) VALUES ('rechnung', 8298), ('angebot', 1371)", []).unwrap();
        (db, p)
    }

    fn mit_position(db: &Connection, projekt: i64, art: &str, ep: i64) -> i64 {
        let mut v = BelegVoll { beleg: neu(db, projekt, art).unwrap(), ..Default::default() };
        v.positionen.push(Position { art: "N".into(), menge: Decimal::ONE, ep: Decimal::from(ep), text: "Arbeit".into(), ..Default::default() });
        schreiben(db, &mut v).unwrap()
    }

    #[test]
    fn angebot_wird_rechnung_mit_nummer_erst_beim_festschreiben() {
        let (db, p) = aufbau();
        let angebot = mit_position(&db, p, "angebot", 1000);
        assert_eq!(Beleg::laden(&db, angebot).unwrap().nummer, "1371");
        let rechnung = umwandeln(&db, angebot, "rechnung", None).unwrap();
        let r = voll_laden(&db, rechnung).unwrap();
        assert_eq!(r.beleg.nummer, "");
        assert_eq!(r.beleg.zahlbetrag, Decimal::from(1190));
        assert_eq!(r.beleg.vorgaenger_id, Some(angebot));
        let fest = festschreiben(&db, rechnung).unwrap();
        assert_eq!((fest.nummer.as_str(), fest.status.as_str()), ("8298", "festgeschrieben"));
        let mut v = voll_laden(&db, rechnung).unwrap();
        assert!(schreiben(&db, &mut v).is_err());
    }

    #[test]
    fn schlussrechnung_zieht_abschlaege_ab() {
        let (db, p) = aufbau();
        let a1 = mit_position(&db, p, "abschlag", 1000);
        festschreiben(&db, a1).unwrap();
        assert_eq!(Beleg::laden(&db, a1).unwrap().titel, "Abschlagsrechnung 1");
        let a2 = neu(&db, p, "abschlag").unwrap();
        assert_eq!(a2.titel, "Abschlagsrechnung 2");
        let mut s = BelegVoll { beleg: neu(&db, p, "schluss").unwrap(), abzuege: abzuege_aus_projekt(&db, p, 0).unwrap(), ..Default::default() };
        s.positionen.push(Position { art: "N".into(), menge: Decimal::ONE, ep: Decimal::from(3000), ..Default::default() });
        let id = schreiben(&db, &mut s).unwrap();
        let v = voll_laden(&db, id).unwrap();
        assert_eq!(v.summen.gesamt, Decimal::from(3570));
        assert_eq!(v.summen.abzug, Decimal::from(1190));
        assert_eq!(v.beleg.zahlbetrag, Decimal::from(2380));
    }

    #[test]
    fn storno_negiert_und_sperrt() {
        let (db, p) = aufbau();
        let r = mit_position(&db, p, "rechnung", 500);
        assert!(stornieren(&db, r).is_err(), "Entwurf wird nicht storniert");
        festschreiben(&db, r).unwrap();
        let s = stornieren(&db, r).unwrap();
        let storno = voll_laden(&db, s).unwrap();
        assert_eq!(storno.beleg.zahlbetrag, Decimal::from(-595));
        assert_eq!(storno.beleg.nummer, "8299");
        assert_eq!(Beleg::laden(&db, r).unwrap().status, "storniert");
        assert!(stornieren(&db, r).is_err());
    }
}
