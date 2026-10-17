//! Daten für die Vorlagen zusammenstellen und PDFs erzeugen.

use super::{betrag, html_absaetze, menge, setzen, text_absaetze, Absatz};
use crate::belege::{self, arten, datum_de, BelegVoll};
use crate::db::{err, Datensatz, Res};
use crate::einstellungen::{self, Firma};
use crate::modelle::*;
use crate::state::AppState;
use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize)]
struct Rand {
    links: f64,
    rechts: f64,
    oben: f64,
    unten: f64,
}

#[derive(Serialize)]
struct Layout {
    rand: Rand,
    logo: Option<String>,
    fussbild: Option<String>,
    seitenleiste: String,
}

/// Briefpapier: Firma, Ränder und Bilder.
struct Papier {
    firma: Firma,
    layout: Layout,
    dateien: HashMap<String, Vec<u8>>,
}

fn f(d: Decimal) -> f64 {
    d.to_string().parse().unwrap_or(0.0)
}

fn papier(db: &Connection) -> Res<Papier> {
    let firma = einstellungen::firma(db)?;
    let a = einstellungen::allgemein(db)?;
    let mut dateien = HashMap::new();
    let mut bild = |name: &str| -> Option<String> {
        let r: Option<(String, Vec<u8>)> = rusqlite::OptionalExtension::optional(
            db.query_row("SELECT mime, daten FROM datei WHERE name = ?1", [name], |r| Ok((r.get(0)?, r.get(1)?))),
        )
        .ok()
        .flatten();
        r.map(|(mime, daten)| {
            let datei = format!("{name}.{}", if mime.contains("png") { "png" } else { "jpg" });
            dateien.insert(datei.clone(), daten);
            datei
        })
    };
    let logo = bild("logo");
    let fussbild = bild("fusszeile");
    Ok(Papier {
        layout: Layout {
            rand: Rand { links: f(a.rand_links), rechts: f(a.rand_rechts), oben: f(a.rand_oben), unten: f(a.rand_unten) },
            logo,
            fussbild,
            seitenleiste: firma.seitenleiste.clone(),
        },
        firma,
        dateien,
    })
}

#[derive(Serialize)]
struct PosDruck {
    art: String,
    posnr: String,
    kurz: String,
    lang: String,
    menge: String,
    einheit: String,
    ep: String,
    gp: String,
    rabatt: String,
}

#[derive(Serialize)]
struct SummeDruck {
    text: String,
    betrag: String,
    stark: bool,
    linie: bool,
}

#[derive(Serialize)]
struct BelegDruck<'a> {
    firma: &'a Firma,
    layout: &'a Layout,
    adresse: Vec<String>,
    info: Vec<(String, String)>,
    titel: String,
    folgekopf: String,
    betreff: String,
    leistung: String,
    vortext: Vec<Absatz>,
    nachtext: Vec<Absatz>,
    positionen: Vec<PosDruck>,
    preise: bool,
    summen: Vec<SummeDruck>,
    hinweise: Vec<Absatz>,
    zahlung: Vec<Absatz>,
    waehrung: String,
    unterschrift: bool,
    zugferd: bool,
}

fn kurz_lang(text: &str) -> (String, String) {
    let mut z = text.splitn(2, '\n');
    (z.next().unwrap_or("").to_string(), z.next().unwrap_or("").trim_end().to_string())
}

/// Zahlungsbedingung mit ersetzten Platzhaltern.
pub fn zahlungstext(b: &Beleg, waehrung: &str) -> String {
    if b.zahlungsbedingung.trim().is_empty() {
        return String::new();
    }
    let ziel = crate::journal::faellig_am(&b.datum, b.zahlungsziel_tage);
    let skontofrist = crate::journal::faellig_am(&b.datum, b.skonto_tage);
    let skonto = crate::rechnen::prozent(b.zahlbetrag, b.skonto_prozent);
    b.zahlungsbedingung
        .replace("#ZN#", &datum_de(&ziel))
        .replace("#ZS#", &datum_de(&skontofrist))
        .replace("#SK#", &super::zahl_de(b.skonto_prozent, 0, 2))
        .replace("#WE#", waehrung)
        .replace("#BN#", &betrag(b.zahlbetrag))
        .replace("#SB#", &betrag(skonto))
        .replace("#BS#", &betrag(b.zahlbetrag - skonto))
}

