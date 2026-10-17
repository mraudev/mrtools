// Spiegel der Rust-Datensätze (src-tauri/src/modelle.rs). Beträge/Mengen kommen als Zahl.

export interface Kunde {
  id: number;
  nummer: string;
  typ: string;
  suchname: string;
  anrede: string;
  name1: string;
  name2: string;
  name3: string;
  strasse: string;
  ortsteil: string;
  plz: string;
  ort: string;
  land: string;
  telefon: string;
  telefon_privat: string;
  telefax: string;
  mobil: string;
  mobil_privat: string;
  email: string;
  email_privat: string;
  internet: string;
  ansprechpartner: string;
  position: string;
  geschlecht: string;
  geburtstag: string;
  bank: string;
  blz: string;
  konto: string;
  iban: string;
  bic: string;
  ustid: string;
  leitweg_id: string;
  unsere_kdnr: string;
  ihre_kdnr: string;
  gruppe: string;
  skonto: string;
  rabatt: string;
  zahlungsbedingung: string;
  rechnungsart: string;
  ust_art: string;
  sonstiges1: string;
  sonstiges2: string;
  sonstiges3: string;
  sonstiges4: string;
  sonstiges5: string;
  bemerkung1: string;
  bemerkung2: string;
  notiz: string;
  alt_id: string;
  angelegt: string;
  geaendert: string;
  zugriff: string;
}

export interface KundeZeile {
  id: number;
  nummer: string;
  typ: string;
  suchname: string;
  name: string;
  strasse: string;
  ort: string;
  telefon: string;
  offen: number;
  letzter_beleg: string;
}

export interface Projekt {
  id: number;
  kunde_id: number;
  nummer: string;
  bezeichnung: string;
  datum: string;
  baustelle: string;
  status: string;
  notiz: string;
}

export interface ProjektZeile extends Projekt {
  belege: number;
  letzter: string;
  summe: number;
}

export interface KundeDetail {
  kunde: Kunde;
  projekte: ProjektZeile[];
  umsatz: [string, number][];
  offen: number;
  briefe: number;
}

export type Belegart = "angebot" | "auftrag" | "lieferschein" | "abschlag" | "schluss" | "rechnung" | "gutschrift" | "aufstellung";
export type Status = "entwurf" | "gedruckt" | "festgeschrieben" | "storniert";

export interface Beleg {
  id: number;
  projekt_id: number;
  kunde_id: number;
  art: Belegart;
  titel: string;
  nummer: string;
  datum: string;
  adresse: string;
  betreff: string;
  vortext: string;
  nachtext: string;
  leistung: string;
  mwst_satz: number;
  rabatt_text: string;
  rabatt_prozent: number;
  skonto_prozent: number;
  skonto_tage: number;
  zahlungsziel_tage: number;
  zahlungsbedingung: string;
  netto_rechnung: boolean;
  lohn_ausweisen: boolean;
  status: Status;
  vorgaenger_id: number | null;
  netto: number;
  mwst: number;
  brutto: number;
  abzug: number;
  zahlbetrag: number;
  bezahlt: number;
  bezahlt_am: string;
  mahnstufe: number;
  gedruckt_am: string;
  waehrung: string;
  alt_slot: string;
  alt_betrag: string;
  angelegt: string;
  geaendert: string;
}

/** N normal, G Gewerk, T Titel, K Text, A Alternativ, B Bedarf, S Zwischensumme */
export type PosArt = "N" | "G" | "T" | "K" | "A" | "B" | "S";

export interface Position {
  id: number;
  beleg_id: number;
  pos: number;
  art: PosArt;
  posnr: string;
  text: string;
  menge: number;
  einheit: string;
  ep: number;
  rabatt: number;
  gp: number;
  lohn: number;
  artikel_id: number | null;
  menge_aus_aufmass: boolean;
}

export type ZuschlagArt = "prozent_netto" | "betrag_netto" | "prozent_brutto" | "betrag_brutto";

export interface Zuschlag {
  id: number;
  beleg_id: number;
  pos: number;
  text: string;
  wert: number;
  art: ZuschlagArt;
}

export interface AbschlagAbzug {
  id: number;
  beleg_id: number;
  pos: number;
  text: string;
  nummer: string;
  datum: string;
  netto: number;
  mwst: number;
  brutto: number;
  bezahlt: number;
  quelle_id: number | null;
}

