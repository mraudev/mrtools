//! Die Datensätze der Tabellen aus `schema/*.sql`.

use crate::datensatz;
use rust_decimal::Decimal;

datensatz!(Kunde, "kunde" {
    nummer: String, typ: String, suchname: String, anrede: String, name1: String, name2: String, name3: String,
    strasse: String, ortsteil: String, plz: String, ort: String, land: String,
    telefon: String, telefon_privat: String, telefax: String, mobil: String, mobil_privat: String,
    email: String, email_privat: String, internet: String, ansprechpartner: String, position: String,
    geschlecht: String, geburtstag: String, bank: String, blz: String, konto: String, iban: String, bic: String,
    ustid: String, leitweg_id: String, unsere_kdnr: String, ihre_kdnr: String, gruppe: String,
    skonto: String, rabatt: String, zahlungsbedingung: String, rechnungsart: String, ust_art: String,
    sonstiges1: String, sonstiges2: String, sonstiges3: String, sonstiges4: String, sonstiges5: String,
    bemerkung1: String, bemerkung2: String, notiz: String, alt_id: String,
    angelegt: String, geaendert: String, zugriff: String,
});

impl Kunde {
    /// Anschrift für Belege und Briefe, eine Zeile je Eintrag, ohne Leerzeilen.
    pub fn anschrift(&self) -> String {
        let ort = format!("{} {}", self.plz.trim(), self.ort.trim()).trim().to_string();
        let ort = if self.ortsteil.trim().is_empty() || self.ort.contains(self.ortsteil.trim()) {
            ort
        } else {
            format!("{ort}-{}", self.ortsteil.trim())
        };
        [&self.anrede, &self.name1, &self.name2, &self.name3, &self.strasse, &ort, &self.land]
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Name für Listen: Suchname oder Name1/Name2.
    pub fn anzeigename(&self) -> String {
        let name = [&self.name1, &self.name2].iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
        if name.is_empty() { self.suchname.clone() } else { name }
    }
}

datensatz!(Projekt, "projekt" {
    kunde_id: i64, nummer: String, bezeichnung: String, datum: String, baustelle: String, status: String, notiz: String,
});

datensatz!(Beleg, "beleg" {
    projekt_id: i64, kunde_id: i64, art: String, titel: String, nummer: String, datum: String, adresse: String,
    betreff: String, vortext: String, nachtext: String, leistung: String,
    mwst_satz: Decimal, rabatt_text: String, rabatt_prozent: Decimal, skonto_prozent: Decimal, skonto_tage: i64,
    zahlungsziel_tage: i64, zahlungsbedingung: String, netto_rechnung: bool, lohn_ausweisen: bool,
    status: String, vorgaenger_id: Option<i64>,
    netto: Decimal, mwst: Decimal, brutto: Decimal, abzug: Decimal, zahlbetrag: Decimal,
    bezahlt: Decimal, bezahlt_am: String, mahnstufe: i64, gedruckt_am: String,
    waehrung: String, alt_slot: String, alt_betrag: String, angelegt: String, geaendert: String,
});

datensatz!(Position, "position" {
    beleg_id: i64, pos: i64, art: String, posnr: String, text: String, menge: Decimal, einheit: String,
    ep: Decimal, rabatt: Decimal, gp: Decimal, lohn: Decimal, artikel_id: Option<i64>, menge_aus_aufmass: bool,
});

datensatz!(Zuschlag, "zuschlag" {
    beleg_id: i64, pos: i64, text: String, wert: Decimal, art: String,
});

datensatz!(AbschlagAbzug, "abschlag_abzug" {
    beleg_id: i64, pos: i64, text: String, nummer: String, datum: String,
    netto: Decimal, mwst: Decimal, brutto: Decimal, bezahlt: Decimal, quelle_id: Option<i64>,
});

datensatz!(Formel, "formel" {
    nummer: i64, name: String, ausdruck: String, hinweis: String,
});

datensatz!(Aufmass, "aufmass" {
    beleg_id: i64, pos: i64, posnr: String, art: String, text: String, anzahl: Decimal, formel: i64,
    m1: String, m2: String, m3: String, ergebnis: Decimal, abzug: bool,
});

datensatz!(Zahlung, "zahlung" {
    beleg_id: i64, datum: String, betrag: Decimal, abzug: Decimal, notiz: String,
});

datensatz!(Mahnung, "mahnung" {
    beleg_id: i64, stufe: i64, datum: String, frist: String, gebuehr: Decimal, text: String,
});

datensatz!(Regie, "regie" {
    projekt_id: i64, datum: String, titel: String, notiz: String, beleg_id: Option<i64>, abgerechnet_id: Option<i64>,
});

datensatz!(RegieZeile, "regie_zeile" {
    regie_id: i64, pos: i64, art: String, posnr: String, text: String, menge: Decimal, einheit: String, ep: Decimal,
    mitarbeiter_id: Option<i64>, artikel_id: Option<i64>,
});

datensatz!(Artikel, "artikel" {
    nummer: String, suchtext: String, text: String, gewerk: String, einheit: String,
    ek: Decimal, vk1: Decimal, vk2: Decimal, vk3: Decimal, lohn: Decimal, rabattgruppe: String, rabatt: Decimal,
    lieferant_id: Option<i64>, lieferant: String, lief_nr: String, ean: String, gewicht: String, abmessung: String,
    warengruppe: String, preiseinheit: Decimal, lager_aktiv: bool, bestand: Decimal, mindestbestand: Decimal,
    bestellmenge: Decimal, lagerplatz: String, datum_preis: String, notiz: String, quelle: String, alt_id: String,
});

datensatz!(LagerBewegung, "lager_bewegung" {
    artikel_id: i64, datum: String, menge: Decimal, text: String, beleg_id: Option<i64>,
});

datensatz!(Mitarbeiter, "mitarbeiter" {
    name: String, qualifikation: String, stundensatz: Decimal, kosten: Decimal, aktiv: bool,
});

datensatz!(Kalkulation, "kalkulation" {
    position_id: i64, pos: i64, art: String, text: String, menge: Decimal, einheit: String, preis: Decimal,
    zuschlag: String, artikel_id: Option<i64>,
});

datensatz!(Zeit, "zeit" {
    projekt_id: i64, mitarbeiter_id: Option<i64>, datum: String, stunden: Decimal, posnr: String, text: String,
});

datensatz!(Brief, "brief" {
    kunde_id: i64, projekt_id: Option<i64>, datum: String, adresse: String, betreff: String, text: String, angelegt: String,
});

datensatz!(Textbaustein, "textbaustein" {
    kategorie: String, name: String, text: String,
});

datensatz!(Plz, "plz" {
    plz: String, ort: String, zusatz: String, land: String,
});