fn beleg_daten<'a>(db: &Connection, v: &BelegVoll, p: &'a Papier, zugferd: bool) -> Res<BelegDruck<'a>> {
    let b = &v.beleg;
    let s = &v.summen;
    let art = arten::art(&b.art);
    let w = if b.waehrung.is_empty() { "EUR".to_string() } else { b.waehrung.clone() };
    let nummer_text = if b.nummer.is_empty() { String::new() } else { format!(" Nr. {}", b.nummer) };
    let titel = if b.status == "entwurf" && art.forderung { format!("{} (Entwurf)", b.titel) } else { format!("{}{}", b.titel, nummer_text) };
    let mut info = vec![("Datum".to_string(), datum_de(&b.datum))];
    if !b.nummer.is_empty() {
        info.push((format!("{}-Nr.", if art.forderung { "Rechnungs" } else { art.titel }), b.nummer.clone()));
    }
    if let Some(k) = &v.kunde {
        if !k.nummer.is_empty() {
            info.push(("Kunden-Nr.".into(), k.nummer.clone()));
        }
        if b.netto_rechnung && !k.ustid.is_empty() {
            info.push(("Ihre USt-IdNr.".into(), k.ustid.clone()));
        }
    }
    if let Some(pr) = &v.projekt {
        if !pr.nummer.is_empty() {
            info.push(("Auftrag".into(), pr.nummer.clone()));
        }
    }
    if let Some(vg) = b.vorgaenger_id.and_then(|id| Beleg::laden(db, id).ok()) {
        if !vg.nummer.is_empty() && vg.art != b.art {
            info.push((format!("zu {}", arten::art(&vg.art).titel), format!("{} vom {}", vg.nummer, datum_de(&vg.datum))));
        }
    }

    let mut positionen = vec![];
    let titel_summen: HashMap<usize, Decimal> = s.titel.iter().cloned().collect();
    let zwischen: HashMap<usize, Decimal> = s.zwischen.iter().cloned().collect();
    let mut offener_titel: Option<(usize, String)> = None;
    let titel_ende = |positionen: &mut Vec<PosDruck>, t: &Option<(usize, String)>| {
        if let Some((i, name)) = t {
            if let Some(summe) = titel_summen.get(i) {
                positionen.push(PosDruck {
                    art: "TS".into(), posnr: String::new(), kurz: format!("Summe {}", name.trim_end_matches(':')), lang: String::new(),
                    menge: String::new(), einheit: String::new(), ep: String::new(), gp: betrag(*summe), rabatt: String::new(),
                });
            }
        }
    };
    for (i, pos) in v.positionen.iter().enumerate() {
        let (kurz, lang) = kurz_lang(&pos.text);
        match pos.art.as_str() {
            "T" | "G" => {
                if pos.art == "T" || offener_titel.is_some() {
                    titel_ende(&mut positionen, &offener_titel);
                    offener_titel = None;
                }
                if pos.art == "T" {
                    offener_titel = Some((i, kurz.clone()));
                }
            }
            "S" => {
                positionen.push(PosDruck {
                    art: "S".into(), posnr: String::new(), kurz: if kurz.is_empty() { "Zwischensumme".into() } else { kurz.clone() }, lang: String::new(),
                    menge: String::new(), einheit: String::new(), ep: String::new(), gp: zwischen.get(&i).map(|d| betrag(*d)).unwrap_or_default(), rabatt: String::new(),
                });
                continue;
            }
            _ => {}
        }
        let preis = matches!(pos.art.as_str(), "N" | "A" | "B");
        positionen.push(PosDruck {
            art: pos.art.clone(),
            posnr: pos.posnr.clone(),
            kurz,
            lang,
            menge: if preis || !pos.menge.is_zero() { menge(pos.menge) } else { String::new() },
            einheit: pos.einheit.clone(),
            ep: if preis { betrag(pos.ep) } else { String::new() },
            gp: if preis { betrag(pos.gp) } else { String::new() },
            rabatt: if pos.rabatt.is_zero() { String::new() } else { super::zahl_de(pos.rabatt, 0, 2) },
        });
    }
    titel_ende(&mut positionen, &offener_titel);

    let mut summen = vec![];
    let zeile = |text: String, d: Decimal, stark: bool, linie: bool| SummeDruck { text, betrag: betrag(d), stark, linie };
    let anpassung = !s.rabatt.is_zero() || !s.zuschlaege_netto.is_empty();
    if anpassung {
        summen.push(zeile("Summe Positionen".into(), s.positionen, false, false));
        if !s.rabatt.is_zero() {
            summen.push(zeile(format!("abzüglich {} {} %", b.rabatt_text, super::zahl_de(b.rabatt_prozent, 0, 2)), -s.rabatt, false, false));
        }
        for z in &s.zuschlaege_netto {
            summen.push(zeile(z.text.clone(), z.betrag, false, false));
        }
    }
    summen.push(zeile("Nettobetrag".into(), s.netto, false, anpassung));
    if !b.netto_rechnung {
        summen.push(zeile(format!("zuzüglich {} % MwSt.", super::zahl_de(s.mwst_satz, 0, 2)), s.mwst, false, false));
        summen.push(zeile(format!("Gesamtbetrag {w}"), s.brutto, true, true));
    }
    for z in &s.zuschlaege_brutto {
        summen.push(zeile(z.text.clone(), z.betrag, false, false));
    }
    if !s.zuschlaege_brutto.is_empty() {
        summen.push(zeile(format!("Endbetrag {w}"), s.gesamt, true, true));
    }
    for a in &v.abzuege {
        let mut t = format!("abzüglich {}", a.text);
        if !a.nummer.is_empty() {
            t.push_str(&format!(" Nr. {}", a.nummer));
        }
        if !a.datum.is_empty() {
            t.push_str(&format!(" vom {}", datum_de(&a.datum)));
        }
        summen.push(zeile(t, -a.brutto, false, false));
    }
    if !v.abzuege.is_empty() {
        summen.push(zeile(format!("Restbetrag {w}"), s.zahlbetrag, true, true));
    }

    let mut hinweise = vec![];
    if b.netto_rechnung && art.forderung {
        hinweise.extend(text_absaetze("Steuerschuldnerschaft des Leistungsempfängers (§ 13b UStG). Die Umsatzsteuer schuldet der Auftraggeber."));
    }
    if b.lohn_ausweisen && !s.lohn_netto.is_zero() {
        hinweise.extend(text_absaetze(&format!(
            "Im Rechnungsbetrag sind Arbeitskosten von {} {w} zuzüglich {} {w} MwSt. (zusammen {} {w}) enthalten (§ 35a EStG).",
            betrag(s.lohn_netto), betrag(s.lohn_mwst), betrag(s.lohn_netto + s.lohn_mwst)
        )));
    }
    let zahlung = if art.forderung && b.zahlbetrag > Decimal::ZERO { text_absaetze(&zahlungstext(b, &w)) } else { vec![] };

    Ok(BelegDruck {
        firma: &p.firma,
        layout: &p.layout,
        adresse: b.adresse.lines().map(|l| l.to_string()).collect(),
        info,
        folgekopf: format!("{}{} vom {}", b.titel, nummer_text, datum_de(&b.datum)),
        titel,
        betreff: b.betreff.clone(),
        leistung: b.leistung.clone(),
        vortext: html_absaetze(&b.vortext),
        nachtext: html_absaetze(&b.nachtext),
        positionen,
        preise: b.art != "lieferschein",
        summen,
        hinweise,
        zahlung,
        waehrung: w,
        unterschrift: b.art == "lieferschein",
        zugferd,
    })
}