export interface Zahlung {
  id: number;
  beleg_id: number;
  datum: string;
  betrag: number;
  abzug: number;
  notiz: string;
}

export interface Mahnung {
  id: number;
  beleg_id: number;
  stufe: number;
  datum: string;
  frist: string;
  gebuehr: number;
  text: string;
}

export interface Zeile {
  text: string;
  betrag: number;
}

export interface Summen {
  positionen: number;
  rabatt: number;
  zuschlaege_netto: Zeile[];
  netto: number;
  mwst_satz: number;
  mwst: number;
  brutto: number;
  zuschlaege_brutto: Zeile[];
  gesamt: number;
  abzug: number;
  zahlbetrag: number;
  lohn_netto: number;
  lohn_mwst: number;
  titel: [number, number][];
  zwischen: [number, number][];
}

export interface BelegKurz {
  id: number;
  art: Belegart;
  titel: string;
  nummer: string;
  datum: string;
  status: Status;
  netto: number;
  zahlbetrag: number;
  bezahlt: number;
  waehrung: string;
}

export interface BelegVoll {
  beleg: Beleg;
  positionen: Position[];
  zuschlaege: Zuschlag[];
  abzuege: AbschlagAbzug[];
  summen: Summen;
  kunde: Kunde | null;
  projekt: Projekt | null;
  zahlungen: Zahlung[];
  mahnungen: Mahnung[];
  aufmass: number;
  nachfolger: BelegKurz[];
}

export interface JournalZeile {
  id: number;
  projekt_id: number;
  kunde_id: number;
  art: Belegart;
  titel: string;
  nummer: string;
  datum: string;
  faellig: string;
  kunde: string;
  projekt: string;
  netto: number;
  mwst: number;
  betrag: number;
  bezahlt: number;
  offen: number;
  bezahlt_am: string;
  mahnstufe: number;
  status: Status;
  waehrung: string;
  ueberfaellig: boolean;
}

export interface JournalListe {
  zeilen: JournalZeile[];
  netto: number;
  mwst: number;
  betrag: number;
  offen: number;
  jahre: string[];
}

export interface JournalFilter {
  jahr: string;
  art: string;
  status: string;
  suche: string;
  kunde_id?: number | null;
}

export interface Umsatz {
  jahr: string;
  monate: [number, number, number][];
  kunden: [string, number][];
  netto: number;
  mwst: number;
  offen: number;
  angebote: number;
  angebote_anzahl: number;
  jahre: [string, number][];
}

export interface Formel {
  id: number;
  nummer: number;
  name: string;
  ausdruck: string;
  hinweis: string;
}

export interface Aufmass {
  id: number;
  beleg_id: number;
  pos: number;
  posnr: string;
  art: "M" | "K" | "R";
  text: string;
  anzahl: number;
  formel: number;
  m1: string;
  m2: string;
  m3: string;
  ergebnis: number;
  abzug: boolean;
}

export interface Regie {
  id: number;
  projekt_id: number;
  datum: string;
  titel: string;
  notiz: string;
  beleg_id: number | null;
  abgerechnet_id: number | null;
}

export interface RegieZeile {
  id: number;
  regie_id: number;
  pos: number;
  art: "T" | "L" | "M" | "G" | "P";
  posnr: string;
  text: string;
  menge: number;
  einheit: string;
  ep: number;
  mitarbeiter_id: number | null;
  artikel_id: number | null;
}

export interface RegieVoll {
  regie: Regie;
  zeilen: RegieZeile[];
}

export interface Artikel {
  id: number;
  nummer: string;
  suchtext: string;
  text: string;
  gewerk: string;
  einheit: string;
  ek: number;
  vk1: number;
  vk2: number;
  vk3: number;
  lohn: number;
  rabattgruppe: string;
  rabatt: number;
  lieferant_id: number | null;
  lieferant: string;
  lief_nr: string;
  ean: string;
  gewicht: string;
  abmessung: string;
  warengruppe: string;
  preiseinheit: number;
  lager_aktiv: boolean;
  bestand: number;
  mindestbestand: number;
  bestellmenge: number;
  lagerplatz: string;
  datum_preis: string;
  notiz: string;
  quelle: string;
  alt_id: string;
}

export interface LagerBewegung {
  id: number;
  artikel_id: number;
  datum: string;
  menge: number;
  text: string;
  beleg_id: number | null;
}

