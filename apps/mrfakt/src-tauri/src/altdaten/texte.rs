//! Parser für die zeilenbasierten Textdateien von GHFakt (Windows-1252).
//!
//! Je Beleg (Auftragsnummer + Endung .051–.060 = „LV-Slot“) gibt es:
//! - `VP` Vorspann: Anschrift, Datum, Belegart, Nummern, Vortext (RTF oder Klartext ab Zeile 22)
//! - `LV` Positionen: 24 Kopfzeilen, danach Sätze zu je 9 Zeilen
//!   (Art, PosNr, Text, Menge, Einheit, EP, Rabatt %, GP, leer); `F` = Folgezeile des vorigen Satzes
//! - `NPN` Nachspann: Rabatt, MwSt, Skonto, sechs Zu-/Abschlagzeilen, Nachtext ab Zeile 109
//! - `NP` älterer Nachspann: Text ab Zeile 23
//! - `EZ`/`AZ` abgezogene Abschlagszahlungen (Blöcke ab „AZn“)
//! - `AU` Aufmaß: wie LV, Sätze zu je 9 Zeilen (N = Bezug auf Position, M = Maßzeile, K = Kommentar,
//!   1/2 = Rechenweg der vorigen Maßzeile)
//! - `AUFTRAGI.DAT` im Kundenordner: Projektliste, feste Satzlänge 100

use rust_decimal::Decimal;
use std::str::FromStr;

pub fn dekodieren(bytes: &[u8]) -> String {
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    text.replace('\r', "")
}

pub fn zeilen(text: &str) -> Vec<&str> {
    text.split('\n').collect()
}

fn z<'a>(l: &[&'a str], i: usize) -> &'a str {
    l.get(i).copied().unwrap_or("").trim()
}

/// Zahl im GHFakt-Format: „1.234,56“, „0,00“ oder „7.00“.
pub fn zahl(s: &str) -> Decimal {
    let s = s.trim().replace(' ', "");
    if s.is_empty() {
        return Decimal::ZERO;
    }
    let s = if s.contains(',') { s.replace('.', "").replace(',', ".") } else { s };
    Decimal::from_str(&s).unwrap_or_default()
}

/// „21.06.2024“ → „2024-06-21“ (leer bei ungültigem Datum).
pub fn datum(s: &str) -> String {
    let t: Vec<&str> = s.trim().split('.').collect();
    if t.len() != 3 {
        return String::new();
    }
    let (Ok(d), Ok(m), Ok(mut y)) = (t[0].trim().parse::<u32>(), t[1].trim().parse::<u32>(), t[2].trim().parse::<i32>()) else {
        return String::new();
    };
    if y < 100 {
        y += if y < 50 { 2000 } else { 1900 };
    }
    match chrono::NaiveDate::from_ymd_opt(y, m, d) {
        Some(dt) => dt.format("%Y-%m-%d").to_string(),
        None => String::new(),
    }
}

/// Excel-/VB-Seriennummer (Tage seit 30.12.1899) → „JJJJ-MM-TT“.
pub fn seriendatum(n: i64) -> String {
    if n <= 0 {
        return String::new();
    }
    let basis = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
    (basis + chrono::Duration::days(n)).format("%Y-%m-%d").to_string()
}

#[derive(Debug, Default, Clone)]
pub struct Vorspann {
    pub adresse: Vec<String>,
    pub datum: String,
    pub belegart: String,
    pub nummer: String,
    pub vortext: String,
}