/// PDF eines Belegs. Mit `e_rechnung` wird (wenn möglich) die ZUGFeRD-XML eingebettet.
pub fn beleg_pdf(db: &Connection, id: i64, e_rechnung: bool) -> Res<(Vec<u8>, String)> {
    let v = belege::voll_laden(db, id)?;
    let mut p = papier(db)?;
    let allg = einstellungen::allgemein(db)?;
    let zugferd = e_rechnung && allg.zugferd && v.beleg.status != "entwurf" && crate::zugferd::moeglich(&v, &p.firma).is_ok();
    let mut dateien = std::mem::take(&mut p.dateien);
    if zugferd {
        dateien.insert("factur-x.xml".into(), crate::zugferd::xml(&v, &p.firma).into_bytes());
    }
    let daten = beleg_daten(db, &v, &p, zugferd)?;
    let mut pdf = setzen("beleg.typ", &daten, dateien, zugferd)?;
    if zugferd {
        pdf = crate::zugferd::xmp_ergaenzen(pdf)?;
    }
    let kunde = v.kunde.as_ref().map(|k| k.anzeigename()).unwrap_or_default();
    Ok((pdf, format!("{} {} {}", v.beleg.titel, v.beleg.nummer, kunde)))
}

#[derive(Serialize)]
struct BriefDruck<'a> {
    firma: &'a Firma,
    layout: &'a Layout,
    adresse: Vec<String>,
    info: Vec<(String, String)>,
    folgekopf: String,
    betreff: String,
    text: Vec<Absatz>,
    gruss: String,
    unterschrift: String,
}