export interface Mitarbeiter {
  id: number;
  name: string;
  qualifikation: string;
  stundensatz: number;
  kosten: number;
  aktiv: boolean;
}

export interface Kalkulation {
  id: number;
  position_id: number;
  pos: number;
  art: "L" | "M" | "G" | "F" | "S";
  text: string;
  menge: number;
  einheit: string;
  preis: number;
  zuschlag: string;
  artikel_id: number | null;
}

export interface Kalkulationssaetze {
  stundensatz: number;
  zuschlag_lohn: number;
  zuschlag_material: number;
  zuschlag_geraet: number;
  zuschlag_fremd: number;
  zuschlag_sonstig: number;
}

export interface KalkErgebnis {
  ep: number;
  lohn: number;
  minuten: number;
  zeilen: [number, number][];
}

export interface Zeit {
  id: number;
  projekt_id: number;
  mitarbeiter_id: number | null;
  datum: string;
  stunden: number;
  posnr: string;
  text: string;
}

export interface Nachkalkulation {
  beleg: string;
  erloes: number;
  soll_stunden: number;
  ist_stunden: number;
  lohnkosten: number;
  soll_lohn: number;
  ergebnis: number;
  je_mitarbeiter: [string, number, number][];
}

export interface Brief {
  id: number;
  kunde_id: number;
  projekt_id: number | null;
  datum: string;
  adresse: string;
  betreff: string;
  text: string;
  angelegt: string;
}

export interface Textbaustein {
  id: number;
  kategorie: string;
  name: string;
  text: string;
}

export interface Plz {
  id: number;
  plz: string;
  ort: string;
  zusatz: string;
  land: string;
}

export interface Treffer {
  art: "kunde" | "projekt" | "beleg" | "artikel";
  id: number;
  kunde_id: number;
  projekt_id: number;
  titel: string;
  info: string;
}

export interface Bank {
  name: string;
  iban: string;
  bic: string;
}

export interface Firma {
  name: string;
  inhaber: string;
  zusatz: string;
  strasse: string;
  plz: string;
  ort: string;
  land: string;
  telefon: string;
  telefax: string;
  mobil: string;
  email: string;
  internet: string;
  steuernummer: string;
  ustid: string;
  registergericht: string;
  banken: Bank[];
  absenderzeile: string;
  fusszeile: string;
  seitenleiste: string;
  waehrung: string;
  mwst_satz: number;
  mwst_ermaessigt: number;
  briefort: string;
  gruss: string;
  unterschrift: string;
}

export interface Zahlungsbedingung {
  name: string;
  text: string;
  skonto: number;
  skonto_tage: number;
  ziel_tage: number;
}

export interface Mahnstufe {
  titel: string;
  gebuehr: number;
  frist_tage: number;
  einleitung: string;
  schluss: string;
}

export interface Belegvorgabe {
  vortext: string;
  nachtext: string;
  zahlungsbedingung: string;
}

export interface Allgemein {
  einheiten: string[];
  zahlungsbedingungen: Zahlungsbedingung[];
  zahlungsbedingung_standard: number;
  mahnstufen: Mahnstufe[];
  belegvorgaben: Record<string, Belegvorgabe>;
  kalkulation: Kalkulationssaetze;
  pdf_ordner: string;
  sicherungen: number;
  zugferd: boolean;
  rand_links: number;
  rand_rechts: number;
  rand_oben: number;
  rand_unten: number;
}

export interface Nummernkreis {
  name: string;
  naechste: number;
  format: string;
}

export interface Einstellungen {
  firma: Firma;
  allgemein: Allgemein;
  nummernkreise: Nummernkreis[];
  logo: boolean;
  fussbild: boolean;
}

export interface DbInfo {
  pfad: string;
  sicherungen: string;
  groesse: number;
  kunden: number;
  belege: number;
}

export interface Abweichung {
  beleg: string;
  kunde: string;
  datum: string;
  berechnet: number;
  rechnungsbuch: number;
  waehrung: string;
}

export interface ImportBericht {
  anzahl: Record<string, number>;
  hinweise: string[];
  abweichungen: Abweichung[];
  abweichungen_gesamt: number;
  dauer_s: number;
}

export interface DatanormErgebnis {
  neu: number;
  geaendert: number;
  preise: number;
  texte: number;
  version: string;
}
