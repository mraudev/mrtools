//! Summen eines Belegs: Positionen, Rabatt, Zu-/Abschläge, MwSt, abgezogene Abschläge, Lohnanteil.
//!
//! Gerundet wird wie in GHFakt kaufmännisch auf Cent: je Position, dann Rabatt, jeder Zuschlag und die MwSt.

use crate::modelle::{AbschlagAbzug, Beleg, Position, Zuschlag};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::Serialize;

pub fn cent(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

pub fn prozent(basis: Decimal, satz: Decimal) -> Decimal {
    cent(basis * satz / Decimal::ONE_HUNDRED)
}

/// Zählt die Position zur Summe? Alternativ-/Bedarfspositionen, Titel und Texte nicht.
pub fn zaehlt(p: &Position) -> bool {
    p.art == "N"
}

/// Gesamtpreis einer Position. Gespeichert wird er in `Position::gp`; `summen` rechnet mit dem
/// gespeicherten Wert, damit festgeschriebene (auch übernommene) Belege ihren Betrag behalten.
pub fn gp(p: &Position) -> Decimal {
    let roh = cent(p.menge * p.ep);
    if p.rabatt.is_zero() { roh } else { roh - prozent(roh, p.rabatt) }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Zeile {
    pub text: String,
    pub betrag: Decimal,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Summen {
    /// Summe der Positionen.
    pub positionen: Decimal,
    pub rabatt: Decimal,
    pub zuschlaege_netto: Vec<Zeile>,
    pub netto: Decimal,
    pub mwst_satz: Decimal,
    pub mwst: Decimal,
    pub brutto: Decimal,
    pub zuschlaege_brutto: Vec<Zeile>,
    /// Brutto nach Zu-/Abschlägen auf den Bruttobetrag.
    pub gesamt: Decimal,
    pub abzug: Decimal,
    pub zahlbetrag: Decimal,
    /// Lohnanteil netto/MwSt (für §35a EStG).
    pub lohn_netto: Decimal,
    pub lohn_mwst: Decimal,
    /// Summe je Titel (Index der Titelposition → Summe der folgenden Positionen bis zum nächsten Titel).
    pub titel: Vec<(usize, Decimal)>,
    /// Zwischensummen (Index der S-Position → Summe seit Anfang oder letzter Zwischensumme).
    pub zwischen: Vec<(usize, Decimal)>,
}

pub fn summen(beleg: &Beleg, positionen: &[Position], zuschlaege: &[Zuschlag], abzuege: &[AbschlagAbzug]) -> Summen {
    let mut s = Summen { mwst_satz: if beleg.netto_rechnung { Decimal::ZERO } else { beleg.mwst_satz }, ..Default::default() };
    let mut titel: Option<(usize, Decimal)> = None;
    let mut zwischen = Decimal::ZERO;
    for (i, p) in positionen.iter().enumerate() {
        match p.art.as_str() {
            "T" => {
                if let Some(t) = titel.take() {
                    s.titel.push(t);
                }
                titel = Some((i, Decimal::ZERO));
            }
            "S" => {
                s.zwischen.push((i, zwischen));
                zwischen = Decimal::ZERO;
            }
            _ if zaehlt(p) => {
                let g = p.gp;
                s.positionen += g;
                zwischen += g;
                if let Some(t) = titel.as_mut() {
                    t.1 += g;
                }
                if !p.lohn.is_zero() {
                    s.lohn_netto += cent(p.menge * p.lohn);
                }
            }
            _ => {}
        }
    }
    if let Some(t) = titel {
        s.titel.push(t);
    }

    // Wie GHFakt: das Netto nach Rabatt wird gerundet, der Rabatt ist die Differenz.
    let mut netto = cent(s.positionen * (Decimal::ONE_HUNDRED - beleg.rabatt_prozent) / Decimal::ONE_HUNDRED);
    s.rabatt = s.positionen - netto;
    let mut sortiert: Vec<&Zuschlag> = zuschlaege.iter().collect();
    sortiert.sort_by_key(|z| z.pos);
    for z in &sortiert {
        let betrag = match z.art.as_str() {
            "prozent_netto" => prozent(netto, z.wert),
            "betrag_netto" => z.wert,
            _ => continue,
        };
        netto += betrag;
        s.zuschlaege_netto.push(Zeile { text: z.text.clone(), betrag });
    }
    s.netto = netto;
    s.mwst = prozent(netto, s.mwst_satz);
    s.brutto = netto + s.mwst;
    let mut gesamt = s.brutto;
    for z in &sortiert {
        let betrag = match z.art.as_str() {
            "prozent_brutto" => prozent(s.brutto, z.wert),
            "betrag_brutto" => z.wert,
            _ => continue,
        };
        gesamt += betrag;
        s.zuschlaege_brutto.push(Zeile { text: z.text.clone(), betrag });
    }
    s.gesamt = gesamt;
    s.abzug = abzuege.iter().map(|a| a.brutto).sum();
    s.zahlbetrag = gesamt - s.abzug;
    if !s.positionen.is_zero() && !s.rabatt.is_zero() {
        // Rabatt anteilig auch auf den Lohn.
        s.lohn_netto -= prozent(s.lohn_netto, beleg.rabatt_prozent);
    }
    s.lohn_mwst = prozent(s.lohn_netto, s.mwst_satz);
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn pos(art: &str, menge: &str, ep: &str) -> Position {
        let mut p = Position { art: art.into(), menge: d(menge), ep: d(ep), ..Default::default() };
        p.gp = gp(&p);
        p
    }

    fn beleg(mwst: &str) -> Beleg {
        Beleg { mwst_satz: d(mwst), ..Default::default() }
    }

    #[test]
    fn angebot_glinka_wie_ghfakt() {
        // 18155LV.051: 265,00 m² × 8,85 = 2345,25 und 157 × 31,10 = 4882,70
        let p = vec![pos("N", "265", "8.85"), pos("K", "0", "0"), pos("N", "157", "31.10"), pos("A", "10", "99")];
        let s = summen(&beleg("19"), &p, &[], &[]);
        assert_eq!(s.positionen, d("7227.95"));
        assert_eq!(s.mwst, d("1373.31"));
        assert_eq!(s.zahlbetrag, d("8601.26"));
    }

    #[test]
    fn rabatt_und_skonto_auf_brutto() {
        // 11386LV.056: Skonto -3 % auf brutto (GHFakt-Modus 2)
        let p = vec![pos("N", "1", "4766.74")];
        let z = vec![Zuschlag { text: "Skonto".into(), wert: d("-3"), art: "prozent_brutto".into(), ..Default::default() }];
        let a = vec![AbschlagAbzug { brutto: d("5104.00"), ..Default::default() }];
        let s = summen(&beleg("16"), &p, &z, &[]);
        assert_eq!(s.brutto, d("5529.42"));
        assert_eq!(s.gesamt, d("5363.54"));
        let mit_abzug = summen(&beleg("16"), &p, &z, &a);
        assert_eq!(mit_abzug.zahlbetrag, d("259.54"));

        let mut b = beleg("19");
        b.rabatt_prozent = d("5");
        let s = summen(&b, &[pos("N", "1", "349.40")], &[], &[]);
        assert_eq!(s.netto, d("331.93"));
        assert_eq!(s.brutto, d("395.00"));

        // 16632LV.054: 223,65 − 10 % = 201,285 → 201,29 → brutto 239,54 wie im Rechnungsbuch
        b.rabatt_prozent = d("10");
        let s = summen(&b, &[pos("N", "1", "223.65")], &[], &[]);
        assert_eq!(s.netto, d("201.29"));
        assert_eq!(s.gesamt, d("239.54"));
    }

    #[test]
    fn titel_zwischensumme_und_lohn() {
        let mut a = pos("N", "2", "10");
        a.lohn = d("6");
        a.gp = gp(&a);
        let p = vec![pos("T", "0", "0"), a, pos("N", "1", "5"), pos("S", "0", "0"), pos("T", "0", "0"), pos("N", "1", "1")];
        let mut b = beleg("19");
        b.netto_rechnung = true;
        let s = summen(&b, &p, &[], &[]);
        assert_eq!(s.titel, vec![(0, d("25")), (4, d("1"))]);
        assert_eq!(s.zwischen, vec![(3, d("25"))]);
        assert_eq!(s.lohn_netto, d("12"));
        assert_eq!(s.mwst, Decimal::ZERO);
    }

    #[test]
    fn positionsrabatt() {
        let mut p = pos("N", "3", "10");
        p.rabatt = d("10");
        assert_eq!(gp(&p), d("27.00"));
    }
}