pub fn brief_pdf(db: &Connection, id: i64) -> Res<(Vec<u8>, String)> {
    let b = Brief::laden(db, id)?;
    let mut p = papier(db)?;
    let mut info = vec![("Datum".to_string(), datum_de(&b.datum))];
    if let Ok(k) = Kunde::laden(db, b.kunde_id) {
        if !k.nummer.is_empty() {
            info.push(("Kunden-Nr.".into(), k.nummer));
        }
    }
    let gruss_im_text = b.text.contains("freundlichen Grüßen") || b.text.contains("freundlichen Grüssen");
    let daten = BriefDruck {
        firma: &p.firma,
        layout: &p.layout,
        adresse: b.adresse.lines().map(|l| l.to_string()).collect(),
        info,
        folgekopf: format!("Schreiben vom {}", datum_de(&b.datum)),
        betreff: b.betreff.clone(),
        text: html_absaetze(&b.text),
        gruss: if gruss_im_text { String::new() } else { p.firma.gruss.clone() },
        unterschrift: if p.firma.unterschrift.is_empty() { p.firma.inhaber.clone() } else { p.firma.unterschrift.clone() },
    };
    let dateien = std::mem::take(&mut p.dateien);
    let pdf = setzen("brief.typ", &daten, dateien, false)?;
    Ok((pdf, format!("Brief {} {}", b.datum, b.betreff)))
}

#[derive(Serialize)]
struct Posten {
    beleg: String,
    datum: String,
    betrag: String,
    bezahlt: String,
    offen: String,
}

#[derive(Serialize)]
struct MahnDruck<'a> {
    firma: &'a Firma,
    layout: &'a Layout,
    adresse: Vec<String>,
    info: Vec<(String, String)>,
    titel: String,
    einleitung: Vec<Absatz>,
    schluss: Vec<Absatz>,
    posten: Vec<Posten>,
    gebuehr: String,
    summe: String,
    waehrung: String,
    gruss: String,
    unterschrift: String,
}

