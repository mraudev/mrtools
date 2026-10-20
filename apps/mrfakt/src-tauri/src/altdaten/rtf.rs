//! Wandelt die RTF-Texte aus GHFakt (TX Text Control) in das einfache HTML von mrfakt
//! (`<p>`, `<strong>`, `<em>`, `<u>`, `<br>`). Alles andere (Schriften, Farben, Ränder) entfällt.

fn cp1252(b: u8) -> char {
    let bytes = [b];
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
    text.chars().next().unwrap_or('?')
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Klartext (Zeilen) in Absätze: Leerzeilen trennen Absätze, einfache Zeilenumbrüche bleiben `<br>`.
pub fn text_zu_html(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let mut out = String::new();
    for absatz in text.split("\n\n") {
        let zeilen: Vec<&str> = absatz.lines().map(|l| l.trim_end()).collect();
        let inhalt = zeilen.iter().map(|l| escape(l)).collect::<Vec<_>>().join("<br>");
        if !inhalt.trim().is_empty() {
            out.push_str(&format!("<p>{inhalt}</p>"));
        }
    }
    out
}

#[derive(Clone, Copy, Default, PartialEq)]
struct Format {
    fett: bool,
    kursiv: bool,
    unterstrichen: bool,
    ignorieren: bool,
}

/// RTF → HTML. Gibt bei Klartext (kein `{\rtf`) den Text als HTML zurück.
pub fn rtf_zu_html(rtf: &str) -> String {
    let rtf = rtf.trim_end_matches(['\0', '\r', '\n', ' ']);
    if !rtf.trim_start().starts_with("{\\rtf") {
        return text_zu_html(rtf);
    }
    let bytes: Vec<char> = rtf.chars().collect();
    let mut stapel: Vec<Format> = vec![];
    let mut f = Format::default();
    let mut absaetze: Vec<String> = vec![];
    let mut absatz = String::new();
    let mut offen = Format::default(); // im aktuellen Absatz geöffnete Auszeichnung
    let mut uc_skip = 0usize;
    let mut i = 0;

    fn umschalten(absatz: &mut String, offen: &mut Format, ziel: Format) {
        if offen.unterstrichen && !ziel.unterstrichen { absatz.push_str("</u>"); }
        if offen.kursiv && !ziel.kursiv { absatz.push_str("</em>"); }
        if offen.fett && !ziel.fett { absatz.push_str("</strong>"); }
        if !offen.fett && ziel.fett { absatz.push_str("<strong>"); }
        if !offen.kursiv && ziel.kursiv { absatz.push_str("<em>"); }
        if !offen.unterstrichen && ziel.unterstrichen { absatz.push_str("<u>"); }
        *offen = Format { ignorieren: false, ..ziel };
    }
    let zeichen = |absatz: &mut String, offen: &mut Format, f: &Format, c: char| {
        if f.ignorieren { return; }
        if *offen != (Format { ignorieren: false, ..*f }) { umschalten(absatz, offen, *f); }
        match c {
            '&' => absatz.push_str("&amp;"),
            '<' => absatz.push_str("&lt;"),
            '>' => absatz.push_str("&gt;"),
            c => absatz.push(c),
        }
    };

    while i < bytes.len() {
        let c = bytes[i];
        match c {
            '{' => {
                stapel.push(f);
                // Zielgruppen ({\*\...}, Schrift-/Farbtabellen) überspringen.
                let rest: String = bytes[i + 1..bytes.len().min(i + 14)].iter().collect();
                if rest.starts_with("\\*") || rest.starts_with("\\fonttbl") || rest.starts_with("\\colortbl")
                    || rest.starts_with("\\stylesheet") || rest.starts_with("\\info") || rest.starts_with("\\pict")
                {
                    f.ignorieren = true;
                }
                i += 1;
            }
            '}' => {
                f = stapel.pop().unwrap_or_default();
                i += 1;
            }
            '\\' => {
                i += 1;
                if i >= bytes.len() { break; }
                let n = bytes[i];
                if n == '\'' {
                    let hex: String = bytes[i + 1..bytes.len().min(i + 3)].iter().collect();
                    i += 3;
                    if uc_skip > 0 { uc_skip -= 1; continue; }
                    if let Ok(b) = u8::from_str_radix(&hex, 16) {
                        zeichen(&mut absatz, &mut offen, &f, cp1252(b));
                    }
                    continue;
                }
                if !n.is_ascii_alphabetic() {
                    // \\ \{ \} und Sonderzeichen
                    match n {
                        '\\' | '{' | '}' => zeichen(&mut absatz, &mut offen, &f, n),
                        '~' => zeichen(&mut absatz, &mut offen, &f, '\u{a0}'),
                        '-' | '_' => {}
                        '\n' | '\r' if !f.ignorieren => {
                            absaetze.push(std::mem::take(&mut absatz));
                            offen = Format::default();
                        }
                        _ => {}
                    }
                    i += 1;
                    continue;
                }
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() { i += 1; }
                let wort: String = bytes[start..i].iter().collect();
                let zstart = i;
                if i < bytes.len() && (bytes[i] == '-' || bytes[i].is_ascii_digit()) {
                    i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
                }
                let zahl: Option<i32> = bytes[zstart..i].iter().collect::<String>().parse().ok();
                if i < bytes.len() && bytes[i] == ' ' { i += 1; }
                match wort.as_str() {
                    "par" | "sect" | "page" => {
                        if !f.ignorieren {
                            umschalten(&mut absatz, &mut offen, Format::default());
                            absaetze.push(std::mem::take(&mut absatz));
                        }
                    }
                    "line" => if !f.ignorieren { absatz.push_str("<br>") },
                    "tab" => zeichen(&mut absatz, &mut offen, &f, ' '),
                    "b" => f.fett = zahl != Some(0),
                    "i" => f.kursiv = zahl != Some(0),
                    "ul" => f.unterstrichen = zahl != Some(0),
                    "ulnone" => f.unterstrichen = false,
                    "plain" => { f.fett = false; f.kursiv = false; f.unterstrichen = false; }
                    "u" => {
                        if let Some(z) = zahl {
                            let code = if z < 0 { (z + 65536) as u32 } else { z as u32 };
                            if let Some(ch) = char::from_u32(code) { zeichen(&mut absatz, &mut offen, &f, ch); }
                            uc_skip = 1;
                        }
                    }
                    "uc" => {}
                    "emdash" => zeichen(&mut absatz, &mut offen, &f, '—'),
                    "endash" => zeichen(&mut absatz, &mut offen, &f, '–'),
                    "bullet" => zeichen(&mut absatz, &mut offen, &f, '•'),
                    "lquote" => zeichen(&mut absatz, &mut offen, &f, '‘'),
                    "rquote" => zeichen(&mut absatz, &mut offen, &f, '’'),
                    "ldblquote" => zeichen(&mut absatz, &mut offen, &f, '“'),
                    "rdblquote" => zeichen(&mut absatz, &mut offen, &f, '”'),
                    _ => {}
                }
                uc_skip = if wort == "u" { uc_skip } else { 0 };
            }
            '\r' | '\n' | '\0' => i += 1,
            c => {
                if uc_skip > 0 { uc_skip -= 1; } else { zeichen(&mut absatz, &mut offen, &f, c); }
                i += 1;
            }
        }
    }
    umschalten(&mut absatz, &mut offen, Format::default());
    absaetze.push(absatz);

    // Leere Absätze am Ende entfernen, innere als Abstand erhalten.
    while absaetze.last().is_some_and(|a| a.trim().is_empty()) { absaetze.pop(); }
    absaetze
        .iter()
        .map(|a| if a.trim().is_empty() { "<p></p>".to_string() } else { format!("<p>{}</p>", a.trim()) })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tx_rtf_wird_html() {
        let rtf = "{\\rtf1\\ansi\\ansicpg1252\\uc0\\deff0{\\fonttbl\n{\\f0\\fswiss\\fcharset0\\fprq2 Arial;}}\n{\\colortbl;\\red0\\green0\\blue0;}\n{\\*\\generator TX_RTF32 10.1.323.500;}\n\\deftab1134\\pard\\plain\\f0\\fs22 f\\loch\\f0\\hich\\f0 \\'fcr Malerarbeiten au\\'dfen an Ihrem Wohnhaus.\\par\\pard\\par }\0";
        assert_eq!(rtf_zu_html(rtf), "<p>für Malerarbeiten außen an Ihrem Wohnhaus.</p>");
    }

    #[test]
    fn fett_und_absaetze() {
        let rtf = "{\\rtf1 Erste {\\b fette} Zeile\\par\\par Zweite & <x>\\par}";
        assert_eq!(rtf_zu_html(rtf), "<p>Erste <strong>fette</strong> Zeile</p><p></p><p>Zweite &amp; &lt;x&gt;</p>");
    }

    #[test]
    fn klartext() {
        assert_eq!(text_zu_html("Zeile 1\nZeile 2\n\nNeu"), "<p>Zeile 1<br>Zeile 2</p><p>Neu</p>");
    }
}
