//! Kleine Formelauswertung für Aufmaß und Kalkulation: + − × ÷, Klammern, Dezimalkomma,
//! Variablen `a`, `b`, `c` (Maß 1–3) und `pi`.

use rust_decimal::{Decimal, MathematicalOps};
use std::str::FromStr;

struct Parser<'a> {
    zeichen: Vec<char>,
    pos: usize,
    vars: &'a [(char, Decimal)],
}

impl Parser<'_> {
    fn leer(&mut self) {
        while self.pos < self.zeichen.len() && self.zeichen[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.leer();
        self.zeichen.get(self.pos).copied()
    }

    fn summe(&mut self) -> Result<Decimal, String> {
        let mut w = self.produkt()?;
        while let Some(c) = self.peek() {
            match c {
                '+' => { self.pos += 1; w += self.produkt()?; }
                '-' | '−' => { self.pos += 1; w -= self.produkt()?; }
                _ => break,
            }
        }
        Ok(w)
    }

    fn produkt(&mut self) -> Result<Decimal, String> {
        let mut w = self.potenz()?;
        while let Some(c) = self.peek() {
            match c {
                '*' | 'x' | '×' | '·' => { self.pos += 1; w *= self.potenz()?; }
                '/' | ':' | '÷' => {
                    self.pos += 1;
                    let n = self.potenz()?;
                    if n.is_zero() {
                        return Err("Division durch 0".into());
                    }
                    w /= n;
                }
                _ => break,
            }
        }
        Ok(w)
    }

    fn potenz(&mut self) -> Result<Decimal, String> {
        let basis = self.einheit()?;
        if self.peek() == Some('^') {
            self.pos += 1;
            let e = self.einheit()?;
            return Ok(basis.powd(e));
        }
        Ok(basis)
    }

    fn einheit(&mut self) -> Result<Decimal, String> {
        match self.peek() {
            Some('-') | Some('−') => { self.pos += 1; Ok(-self.einheit()?) }
            Some('+') => { self.pos += 1; self.einheit() }
            Some('(') => {
                self.pos += 1;
                let w = self.summe()?;
                if self.peek() != Some(')') {
                    return Err("Klammer fehlt".into());
                }
                self.pos += 1;
                Ok(w)
            }
            Some(c) if c.is_ascii_digit() || c == ',' || c == '.' => {
                let start = self.pos;
                while self.pos < self.zeichen.len() && (self.zeichen[self.pos].is_ascii_digit() || self.zeichen[self.pos] == ',' || self.zeichen[self.pos] == '.') {
                    self.pos += 1;
                }
                let text: String = self.zeichen[start..self.pos].iter().collect();
                // „1.234,5“ → 1234.5; „1,5“ → 1.5; „1.5“ → 1.5
                let norm = if text.contains(',') { text.replace('.', "").replace(',', ".") } else { text };
                Decimal::from_str(&norm).map_err(|_| format!("Zahl „{norm}“ ungültig"))
            }
            Some(c) if c.is_alphabetic() => {
                let start = self.pos;
                while self.pos < self.zeichen.len() && self.zeichen[self.pos].is_alphanumeric() {
                    self.pos += 1;
                }
                let name: String = self.zeichen[start..self.pos].iter().collect::<String>().to_lowercase();
                if name == "pi" {
                    return Ok(Decimal::PI);
                }
                let mut cs = name.chars();
                if let (Some(v), None) = (cs.next(), cs.next()) {
                    if let Some((_, w)) = self.vars.iter().find(|(n, _)| *n == v) {
                        return Ok(*w);
                    }
                }
                Err(format!("Unbekannt: {name}"))
            }
            Some(c) => Err(format!("Unerwartetes Zeichen „{c}“")),
            None => Err("Ausdruck unvollständig".into()),
        }
    }
}

pub fn auswerten(ausdruck: &str, vars: &[(char, Decimal)]) -> Result<Decimal, String> {
    let mut p = Parser { zeichen: ausdruck.chars().collect(), pos: 0, vars };
    if p.peek().is_none() {
        return Ok(Decimal::ZERO);
    }
    let w = p.summe()?;
    if p.peek().is_some() {
        return Err(format!("Unerwartetes Zeichen an Stelle {}", p.pos + 1));
    }
    Ok(w)
}

/// Feldinhalt (Zahl oder Rechnung wie „12+12+9“) auswerten; leer = `None`.
pub fn feld(text: &str) -> Result<Option<Decimal>, String> {
    if text.trim().is_empty() {
        Ok(None)
    } else {
        auswerten(text, &[]).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn rechnen() {
        assert_eq!(auswerten("12+12+9+9+3+3+1", &[]).unwrap(), d("49"));
        assert_eq!(auswerten("9,85 * 6,02", &[]).unwrap(), d("59.2970"));
        assert_eq!(auswerten("(a+b)/2*c", &[('a', d("2")), ('b', d("4")), ('c', d("3"))]).unwrap(), d("9"));
        assert_eq!(auswerten("-2*(3-1)", &[]).unwrap(), d("-4"));
        assert_eq!(auswerten("2^3", &[]).unwrap(), d("8"));
        assert!(auswerten("1/0", &[]).is_err());
        assert!(auswerten("2+", &[]).is_err());
        assert!(auswerten("x", &[]).is_err());
        assert_eq!(feld("").unwrap(), None);
    }
}