pub fn mahnung_pdf(db: &Connection, id: i64) -> Res<(Vec<u8>, String)> {
    let m = Mahnung::laden(db, id)?;
    let b = Beleg::laden(db, m.beleg_id)?;
    let mut p = papier(db)?;
    let allg = einstellungen::allgemein(db)?;
    let stufe = allg.mahnstufen.get(m.stufe as usize).cloned().unwrap_or_default();
    let ersetzen = |t: &str| t.replace("#ZN#", &datum_de(&m.frist));
    let offen = b.zahlbetrag - b.bezahlt;
    let mut info = vec![("Datum".to_string(), datum_de(&m.datum))];
    if let Ok(k) = Kunde::laden(db, b.kunde_id) {
        if !k.nummer.is_empty() {
            info.push(("Kunden-Nr.".into(), k.nummer));
        }
    }
    info.push(("Zahlbar bis".into(), datum_de(&m.frist)));
    let text_oder = |eigen: &str, vorgabe: &str| -> Vec<Absatz> {
        let t = ersetzen(if eigen.trim().is_empty() { vorgabe } else { eigen });
        if t.contains('<') { html_absaetze(&t) } else { html_absaetze(&crate::altdaten::rtf::text_zu_html(&t)) }
    };
    let daten = MahnDruck {
        firma: &p.firma,
        layout: &p.layout,
        adresse: b.adresse.lines().map(|l| l.to_string()).collect(),
        info,
        titel: if stufe.titel.is_empty() { format!("{}. Mahnung", m.stufe) } else { stufe.titel.clone() },
        einleitung: text_oder(&m.text, &stufe.einleitung),
        schluss: text_oder("", &stufe.schluss),
        posten: vec![Posten {
            beleg: format!("{} {}", b.titel, b.nummer),
            datum: datum_de(&b.datum),
            betrag: betrag(b.zahlbetrag),
            bezahlt: betrag(b.bezahlt),
            offen: betrag(offen),
        }],
        gebuehr: if m.gebuehr.is_zero() { String::new() } else { betrag(m.gebuehr) },
        summe: betrag(offen + m.gebuehr),
        waehrung: b.waehrung.clone(),
        gruss: p.firma.gruss.clone(),
        unterschrift: if p.firma.unterschrift.is_empty() { p.firma.inhaber.clone() } else { p.firma.unterschrift.clone() },
    };
    let dateien = std::mem::take(&mut p.dateien);
    let pdf = setzen("mahnung.typ", &daten, dateien, false)?;
    Ok((pdf, format!("{} {} {}", daten.titel, b.nummer, b.datum)))
}

#[derive(Serialize)]
struct AufmassZeileDruck {
    art: String,
    text: String,
    anzahl: String,
    formel: String,
    m1: String,
    m2: String,
    m3: String,
    ergebnis: String,
    abzug: bool,
}

#[derive(Serialize)]
struct AufmassPos {
    posnr: String,
    kurz: String,
    einheit: String,
    summe: String,
    zeilen: Vec<AufmassZeileDruck>,
}

#[derive(Serialize)]
struct AufmassDruck<'a> {
    firma: &'a Firma,
    layout: &'a Layout,
    adresse: Vec<String>,
    info: Vec<(String, String)>,
    titel: String,
    betreff: String,
    positionen: Vec<AufmassPos>,
}

