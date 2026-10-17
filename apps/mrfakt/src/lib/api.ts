import { invoke } from "@tauri-apps/api/core";
import type {
  AbschlagAbzug,
  Allgemein,
  Artikel,
  Aufmass,
  BelegKurz,
  BelegVoll,
  Brief,
  DatanormErgebnis,
  DbInfo,
  Einstellungen,
  Firma,
  Formel,
  ImportBericht,
  JournalFilter,
  JournalListe,
  JournalZeile,
  KalkErgebnis,
  Kalkulation,
  Kalkulationssaetze,
  Kunde,
  KundeDetail,
  KundeZeile,
  LagerBewegung,
  Mahnung,
  Mitarbeiter,
  Nachkalkulation,
  Nummernkreis,
  Plz,
  Position,
  Projekt,
  RegieVoll,
  Summen,
  Textbaustein,
  Treffer,
  Umsatz,
  Zahlung,
  Zeit,
} from "./types";

export type Druckart = "beleg" | "brief" | "mahnung" | "aufmass" | "regie" | "regie_preise";

/** Typisierte Aufrufe der Rust-Befehle in `src-tauri/src`. */
export const api = {
  // Kunden und Projekte
  kunden: (suche: string, typ = "") => invoke<KundeZeile[]>("kunden_liste", { suche, typ }),
  kunde: (id: number) => invoke<KundeDetail>("kunde_laden", { id }),
  kundeSpeichern: (kunde: Kunde) => invoke<number>("kunde_speichern", { kunde }),
  kundeLoeschen: (id: number) => invoke<void>("kunde_loeschen", { id }),
  plz: (plz: string, ort: string) => invoke<Plz[]>("plz_suche", { plz, ort }),
  projektSpeichern: (projekt: Projekt) => invoke<number>("projekt_speichern", { projekt }),
  projektLoeschen: (id: number) => invoke<void>("projekt_loeschen", { id }),
  projekteSuchen: (suche: string) =>
    invoke<{ id: number; kunde_id: number; nummer: string; bezeichnung: string; kunde: string; datum: string }[]>("projekte_suchen", { suche }),

  // Belege
  beleg: (id: number) => invoke<BelegVoll>("beleg_laden", { id }),
  belegSpeichern: (beleg: BelegVoll) => invoke<BelegVoll>("beleg_speichern", { beleg }),
  belegSummen: (beleg: BelegVoll) => invoke<Summen>("beleg_summen", { beleg }),
  belegNeu: (projektId: number, art: string) => invoke<BelegVoll>("beleg_neu", { projektId, art }),
  belegUmwandeln: (id: number, art: string, projektId?: number) => invoke<BelegVoll>("beleg_umwandeln", { id, art, projektId: projektId ?? null }),
  belegLoeschen: (id: number) => invoke<void>("beleg_loeschen", { id }),
  belegFestschreiben: (id: number) => invoke<BelegVoll>("beleg_festschreiben", { id }),
  belegStornieren: (id: number) => invoke<BelegVoll>("beleg_stornieren", { id }),
  belegeProjekt: (projektId: number) => invoke<BelegKurz[]>("belege_projekt", { projektId }),
  abzuegeVorschlagen: (projektId: number, belegId: number) => invoke<AbschlagAbzug[]>("abzuege_vorschlagen", { projektId, belegId }),

  // Rechnungsbuch
  rechnungsbuch: (filter: JournalFilter) => invoke<JournalListe>("rechnungsbuch", { filter }),
  zahlungBuchen: (zahlung: Partial<Zahlung>) => invoke<void>("zahlung_buchen", { zahlung }),
  zahlungLoeschen: (id: number) => invoke<void>("zahlung_loeschen", { id }),
  mahnen: (belegId: number, stufe?: number) => invoke<Mahnung>("mahnung_erstellen", { belegId, stufe: stufe ?? null }),
  mahnungLoeschen: (id: number) => invoke<void>("mahnung_loeschen", { id }),
  mahnvorschlag: () => invoke<JournalZeile[]>("mahnvorschlag"),
  umsatz: (jahr: string) => invoke<Umsatz>("umsatz", { jahr }),
  rechnungsbuchCsv: (filter: JournalFilter, pfad: string) => invoke<void>("rechnungsbuch_csv", { filter, pfad }),

  // Druck
  pdfVorschau: (art: Druckart, id: number, ids?: number[]) => invoke<string>("pdf_vorschau", { art, id, ids: ids ?? null }),
  pdfSpeichern: (art: Druckart, id: number, opts: { ids?: number[]; pfad?: string; oeffnen?: boolean } = {}) =>
    invoke<string>("pdf_speichern", { art, id, ids: opts.ids ?? null, pfad: opts.pfad ?? null, oeffnen: opts.oeffnen ?? false }),
  erechnungXml: (id: number, pfad: string) => invoke<void>("erechnung_xml", { id, pfad }),
  oeffnen: (pfad: string) => invoke<void>("datei_oeffnen", { pfad }),
  ordnerZeigen: (pfad: string) => invoke<void>("ordner_zeigen", { pfad }),

  // Aufmaß
  aufmass: (belegId: number) => invoke<{ zeilen: Aufmass[]; formeln: Formel[]; gesperrt: boolean }>("aufmass_laden", { belegId }),
  aufmassSpeichern: (belegId: number, zeilen: Aufmass[], uebernehmen: boolean) => invoke<string[]>("aufmass_speichern", { belegId, zeilen, uebernehmen }),
  aufmassRechnen: (zeile: Aufmass) => invoke<number>("aufmass_rechnen", { zeile }),
  formeln: () => invoke<Formel[]>("formeln_liste"),
  formelnSpeichern: (formeln: Formel[]) => invoke<void>("formeln_speichern", { formeln }),

  // Regie
  regie: (projektId: number) => invoke<RegieVoll[]>("regie_liste", { projektId }),
  regieSpeichern: (regie: RegieVoll) => invoke<number>("regie_speichern", { regie }),
  regieLoeschen: (id: number) => invoke<void>("regie_loeschen", { id }),
  regieAbrechnen: (projektId: number, ids: number[]) => invoke<number>("regie_abrechnen", { projektId, ids }),

  // Artikel und Lager
  artikel: (suche: string, nurLager = false) => invoke<Artikel[]>("artikel_liste", { suche, nurLager }),
  artikelSpeichern: (artikel: Artikel) => invoke<number>("artikel_speichern", { artikel }),
  artikelLoeschen: (id: number) => invoke<void>("artikel_loeschen", { id }),
  lagerBuchen: (artikelId: number, menge: number, text: string) => invoke<void>("lager_buchen", { artikelId, menge, text }),
  lagerBewegungen: (artikelId: number) => invoke<LagerBewegung[]>("lager_bewegungen", { artikelId }),
  bestellvorschlag: () => invoke<Artikel[]>("bestellvorschlag"),
  datanorm: (pfade: string[], lieferant: string, aufschlag: number) => invoke<DatanormErgebnis>("datanorm_import", { pfade, lieferant, aufschlag }),
  artikelEtiketten: (ids: number[], pfad: string, spalten: number, hoehe: number) => invoke<void>("artikel_etiketten", { ids, pfad, spalten, hoehe }),
  kundenEtiketten: (ids: number[], pfad: string, spalten: number, hoehe: number) => invoke<void>("kunden_etiketten", { ids, pfad, spalten, hoehe }),

  // Kalkulation
  mitarbeiter: () => invoke<Mitarbeiter[]>("mitarbeiter_liste"),
  mitarbeiterSpeichern: (mitarbeiter: Mitarbeiter) => invoke<number>("mitarbeiter_speichern", { mitarbeiter }),
  mitarbeiterLoeschen: (id: number) => invoke<void>("mitarbeiter_loeschen", { id }),
  kalkulation: (positionId: number) =>
    invoke<{ zeilen: Kalkulation[]; saetze: Kalkulationssaetze; ergebnis: KalkErgebnis; gesperrt: boolean }>("kalkulation_laden", { positionId }),
  kalkulationRechnen: (zeilen: Kalkulation[]) => invoke<KalkErgebnis>("kalkulation_rechnen", { zeilen }),
  kalkulationSpeichern: (positionId: number, zeilen: Kalkulation[], uebernehmen: boolean) =>
    invoke<KalkErgebnis>("kalkulation_speichern", { positionId, zeilen, uebernehmen }),
  kalkulationArtikel: (artikelId: number) => invoke<Kalkulation>("kalkulation_artikel", { artikelId }),
  zeiten: (projektId: number) => invoke<Zeit[]>("zeiten_liste", { projektId }),
  zeitSpeichern: (zeit: Zeit) => invoke<number>("zeit_speichern", { zeit }),
  zeitLoeschen: (id: number) => invoke<void>("zeit_loeschen", { id }),
  nachkalkulation: (projektId: number) => invoke<Nachkalkulation>("nachkalkulation", { projektId }),

  // GAEB
  gaebImport: (projektId: number, pfad: string) => invoke<number>("gaeb_import", { projektId, pfad }),
  gaebExport: (belegId: number, pfad: string) => invoke<void>("gaeb_export", { belegId, pfad }),

  // Briefe, Textbausteine, Suche
  briefe: (kundeId: number) => invoke<Brief[]>("briefe_liste", { kundeId }),
  brief: (id: number) => invoke<Brief>("brief_laden", { id }),
  briefNeu: (kundeId: number, projektId?: number) => invoke<Brief>("brief_neu", { kundeId, projektId: projektId ?? null }),
  briefSpeichern: (brief: Brief) => invoke<number>("brief_speichern", { brief }),
  briefLoeschen: (id: number) => invoke<void>("brief_loeschen", { id }),
  textbausteine: () => invoke<Textbaustein[]>("textbausteine"),
  textbausteinSpeichern: (baustein: Textbaustein) => invoke<number>("textbaustein_speichern", { baustein }),
  textbausteinLoeschen: (id: number) => invoke<void>("textbaustein_loeschen", { id }),
  suche: (text: string) => invoke<Treffer[]>("suche", { text }),

  // Einstellungen und Daten
  einstellungen: () => invoke<Einstellungen>("einstellungen_laden"),
  einstellungenSpeichern: (firma: Firma, allgemein: Allgemein, nummernkreise: Nummernkreis[]) =>
    invoke<void>("einstellungen_speichern", { firma, allgemein, nummernkreise }),
  bildSetzen: (name: "logo" | "fusszeile", pfad: string) => invoke<void>("bild_setzen", { name, pfad }),
  bildLaden: (name: "logo" | "fusszeile") => invoke<string | null>("bild_laden", { name }),
  dbInfo: () => invoke<DbInfo>("db_info"),
  sichern: () => invoke<string>("sichern"),
  dbWechseln: (pfad: string) => invoke<void>("db_wechseln", { pfad }),
  wiederherstellen: (quelle: string) => invoke<void>("wiederherstellen", { quelle }),
  altdatenVorschlag: () => invoke<string | null>("altdaten_vorschlag"),
  altdatenImportieren: (ordner: string) => invoke<ImportBericht>("altdaten_importieren", { ordner }),
};

export type { Position };
