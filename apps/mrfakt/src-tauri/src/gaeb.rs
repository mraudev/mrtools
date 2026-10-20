//! GAEB: Leistungsverzeichnisse einlesen (GAEB DA XML X81/X83/X84/X86, GAEB 90 D81/D83)
//! und Angebote als GAEB DA XML X84 (Angebotsabgabe) ausgeben.

use crate::belege::{self, BelegVoll};
use crate::db::{err, Res};
use crate::modelle::Position;
use crate::state::AppState;
use quick_xml::events::Event;
use quick_xml::Reader;
use rust_decimal::Decimal;
use serde::Serialize;
use std::str::FromStr;
use tauri::State;

#[derive(Debug, Default, Serialize, Clone)]
pub struct Lv {
    pub bezeichnung: String,
    pub positionen: Vec<Position>,
}

fn dez(s: &str) -> Decimal {
    let s = s.trim();
    let s = if s.contains(',') { s.replace('.', "").replace(',', ".") } else { s.to_string() };
    Decimal::from_str(&s).unwrap_or_default()
}

/// GAEB DA XML (alle Versionen ab 3.0): BoQBody → BoQCtgy (Titel) / Itemlist → Item.
pub fn xml_lesen(text: &str) -> Result<Lv, String> {
    let mut r = Reader::from_str(text);
    r.config_mut().trim_text(false);
    let mut lv = Lv::default();
    let mut pfad: Vec<String> = vec![];
    let mut nummern: Vec<String> = vec![]; // RNoPart je Ebene
    let mut aktuell: Option<Position> = None;
    let mut titel: Option<String> = None;
    let mut kurz = String::new();
    let mut lang: Vec<String> = vec![];
    let mut zeile = String::new();
    let mut buf = vec![];
    loop {
        match r.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_string();
                match name.as_str() {
                    "BoQCtgy" => {
                        let rno = e.attributes().flatten().find(|a| a.key.local_name().as_ref() == "RNoPart").map(|a| a.value.to_string()).unwrap_or_default();
                        nummern.push(rno);
                        titel = Some(String::new());
                    }
                    "Item" => {
                        let rno = e.attributes().flatten().find(|a| a.key.local_name().as_ref() == "RNoPart").map(|a| a.value.to_string()).unwrap_or_default();
                        let mut nr = nummern.clone();
                        nr.push(rno);
                        aktuell = Some(Position { art: "N".into(), posnr: nr.iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join("."), ..Default::default() });
                        kurz.clear();
                        lang.clear();
                    }
                    "p" | "span" | "br" => {}
                    _ => {}
                }
                pfad.push(name);
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_string();
                if name == "br" && pfad.iter().any(|p| p == "DetailTxt") {
                    lang.push(std::mem::take(&mut zeile));
                }
            }
            Ok(Event::End(e)) => {
                let name = e.local_name().as_ref().to_string();
                pfad.pop();
                match name.as_str() {
                    "p" if pfad.iter().any(|p| p == "DetailTxt") => lang.push(std::mem::take(&mut zeile)),
                    "LblTx" if titel.is_some() && aktuell.is_none() => {
                        let t = std::mem::take(&mut zeile);
                        if let Some(ti) = titel.as_mut() {
                            if ti.is_empty() {
                                *ti = t.trim().to_string();
                                lv.positionen.push(Position { art: "T".into(), posnr: nummern.iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join("."), text: ti.clone(), ..Default::default() });
                            }
                        }
                    }
                    "BoQCtgy" => {
                        nummern.pop();
                        titel = None;
                    }
                    "Item" => {
                        if let Some(mut p) = aktuell.take() {
                            let lang_text = lang.iter().map(|s| s.trim().to_string()).collect::<Vec<_>>();
                            let mut zeilen = lang_text;
                            while zeilen.last().is_some_and(|s| s.is_empty()) {
                                zeilen.pop();
                            }
                            let kopf = if kurz.trim().is_empty() { zeilen.first().cloned().unwrap_or_default() } else { kurz.trim().to_string() };
                            let rest: Vec<String> = if kurz.trim().is_empty() && !zeilen.is_empty() { zeilen[1..].to_vec() } else { zeilen };
                            p.text = std::iter::once(kopf).chain(rest).collect::<Vec<_>>().join("\n").trim_end().to_string();
                            lv.positionen.push(p);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                let s = t.html_content().to_string();
                let oben = pfad.last().map(String::as_str).unwrap_or("");
                if pfad.iter().any(|p| p == "Prj" || p == "PrjInfo") && oben == "LblPrj" {
                    lv.bezeichnung.push_str(s.trim());
                } else if let Some(p) = aktuell.as_mut() {
                    match oben {
                        "Qty" => p.menge = dez(&s),
                        "QU" => p.einheit = s.trim().to_string(),
                        "UP" => p.ep = dez(&s),
                        _ if pfad.iter().any(|x| x == "OutlineText" || x == "TextOutlTxt") => kurz.push_str(&s),
                        _ if pfad.iter().any(|x| x == "DetailTxt") => zeile.push_str(&s),
                        _ => {}
                    }
                } else if titel.is_some() && pfad.iter().any(|x| x == "LblTx") {
                    zeile.push_str(&s);
                }
            }
            Ok(Event::GeneralRef(e)) => {
                let ent = e.into_inner().to_string();
                let zeichen = match ent.as_str() {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    n if n.starts_with("#x") => u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32),
                    n if n.starts_with('#') => n[1..].parse().ok().and_then(char::from_u32),
                    _ => None,
                };
                let c = zeichen.map(|c| c.to_string()).unwrap_or_default();
                let c = c.as_str();
                if pfad.iter().any(|x| x == "DetailTxt" || x == "LblTx") { zeile.push_str(c) } else if pfad.iter().any(|x| x == "OutlineText") { kurz.push_str(c) }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("GAEB-XML nicht lesbar (Zeile {}): {e}", r.buffer_position())),
            _ => {}
        }
        buf.clear();
    }
    if lv.positionen.iter().all(|p| p.art != "N") {
        return Err("Keine Positionen gefunden – ist das eine GAEB-DA-XML-Datei?".into());
    }
    Ok(lv)
}