pub fn aufmass_pdf(db: &Connection, beleg_id: i64) -> Res<(Vec<u8>, String)> {
    let v = belege::voll_laden(db, beleg_id)?;
    let zeilen = Aufmass::alle(db, "WHERE beleg_id = ?1 ORDER BY pos", &[&beleg_id])?;
    let formeln: HashMap<i64, String> = Formel::alle(db, "", &[])?.into_iter().map(|f| (f.nummer, f.name)).collect();
    let mut p = papier(db)?;
    let mut positionen: Vec<AufmassPos> = vec![];
    for pos in v.positionen.iter().filter(|p| p.art == "N" || p.art == "A" || p.art == "B") {
        let eigene: Vec<&Aufmass> = zeilen.iter().filter(|z| z.posnr == pos.posnr && !pos.posnr.is_empty()).collect();
        if eigene.is_empty() {
            continue;
        }
        let summe: Decimal = eigene.iter().filter(|z| z.art == "M").map(|z| z.ergebnis).sum();
        positionen.push(AufmassPos {
            posnr: pos.posnr.clone(),
            kurz: kurz_lang(&pos.text).0,
            einheit: pos.einheit.clone(),
            summe: menge(summe),
            zeilen: eigene
                .iter()
                .map(|z| AufmassZeileDruck {
                    art: z.art.clone(),
                    text: z.text.clone(),
                    anzahl: if z.art == "M" { super::zahl_de(z.anzahl, 0, 3) } else { String::new() },
                    formel: if z.formel > 0 { formeln.get(&z.formel).cloned().unwrap_or(z.formel.to_string()) } else { String::new() },
                    m1: z.m1.clone(),
                    m2: z.m2.clone(),
                    m3: z.m3.clone(),
                    ergebnis: if z.art == "M" { menge(z.ergebnis) } else { String::new() },
                    abzug: z.abzug,
                })
                .collect(),
        });
    }
    let titel = format!("Aufmaß zu {} {}", v.beleg.titel, v.beleg.nummer);
    let daten = AufmassDruck {
        firma: &p.firma,
        layout: &p.layout,
        adresse: v.beleg.adresse.lines().map(|l| l.to_string()).collect(),
        info: vec![("Datum".into(), datum_de(&v.beleg.datum)), ("Auftrag".into(), v.projekt.as_ref().map(|p| p.nummer.clone()).unwrap_or_default())],
        titel: titel.clone(),
        betreff: v.beleg.betreff.clone(),
        positionen,
    };
    let dateien = std::mem::take(&mut p.dateien);
    Ok((setzen("aufmass.typ", &daten, dateien, false)?, titel))
}

#[derive(Serialize)]
struct RegieZeileDruck {
    art: String,
    posnr: String,
    text: String,
    menge: String,
    einheit: String,
    ep: String,
    gp: String,
}

#[derive(Serialize)]
struct RegieBlock {
    datum: String,
    titel: String,
    zeilen: Vec<RegieZeileDruck>,
}

#[derive(Serialize)]
struct RegieDruck<'a> {
    firma: &'a Firma,
    layout: &'a Layout,
    adresse: Vec<String>,
    info: Vec<(String, String)>,
    titel: String,
    betreff: String,
    preise: bool,
    berichte: Vec<RegieBlock>,
}

/// Regieberichte drucken: einzelne (`ids`) oder alle eines Projekts.
pub fn regie_pdf(db: &Connection, projekt_id: i64, ids: &[i64], preise: bool) -> Res<(Vec<u8>, String)> {
    let projekt = Projekt::laden(db, projekt_id)?;
    let kunde = Kunde::laden(db, projekt.kunde_id)?;
    let mut berichte = Regie::alle(db, "WHERE projekt_id = ?1 ORDER BY datum, id", &[&projekt_id])?;
    if !ids.is_empty() {
        berichte.retain(|r| ids.contains(&r.id));
    }
    let mut p = papier(db)?;
    let mut bloecke = vec![];
    for r in &berichte {
        let zeilen = RegieZeile::alle(db, "WHERE regie_id = ?1 ORDER BY pos", &[&r.id])?;
        bloecke.push(RegieBlock {
            datum: datum_de(&r.datum),
            titel: r.titel.clone(),
            zeilen: zeilen
                .iter()
                .map(|z| RegieZeileDruck {
                    art: z.art.clone(),
                    posnr: z.posnr.clone(),
                    text: z.text.clone(),
                    menge: if z.art == "T" { String::new() } else { menge(z.menge) },
                    einheit: z.einheit.clone(),
                    ep: if z.ep.is_zero() { String::new() } else { betrag(z.ep) },
                    gp: if z.ep.is_zero() { String::new() } else { betrag(crate::rechnen::cent(z.menge * z.ep)) },
                })
                .collect(),
        });
    }
    let titel = format!("Regiebericht{} Auftrag {}", if berichte.len() > 1 { "e" } else { "" }, projekt.nummer);
    let daten = RegieDruck {
        firma: &p.firma,
        layout: &p.layout,
        adresse: kunde.anschrift().lines().map(|l| l.to_string()).collect(),
        info: vec![("Datum".into(), datum_de(&crate::db::heute())), ("Auftrag".into(), projekt.nummer.clone())],
        titel: titel.clone(),
        betreff: projekt.bezeichnung.clone(),
        preise,
        berichte: bloecke,
    };
    let dateien = std::mem::take(&mut p.dateien);
    Ok((setzen("regie.typ", &daten, dateien, false)?, titel))
}