pub fn vorspann(text: &str) -> Vorspann {
    let l = zeilen(text);
    // Zeile 3 ist Ortsteil, 5/6 PLZ und Ort – für die Anschrift zusammenfassen.
    let ort = format!("{} {}", z(&l, 5), z(&l, 6)).trim().to_string();
    let mut adresse: Vec<String> = [z(&l, 0), z(&l, 1), z(&l, 2), z(&l, 4)].iter().map(|s| s.to_string()).collect();
    adresse.push(ort);
    let adresse = adresse.into_iter().filter(|s| !s.is_empty()).collect();
    let rest = if l.len() > 21 { l[21..].join("\n") } else { String::new() };
    Vorspann {
        adresse,
        datum: datum(z(&l, 9)),
        belegart: z(&l, 10).to_string(),
        nummer: z(&l, 12).to_string(),
        vortext: super::rtf::rtf_zu_html(rest.trim()),
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AltPosition {
    pub art: String,
    pub posnr: String,
    pub text: String,
    pub menge: Decimal,
    pub einheit: String,
    pub ep: Decimal,
    pub rabatt: Decimal,
    pub gp: Decimal,
}

#[derive(Debug, Default)]
pub struct Lv {
    pub waehrung: String,
    pub positionen: Vec<AltPosition>,
}

pub fn lv(text: &str) -> Lv {
    let l = zeilen(text);
    let mut lv = Lv { waehrung: z(&l, 5).to_string(), positionen: vec![] };
    let mut i = 24;
    while i < l.len() && !z(&l, i).is_empty() {
        let art = z(&l, i).to_string();
        let satz_text = l.get(i + 2).copied().unwrap_or("").trim_end().to_string();
        if art == "F" {
            match lv.positionen.last_mut() {
                Some(p) => {
                    p.text.push('\n');
                    p.text.push_str(&satz_text);
                }
                None => lv.positionen.push(AltPosition { art: "K".into(), text: satz_text, ..Default::default() }),
            }
        } else {
            lv.positionen.push(AltPosition {
                art,
                posnr: z(&l, i + 1).to_string(),
                text: satz_text,
                menge: zahl(z(&l, i + 3)),
                einheit: z(&l, i + 4).to_string(),
                ep: zahl(z(&l, i + 5)),
                rabatt: zahl(z(&l, i + 6)),
                gp: zahl(z(&l, i + 7)),
            });
        }
        i += 9;
    }
    for p in &mut lv.positionen {
        // Leere Folgezeilen am Ende abschneiden; „•“ markierte in GHFakt Hervorhebungen.
        p.text = p.text.replace('•', "").trim_end().to_string();
    }
    lv
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AltZuschlag {
    pub text: String,
    pub wert: Decimal,
    /// 0 % auf Netto, 1 Betrag auf Netto, 2 % auf Brutto, 3 Betrag auf Brutto
    pub modus: i64,
}

#[derive(Debug, Default)]
pub struct Nachspann {
    pub rabatt_text: String,
    pub rabatt: Decimal,
    pub mwst: Option<Decimal>,
    pub skonto: Decimal,
    pub skonto_tage: i64,
    pub ziel_tage: i64,
    pub zuschlaege: Vec<AltZuschlag>,
    pub text: String,
}

pub fn nachspann(text: &str) -> Nachspann {
    let l = zeilen(text);
    let mut n = Nachspann {
        rabatt_text: z(&l, 0).to_string(),
        rabatt: zahl(z(&l, 7)),
        mwst: if z(&l, 8).is_empty() { None } else { Some(zahl(z(&l, 8))) },
        skonto: zahl(z(&l, 10)),
        skonto_tage: z(&l, 13).parse().unwrap_or(0),
        ziel_tage: z(&l, 14).parse().unwrap_or(0),
        ..Default::default()
    };
    for k in 0..6 {
        let j = 15 + 7 * k;
        let wert = zahl(z(&l, j + 1));
        if !wert.is_zero() {
            n.zuschlaege.push(AltZuschlag { text: z(&l, j).to_string(), wert, modus: z(&l, j + 2).parse().unwrap_or(0) });
        }
    }
    if l.len() > 108 {
        n.text = super::rtf::rtf_zu_html(l[108..].join("\n").trim());
    }
    n
}

/// Älterer Nachspann (`NP`): nur der Text ab Zeile 23 ist von Belang.
pub fn nachspann_alt(text: &str) -> String {
    let l = zeilen(text);
    if l.len() <= 22 {
        return String::new();
    }
    super::rtf::rtf_zu_html(l[22..].join("\n").trim())
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AltAbschlag {
    pub text: String,
    pub nummer: String,
    pub datum: String,
    pub netto: Decimal,
    pub mwst: Decimal,
    pub brutto: Decimal,
}

/// `EZ`/`AZ`: Blöcke zu 14 Zeilen: Bezeichnung („AZ1“ oder „Abschlagsrechnung 1“), Rechnungsnummer,
/// Brutto, MwSt-Satz, –, Datum, –, Netto, MwSt, Brutto, … Leere Blöcke haben Brutto 0.
pub fn abschlaege(text: &str) -> Vec<AltAbschlag> {
    let l = zeilen(text);
    let mut out = vec![];
    for i in (0..l.len()).step_by(14) {
        let brutto = zahl(z(&l, i + 9));
        let brutto = if brutto.is_zero() { zahl(z(&l, i + 2)) } else { brutto };
        if brutto.is_zero() {
            continue;
        }
        let kopf = z(&l, i);
        let text = match kopf.strip_prefix("AZ") {
            Some(n) if n.chars().all(|c| c.is_ascii_digit()) => format!("Abschlagsrechnung {n}"),
            _ if kopf.is_empty() => format!("Abschlagsrechnung {}", out.len() + 1),
            _ => kopf.to_string(),
        };
        out.push(AltAbschlag {
            text,
            nummer: z(&l, i + 1).to_string(),
            datum: datum(z(&l, i + 5)),
            netto: zahl(z(&l, i + 7)),
            mwst: zahl(z(&l, i + 8)),
            brutto,
        });
    }
    out
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AltAufmass {
    pub posnr: String,
    pub art: String,
    pub text: String,
    pub anzahl: Decimal,
    pub formel: i64,
    pub m1: String,
    pub m2: String,
    pub m3: String,
    pub ergebnis: Decimal,
}

pub fn aufmass(text: &str) -> Vec<AltAufmass> {
    let l = zeilen(text);
    let Some(mut i) = (1..l.len().min(80)).find(|&k| matches!(z(&l, k), "N" | "M" | "K")) else {
        return vec![];
    };
    let mut out = vec![];
    let mut posnr = String::new();
    while i < l.len() && !z(&l, i).is_empty() {
        let art = z(&l, i);
        let text = z(&l, i + 1).to_string();
        match art {
            "N" => posnr = z(&l, i + 4).to_string(),
            "M" | "K" | "G" => out.push(AltAufmass {
                posnr: posnr.clone(),
                art: if art == "M" { "M".into() } else { "K".into() },
                text,
                anzahl: zahl(z(&l, i + 2)),
                formel: z(&l, i + 3).parse().unwrap_or(0),
                m1: z(&l, i + 4).to_string(),
                m2: z(&l, i + 5).to_string(),
                m3: z(&l, i + 6).to_string(),
                ergebnis: zahl(z(&l, i + 7)),
            }),
            // Rechenweg (z. B. „12+12+9“) zur vorigen Maßzeile
            _ if !text.is_empty() => out.push(AltAufmass { posnr: posnr.clone(), art: "R".into(), text, ..Default::default() }),
            _ => {}
        }
        i += 9;
    }
    out
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AltProjekt {
    pub nummer: String,
    pub datum: String,
    pub bezeichnung: String,
}

/// `AUFTRAGI.DAT`: Sätze zu 100 Zeichen: lfd. Nr (4), 2, Auftrag (5), 2, Datum (10), 2, Bezeichnung (50), …
pub fn projekte(text: &str) -> Vec<AltProjekt> {
    let zeichen: Vec<char> = text.chars().filter(|c| *c != '\n').collect();
    zeichen
        .chunks(100)
        .filter_map(|satz| {
            let s: String = satz.iter().collect();
            let feld = |a: usize, b: usize| s.chars().skip(a).take(b - a).collect::<String>().trim().to_string();
            let nummer = feld(6, 11);
            if nummer.is_empty() || !nummer.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            Some(AltProjekt { nummer, datum: datum(&feld(13, 23)), bezeichnung: feld(25, 75) })
        })
        .collect()
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AltBrief {
    pub titel: String,
    pub adresse: Vec<String>,
    pub datum: String,
    pub text: String,
}

/// Briefe (`00000GHT.0xx`): Bezeichnung (Zeile 1), Anschrift (2–6, PLZ/Ort 7/8), Datum (Zeile 11),
/// Text ab der ersten Textzeile nach den Kennzahlen.
pub fn brief(text: &str) -> AltBrief {
    let l = zeilen(text);
    let mut adresse: Vec<String> = (1..6).map(|i| z(&l, i).to_string()).collect();
    adresse.push(format!("{} {}", z(&l, 6), z(&l, 7)).trim().to_string());
    let adresse = adresse.into_iter().filter(|s| !s.is_empty()).collect();
    let start = (11..l.len()).find(|&i| {
        let t = z(&l, i);
        !t.is_empty() && !t.chars().all(|c| c.is_ascii_digit() || c == '-' || c == ' ')
    });
    let body = start.map(|s| l[s..].join("\n")).unwrap_or_default();
    AltBrief { titel: z(&l, 0).to_string(), adresse, datum: datum(z(&l, 10)), text: super::rtf::rtf_zu_html(body.trim()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    const LV: &str = " 3 \n 4 \n 6 \n 7 \n 0\nEUR\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\nN\n1\nArbeitsgerüst, incl. Auf- und Abbau.\n      265,00\nm²\n          8,85\n    0,00\n      2345,25\n\nN\n2\nHolzwerk wie Dachgesimse\n      157,00\nm²\n        31,10\n    2,00\n      4785,05\n\nF\n\nabwaschen, schleifen\n\n\n\n\n\n\nK\n\nCom. Calw:\n\n\n\n\n\n\n\n";

    #[test]
    fn lv_mit_folgezeilen() {
        let lv = lv(LV);
        assert_eq!(lv.waehrung, "EUR");
        assert_eq!(lv.positionen.len(), 3);
        assert_eq!(lv.positionen[1].text, "Holzwerk wie Dachgesimse\nabwaschen, schleifen");
        assert_eq!(lv.positionen[1].rabatt, d("2"));
        assert_eq!(lv.positionen[0].gp, d("2345.25"));
        assert_eq!(lv.positionen[2].art, "K");
    }

    #[test]
    fn zahlen_und_datum() {
        assert_eq!(zahl("  1.234,56 "), d("1234.56"));
        assert_eq!(zahl("7.00"), d("7"));
        assert_eq!(zahl(""), Decimal::ZERO);
        assert_eq!(datum("21.06.2024"), "2024-06-21");
        assert_eq!(datum("1.2.99"), "1999-02-01");
        assert_eq!(datum("x"), "");
        assert_eq!(seriendatum(45464), "2024-06-21");
    }

    #[test]
    fn vorspann_glinka() {
        let mut t = String::from("Herrn\nManfred Glinka\n\n\nHölderlinstr. 3\n75387\nNeubulach\n\n\n21.06.2024\nAngebot\n18155\n1370\n655\n\ntx10\n\n4408034157\n\n\n\n");
        t.push_str("{\\rtf1\\ansi\\pard\\plain f\\'fcr Malerarbeiten.\\par }");
        let v = vorspann(&t);
        assert_eq!(v.adresse, vec!["Herrn", "Manfred Glinka", "Hölderlinstr. 3", "75387 Neubulach"]);
        assert_eq!((v.datum.as_str(), v.belegart.as_str(), v.nummer.as_str()), ("2024-06-21", "Angebot", "1370"));
        assert_eq!(v.vortext, "<p>für Malerarbeiten.</p>");
    }

    #[test]
    fn nachspann_mit_rabatt_und_skonto() {
        let mut l = vec![""; 116];
        l[0] = "Rabatt";
        l[5] = " 1 ";
        l[7] = "5,00";
        l[8] = "19,00";
        l[10] = "2,00";
        l[13] = "8";
        l[14] = "16";
        l[15] = "Skonto";
        l[16] = "-3,00";
        l[17] = " 2 ";
        l[108] = "Wir bedanken uns für den Auftrag.";
        let n = nachspann(&l.join("\n"));
        assert_eq!(n.rabatt, d("5"));
        assert_eq!(n.mwst, Some(d("19")));
        assert_eq!((n.skonto, n.skonto_tage, n.ziel_tage), (d("2"), 8, 16));
        assert_eq!(n.zuschlaege, vec![AltZuschlag { text: "Skonto".into(), wert: d("-3"), modus: 2 }]);
        assert_eq!(n.text, "<p>Wir bedanken uns für den Auftrag.</p>");
    }

    #[test]
    fn abschlagszahlungen() {
        let t = "AZ1\n6059\n7735\n19,00\n\n27.07.2016\n\n6500,00\n1235,00\n7735,00\n\n\n\n\nAZ2\n\n0\n";
        let a = abschlaege(t);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].nummer, "6059");
        assert_eq!(a[0].brutto, d("7735"));
        assert_eq!(a[0].datum, "2016-07-27");
    }

    #[test]
    fn aufmass_zeilen() {
        let t = " 3 \n 10 \n 0 \n 2 \nN\nEntfernen und Entsorgen\n  1  \n     \n1   \n\n\n\n\nM\nSitzungszimmer\n  1  \n  0  \n  9,85  \n  6,02  \n\n  59,30  \n\n1\n9+0,85\n 1\n\n\n\n\n\n\n";
        let a = aufmass(t);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].posnr, "1");
        assert_eq!(a[0].m1, "9,85");
        assert_eq!(a[0].ergebnis, d("59.3"));
        assert_eq!(a[1].art, "R");
    }

    #[test]
    fn projektliste() {
        let mut t = String::new();
        for (n, b) in [("10529", "Altbau Firma Hollingsworth"), ("10606", "Malerarbeiten aussen")] {
            let satz = format!("0001  {n}  15.02.2000  {b:<50}XG  G");
            t.push_str(&format!("{satz:<100}"));
        }
        let p = projekte(&t);
        assert_eq!(p.len(), 2);
        assert_eq!(p[1], AltProjekt { nummer: "10606".into(), datum: "2000-02-15".into(), bezeichnung: "Malerarbeiten aussen".into() });
    }
}