/// GAEB 90 (D81/D83): Satzarten in Spalte 1–2 (11/12 Bereich, 21 Position, 25 Kurztext, 26 Langtext).
pub fn g90_lesen(text: &str) -> Result<Lv, String> {
    let mut lv = Lv::default();
    let mut aktuell: Option<Position> = None;
    let feld = |z: &str, a: usize, b: usize| z.chars().skip(a).take(b.saturating_sub(a)).collect::<String>().trim().to_string();
    let fertig = |lv: &mut Lv, p: &mut Option<Position>| {
        if let Some(p) = p.take() {
            lv.positionen.push(p);
        }
    };
    for z in text.lines() {
        let satz = feld(z, 0, 2);
        match satz.as_str() {
            "02" | "03" => {
                if lv.bezeichnung.is_empty() {
                    lv.bezeichnung = feld(z, 2, 74);
                }
            }
            "11" => {
                fertig(&mut lv, &mut aktuell);
                lv.positionen.push(Position { art: "T".into(), posnr: feld(z, 2, 11).trim_matches('0').to_string(), ..Default::default() });
            }
            "12" => {
                if let Some(t) = lv.positionen.last_mut().filter(|p| p.art == "T") {
                    if !t.text.is_empty() { t.text.push(' '); }
                    t.text.push_str(&feld(z, 2, 74));
                }
            }
            "21" => {
                fertig(&mut lv, &mut aktuell);
                let oz = feld(z, 2, 11);
                let menge_roh: String = feld(z, 23, 34).chars().filter(|c| c.is_ascii_digit()).collect();
                let menge = Decimal::from_str(&menge_roh).map(|m| m / Decimal::from(1000)).unwrap_or_default();
                aktuell = Some(Position { art: "N".into(), posnr: oz, menge, einheit: feld(z, 34, 38), ..Default::default() });
            }
            "25" | "26" => {
                if let Some(p) = aktuell.as_mut() {
                    let t = feld(z, 2, 74);
                    if !p.text.is_empty() { p.text.push('\n'); }
                    p.text.push_str(&t);
                }
            }
            _ => {}
        }
    }
    fertig(&mut lv, &mut aktuell);
    if lv.positionen.iter().all(|p| p.art != "N") {
        return Err("Keine Positionen gefunden – ist das eine GAEB-90-Datei?".into());
    }
    Ok(lv)
}