/// Etiketten: je Etikett eine Liste von Zeilen.
pub fn etiketten_pdf(etiketten: Vec<Vec<String>>, spalten: u32, hoehe: f64) -> Res<Vec<u8>> {
    setzen("etiketten.typ", &serde_json::json!({ "etiketten": etiketten, "spalten": spalten.max(1), "hoehe": hoehe }), HashMap::new(), false)
}

fn dateiname(s: &str) -> String {
    let t: String = s.chars().map(|c| if r#"\/:*?"<>|"#.contains(c) || c.is_control() { '_' } else { c }).collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() { "Dokument".into() } else { t.chars().take(120).collect() }
}

pub fn pdf_ordner(app: &AppHandle, db: &Connection) -> PathBuf {
    let a = einstellungen::allgemein(db).unwrap_or_default();
    if !a.pdf_ordner.trim().is_empty() {
        return PathBuf::from(a.pdf_ordner.trim());
    }
    app.path().document_dir().unwrap_or_else(|_| std::env::temp_dir()).join("mrfakt")
}

fn erzeugen(db: &Connection, art: &str, id: i64, ids: &[i64]) -> Res<(Vec<u8>, String)> {
    match art {
        "beleg" => beleg_pdf(db, id, true),
        "brief" => brief_pdf(db, id),
        "mahnung" => mahnung_pdf(db, id),
        "aufmass" => aufmass_pdf(db, id),
        "regie" => regie_pdf(db, id, ids, false),
        "regie_preise" => regie_pdf(db, id, ids, true),
        _ => Err(format!("Unbekannte Druckart {art}")),
    }
}

/// PDF als Base64 für die Vorschau.
#[tauri::command]
pub async fn pdf_vorschau(app: AppHandle, art: String, id: i64, ids: Option<Vec<i64>>) -> Res<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let (pdf, _) = state.mit_db(|db| erzeugen(db, &art, id, &ids.unwrap_or_default()))?;
        Ok(einstellungen::base64(&pdf))
    })
    .await
    .map_err(err)?
}

/// PDF in den PDF-Ordner (Unterordner je Jahr) oder nach `pfad` speichern und den Pfad zurückgeben.
#[tauri::command]
pub async fn pdf_speichern(app: AppHandle, art: String, id: i64, ids: Option<Vec<i64>>, pfad: Option<String>, oeffnen: bool) -> Res<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let (ziel, pdf) = state.mit_db(|db| {
            let (pdf, name) = erzeugen(db, &art, id, &ids.unwrap_or_default())?;
            let ziel = match pfad {
                Some(p) if !p.is_empty() => PathBuf::from(p),
                _ => {
                    let ordner = pdf_ordner(&app, db).join(chrono::Local::now().format("%Y").to_string());
                    ordner.join(format!("{}.pdf", dateiname(&name)))
                }
            };
            Ok((ziel, pdf))
        })?;
        if let Some(o) = ziel.parent() {
            std::fs::create_dir_all(o).map_err(err)?;
        }
        std::fs::write(&ziel, pdf).map_err(|e| format!("{}: {e}", ziel.display()))?;
        if oeffnen {
            use tauri_plugin_opener::OpenerExt;
            app.opener().open_path(ziel.to_string_lossy(), None::<&str>).map_err(err)?;
        }
        Ok(ziel.to_string_lossy().into_owned())
    })
    .await
    .map_err(err)?
}

