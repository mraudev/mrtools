//! Belegarten.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Art {
    pub key: &'static str,
    pub titel: &'static str,
    /// Nummernkreis für neue Belege dieser Art.
    pub kreis: &'static str,
    /// Forderung, die ins Rechnungsbuch (offene Posten) gehört.
    pub forderung: bool,
}

pub const ARTEN: &[Art] = &[
    Art { key: "angebot", titel: "Angebot", kreis: "angebot", forderung: false },
    Art { key: "auftrag", titel: "Auftragsbestätigung", kreis: "auftrag", forderung: false },
    Art { key: "lieferschein", titel: "Lieferschein", kreis: "lieferschein", forderung: false },
    Art { key: "abschlag", titel: "Abschlagsrechnung", kreis: "rechnung", forderung: true },
    Art { key: "schluss", titel: "Schlussrechnung", kreis: "rechnung", forderung: true },
    Art { key: "rechnung", titel: "Rechnung", kreis: "rechnung", forderung: true },
    Art { key: "gutschrift", titel: "Gutschrift", kreis: "gutschrift", forderung: true },
    Art { key: "aufstellung", titel: "Aufstellung", kreis: "", forderung: false },
];

pub fn art(key: &str) -> Art {
    ARTEN.iter().copied().find(|a| a.key == key).unwrap_or(ARTEN[ARTEN.len() - 1])
}

/// Ordnet die gedruckte Bezeichnung aus GHFakt einer Belegart zu.
pub fn art_aus_titel(titel: &str) -> &'static str {
    let t = titel.to_lowercase();
    if t.contains("abschlag") {
        "abschlag"
    } else if t.contains("schlussrechnung") || t.contains("schlußrechnung") {
        "schluss"
    } else if t.contains("gutschrift") {
        "gutschrift"
    } else if t.contains("rechnung") {
        "rechnung"
    } else if t.contains("lieferschein") {
        "lieferschein"
    } else if t.contains("bestätigung") || t.contains("auftrag") {
        "auftrag"
    } else if t.contains("angebot") || t.contains("kostenvoranschlag") || t.contains("kostenschätzung") || t.contains("ausschreibung") {
        "angebot"
    } else {
        "aufstellung"
    }
}

/// Art eines Belegs, der im Rechnungsbuch steht, wenn die Belegdatei fehlt: aus dem LV-Slot.
pub fn art_aus_slot(slot: i64) -> &'static str {
    match slot {
        51 => "angebot",
        52 => "auftrag",
        53 => "lieferschein",
        54 | 19 | 23 => "rechnung",
        55 => "gutschrift",
        56..=60 => "abschlag",
        _ => "aufstellung",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titel_zuordnen() {
        assert_eq!(art_aus_titel("Abschlagsrechnung 2"), "abschlag");
        assert_eq!(art_aus_titel("Schlußrechnung   Nr. 08/1"), "schluss");
        assert_eq!(art_aus_titel("Rapport-Rechnung"), "rechnung");
        assert_eq!(art_aus_titel("Nachtragsangebot"), "angebot");
        assert_eq!(art_aus_titel("Bestätigung"), "auftrag");
        assert_eq!(art_aus_titel("Zusammenstellung"), "aufstellung");
        assert!(art("rechnung").forderung && !art("angebot").forderung);
    }
}