pub fn lesen(pfad: &str) -> Res<Lv> {
    let bytes = std::fs::read(pfad).map_err(|e| format!("{pfad}: {e}"))?;
    let text = match String::from_utf8(bytes.clone()) {
        Ok(t) => t,
        Err(_) => encoding_rs::WINDOWS_1252.decode(&bytes).0.into_owned(),
    };
    if text.trim_start().starts_with('<') || text.contains("<GAEB") {
        xml_lesen(&text)
    } else {
        g90_lesen(&text)
    }
}

/// Liest ein LV und legt daraus ein Angebot im Projekt an.
#[tauri::command]
pub fn gaeb_import(state: State<AppState>, projekt_id: i64, pfad: String) -> Res<i64> {
    let lv = lesen(&pfad)?;
    state.mit_db(|db| {
        let mut b = belege::neu(db, projekt_id, "angebot")?;
        if !lv.bezeichnung.is_empty() {
            b.betreff = lv.bezeichnung.clone();
        }
        let mut v = BelegVoll { beleg: b, positionen: lv.positionen, ..Default::default() };
        belege::schreiben(db, &mut v)
    })
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// GAEB DA XML 3.3, Datenart 84 (Angebotsabgabe) mit Einheits- und Gesamtpreisen.
pub fn x84(v: &BelegVoll, firma: &crate::einstellungen::Firma) -> String {
    let mut x = String::new();
    x.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA84/3.3">
<GAEBInfo><Version>3.3</Version><VersDate>2021-05</VersDate><ProgSystem>mrfakt</ProgSystem></GAEBInfo>
"#);
    x.push_str(&format!(
        "<PrjInfo><NamePrj>{}</NamePrj><LblPrj>{}</LblPrj><Cur>EUR</Cur><CurLbl>Euro</CurLbl></PrjInfo>\n<Award><DP>84</DP><CTR><Address><Name1>{}</Name1><Street>{}</Street><PCode>{}</PCode><City>{}</City></Address></CTR>\n<BoQ><BoQInfo><Name>{}</Name><LblTx>{}</LblTx></BoQInfo><BoQBody>\n",
        esc(&v.projekt.as_ref().map(|p| p.nummer.clone()).unwrap_or_default()),
        esc(&v.beleg.betreff),
        esc(&firma.name), esc(&firma.strasse), esc(&firma.plz), esc(&firma.ort),
        esc(&v.beleg.nummer),
        esc(&v.beleg.titel)
    ));
    let mut offen_titel = false;
    let mut liste_offen = false;
    for p in &v.positionen {
        match p.art.as_str() {
            "T" | "G" => {
                if liste_offen { x.push_str("</Itemlist>"); liste_offen = false; }
                if offen_titel { x.push_str("</BoQBody></BoQCtgy>\n"); }
                x.push_str(&format!("<BoQCtgy RNoPart=\"{}\"><LblTx><p>{}</p></LblTx><BoQBody>", esc(&p.posnr), esc(p.text.lines().next().unwrap_or(""))));
                offen_titel = true;
            }
            "N" | "A" | "B" => {
                if !liste_offen { x.push_str("<Itemlist>"); liste_offen = true; }
                let mut zeilen = p.text.lines();
                let kurz = zeilen.next().unwrap_or("");
                let lang: String = zeilen.map(|l| format!("<p>{}</p>", esc(l))).collect();
                let rno = p.posnr.rsplit('.').next().unwrap_or(&p.posnr);
                x.push_str(&format!(
                    "<Item RNoPart=\"{}\">{}<Qty>{}</Qty><QU>{}</QU><UP>{}</UP><IT>{}</IT><Description><CompleteText><DetailTxt><Text><p>{}</p>{lang}</Text></DetailTxt><OutlineText><OutlTxt><TextOutlTxt><p>{}</p></TextOutlTxt></OutlTxt></OutlineText></CompleteText></Description></Item>\n",
                    esc(rno),
                    match p.art.as_str() { "A" => "<ALNGroupNo>1</ALNGroupNo><ALNSerNo>1</ALNSerNo>", "B" => "<Provis>WithoutTotal</Provis>", _ => "" },
                    p.menge.round_dp(3).normalize(),
                    esc(&p.einheit),
                    crate::rechnen::cent(p.ep),
                    crate::rechnen::cent(p.gp),
                    esc(kurz),
                    esc(kurz)
                ));
            }
            _ => {}
        }
    }
    if liste_offen { x.push_str("</Itemlist>"); }
    if offen_titel { x.push_str("</BoQBody></BoQCtgy>\n"); }
    x.push_str("</BoQBody></BoQ></Award>\n</GAEB>\n");
    x
}

#[tauri::command]
pub fn gaeb_export(state: State<AppState>, beleg_id: i64, pfad: String) -> Res<()> {
    state.mit_db(|db| {
        let v = belege::voll_laden(db, beleg_id)?;
        let firma = crate::einstellungen::firma(db)?;
        std::fs::write(&pfad, x84(&v, &firma)).map_err(err)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const X83: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.2"><PrjInfo><NamePrj>17</NamePrj><LblPrj>Sanierung Rathaus</LblPrj></PrjInfo>
<Award><BoQ><BoQBody>
<BoQCtgy RNoPart="01"><LblTx><p>Malerarbeiten</p></LblTx><BoQBody><Itemlist>
<Item RNoPart="0010"><Qty>125,500</Qty><QU>m2</QU><Description><CompleteText><DetailTxt><Text><p>Wandflächen</p><p>zweimal streichen &amp; spachteln</p></Text></DetailTxt><OutlineText><OutlTxt><TextOutlTxt><p>Wände streichen</p></TextOutlTxt></OutlTxt></OutlineText></CompleteText></Description></Item>
</Itemlist></BoQBody></BoQCtgy></BoQBody></BoQ></Award></GAEB>"#;

    #[test]
    fn x83_einlesen_und_x84_schreiben() {
        let lv = xml_lesen(X83).unwrap();
        assert_eq!(lv.bezeichnung, "Sanierung Rathaus");
        assert_eq!(lv.positionen.len(), 2);
        assert_eq!(lv.positionen[0].art, "T");
        assert_eq!(lv.positionen[0].text, "Malerarbeiten");
        let p = &lv.positionen[1];
        assert_eq!(p.posnr, "01.0010");
        assert_eq!(p.menge, Decimal::new(1255, 1));
        assert_eq!(p.einheit, "m2");
        assert_eq!(p.text, "Wände streichen\nWandflächen\nzweimal streichen & spachteln");
        let mut v = BelegVoll { positionen: lv.positionen, ..Default::default() };
        v.positionen[1].ep = Decimal::from(10);
        v.positionen[1].gp = Decimal::from(1255);
        let x = x84(&v, &Default::default());
        assert!(x.contains("<Item RNoPart=\"0010\">"));
        assert!(x.contains("<IT>1255</IT>") || x.contains("<IT>1255.00</IT>"));
        assert!(xml_lesen(&x).is_ok());
    }

    #[test]
    fn gaeb90_einlesen() {
        let mut t = String::new();
        t.push_str(&format!("{:<80}\n", "11000001"));
        t.push_str(&format!("{:<80}\n", "12Malerarbeiten"));
        t.push_str(&format!("{:<80}\n", format!("21{:<9}{:<12}{:0>11}{:<4}", "000010", "", "00000125500", "m2")));
        t.push_str(&format!("{:<80}\n", "25Wände streichen"));
        let lv = g90_lesen(&t).unwrap();
        let p = lv.positionen.iter().find(|p| p.art == "N").unwrap();
        assert_eq!(p.menge, Decimal::new(1255, 1));
        assert_eq!(p.einheit, "m2");
        assert_eq!(p.text, "Wände streichen");
    }
}