/// E-Rechnung (XRechnung/ZUGFeRD-XML) eines Belegs als Datei speichern.
#[tauri::command]
pub fn erechnung_xml(state: State<AppState>, id: i64, pfad: String) -> Res<()> {
    state.mit_db(|db| {
        let v = belege::voll_laden(db, id)?;
        let firma = einstellungen::firma(db)?;
        crate::zugferd::moeglich(&v, &firma)?;
        std::fs::write(&pfad, crate::zugferd::xml(&v, &firma)).map_err(err)
    })
}

#[tauri::command]
pub fn datei_oeffnen(app: AppHandle, pfad: String) -> Res<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_path(pfad, None::<&str>).map_err(err)
}

#[tauri::command]
pub fn ordner_zeigen(app: AppHandle, pfad: String) -> Res<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().reveal_item_in_dir(pfad).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belege::{festschreiben, neu, schreiben};

    #[test]
    fn rechnung_als_pdf_mit_zugferd() {
        let db = crate::db::test_db();
        crate::db::einstellung_setzen(&db, "firma", &Firma { name: "Malergeschäft Seydt".into(), strasse: "Schillerstr. 26".into(), plz: "75387".into(), ort: "Neubulach".into(), steuernummer: "28 45245/32392".into(), ..Default::default() }).unwrap();
        let k = Kunde { anrede: "Herrn".into(), name1: "Manfred Glinka".into(), strasse: "Hölderlinstr. 3".into(), plz: "75387".into(), ort: "Neubulach".into(), ..Default::default() }.speichern(&db).unwrap();
        let p = Projekt { kunde_id: k, nummer: "18155".into(), bezeichnung: "Wohnhaus und Garage".into(), ..Default::default() }.speichern(&db).unwrap();
        let mut v = BelegVoll { beleg: neu(&db, p, "rechnung").unwrap(), ..Default::default() };
        v.beleg.vortext = "<p>für <strong>Malerarbeiten</strong> außen.</p>".into();
        v.beleg.lohn_ausweisen = true;
        v.positionen = vec![
            Position { art: "T".into(), text: "Aussen:".into(), ..Default::default() },
            Position { art: "N".into(), posnr: "1".into(), text: "Arbeitsgerüst, incl. Auf- und Abbau.".into(), menge: Decimal::from(265), einheit: "m²".into(), ep: Decimal::new(885, 2), lohn: Decimal::from(5), ..Default::default() },
            Position { art: "N".into(), posnr: "2".into(), text: "Holzwerk\nabwaschen, schleifen".into(), menge: Decimal::from(157), einheit: "m²".into(), ep: Decimal::new(3110, 2), ..Default::default() },
            Position { art: "A".into(), posnr: "2a".into(), text: "Alternativ".into(), menge: Decimal::ONE, einheit: "St.".into(), ep: Decimal::from(10), ..Default::default() },
        ];
        let id = schreiben(&db, &mut v).unwrap();
        let (entwurf, _) = beleg_pdf(&db, id, true).unwrap();
        assert!(entwurf.starts_with(b"%PDF"));
        festschreiben(&db, id).unwrap();
        let (pdf, name) = beleg_pdf(&db, id, true).unwrap();
        assert!(name.starts_with("Rechnung 1 Manfred Glinka"));
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("factur-x.xml"), "XML eingebettet");
        assert!(text.contains("fx:ConformanceLevel"), "XMP ergänzt");
        if let Ok(ziel) = std::env::var("MRFAKT_PDF") {
            std::fs::write(ziel, &pdf).unwrap();
        }
    }
}
