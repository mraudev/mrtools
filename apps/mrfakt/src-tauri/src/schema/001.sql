-- mrfakt Schema 1. Beträge/Mengen als TEXT (Dezimal), Datum als TEXT 'JJJJ-MM-TT'.

CREATE TABLE einstellung (
  schluessel TEXT PRIMARY KEY,
  wert TEXT NOT NULL
);

CREATE TABLE nummernkreis (
  name TEXT PRIMARY KEY,          -- kunde, projekt, angebot, auftrag, lieferschein, rechnung, gutschrift, regie
  naechste INTEGER NOT NULL,
  format TEXT NOT NULL DEFAULT '{n}'
);

CREATE TABLE datei (
  name TEXT PRIMARY KEY,          -- logo, fusszeile, ...
  mime TEXT NOT NULL,
  daten BLOB NOT NULL
);

CREATE TABLE plz (
  id INTEGER PRIMARY KEY,
  plz TEXT NOT NULL,
  ort TEXT NOT NULL,
  zusatz TEXT NOT NULL DEFAULT '',
  land TEXT NOT NULL DEFAULT ''
);
CREATE INDEX plz_plz ON plz (plz);
CREATE INDEX plz_ort ON plz (ort);

CREATE TABLE kunde (
  id INTEGER PRIMARY KEY,
  nummer TEXT NOT NULL DEFAULT '',
  typ TEXT NOT NULL DEFAULT 'Kunde',      -- Kunde, Lieferant, Interessent, Sonstige
  suchname TEXT NOT NULL DEFAULT '',
  anrede TEXT NOT NULL DEFAULT '',        -- Briefanrede/erste Adresszeile (Herrn, Frau, Firma, ...)
  name1 TEXT NOT NULL DEFAULT '',
  name2 TEXT NOT NULL DEFAULT '',
  name3 TEXT NOT NULL DEFAULT '',
  strasse TEXT NOT NULL DEFAULT '',
  ortsteil TEXT NOT NULL DEFAULT '',
  plz TEXT NOT NULL DEFAULT '',
  ort TEXT NOT NULL DEFAULT '',
  land TEXT NOT NULL DEFAULT '',
  telefon TEXT NOT NULL DEFAULT '',
  telefon_privat TEXT NOT NULL DEFAULT '',
  telefax TEXT NOT NULL DEFAULT '',
  mobil TEXT NOT NULL DEFAULT '',
  mobil_privat TEXT NOT NULL DEFAULT '',
  email TEXT NOT NULL DEFAULT '',
  email_privat TEXT NOT NULL DEFAULT '',
  internet TEXT NOT NULL DEFAULT '',
  ansprechpartner TEXT NOT NULL DEFAULT '',
  position TEXT NOT NULL DEFAULT '',
  geschlecht TEXT NOT NULL DEFAULT '',
  geburtstag TEXT NOT NULL DEFAULT '',
  bank TEXT NOT NULL DEFAULT '',
  blz TEXT NOT NULL DEFAULT '',
  konto TEXT NOT NULL DEFAULT '',
  iban TEXT NOT NULL DEFAULT '',
  bic TEXT NOT NULL DEFAULT '',
  ustid TEXT NOT NULL DEFAULT '',
  leitweg_id TEXT NOT NULL DEFAULT '',    -- für XRechnung an Behörden
  unsere_kdnr TEXT NOT NULL DEFAULT '',   -- unsere Kundennummer beim Lieferanten
  ihre_kdnr TEXT NOT NULL DEFAULT '',     -- Kontonummer des Kunden für uns
  gruppe TEXT NOT NULL DEFAULT '',
  skonto TEXT NOT NULL DEFAULT '',
  rabatt TEXT NOT NULL DEFAULT '',
  zahlungsbedingung TEXT NOT NULL DEFAULT '',
  rechnungsart TEXT NOT NULL DEFAULT '',  -- Neutral, Privat, Firma, Behörde
  ust_art TEXT NOT NULL DEFAULT '',       -- '', 13b, EU, Drittland
  sonstiges1 TEXT NOT NULL DEFAULT '',
  sonstiges2 TEXT NOT NULL DEFAULT '',
  sonstiges3 TEXT NOT NULL DEFAULT '',
  sonstiges4 TEXT NOT NULL DEFAULT '',
  sonstiges5 TEXT NOT NULL DEFAULT '',
  bemerkung1 TEXT NOT NULL DEFAULT '',
  bemerkung2 TEXT NOT NULL DEFAULT '',
  notiz TEXT NOT NULL DEFAULT '',
  alt_id TEXT NOT NULL DEFAULT '',        -- ID aus GHFakt (Projektordner G<id>)
  angelegt TEXT NOT NULL DEFAULT '',
  geaendert TEXT NOT NULL DEFAULT '',
  zugriff TEXT NOT NULL DEFAULT ''
);
CREATE INDEX kunde_suchname ON kunde (suchname);
CREATE INDEX kunde_alt ON kunde (alt_id);

CREATE TABLE projekt (
  id INTEGER PRIMARY KEY,
  kunde_id INTEGER NOT NULL REFERENCES kunde (id) ON DELETE CASCADE,
  nummer TEXT NOT NULL DEFAULT '',        -- Auftragsnummer (GHFakt: AuftrNummer)
  bezeichnung TEXT NOT NULL DEFAULT '',
  datum TEXT NOT NULL DEFAULT '',
  baustelle TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'offen',   -- offen, abgeschlossen
  notiz TEXT NOT NULL DEFAULT ''
);
CREATE INDEX projekt_kunde ON projekt (kunde_id);
CREATE INDEX projekt_nummer ON projekt (nummer);

CREATE TABLE beleg (
  id INTEGER PRIMARY KEY,
  projekt_id INTEGER NOT NULL REFERENCES projekt (id) ON DELETE CASCADE,
  kunde_id INTEGER NOT NULL REFERENCES kunde (id),
  art TEXT NOT NULL,                      -- siehe belege::ARTEN
  titel TEXT NOT NULL DEFAULT '',         -- gedruckte Bezeichnung, z. B. "Abschlagsrechnung 2"
  nummer TEXT NOT NULL DEFAULT '',
  datum TEXT NOT NULL DEFAULT '',
  adresse TEXT NOT NULL DEFAULT '',       -- Anschrift zum Zeitpunkt des Belegs (Zeilen)
  betreff TEXT NOT NULL DEFAULT '',
  vortext TEXT NOT NULL DEFAULT '',       -- einfaches HTML
  nachtext TEXT NOT NULL DEFAULT '',
  leistung TEXT NOT NULL DEFAULT '',      -- Leistungs-/Lieferzeitraum
  mwst_satz TEXT NOT NULL DEFAULT '19',
  rabatt_text TEXT NOT NULL DEFAULT 'Rabatt',
  rabatt_prozent TEXT NOT NULL DEFAULT '0',
  skonto_prozent TEXT NOT NULL DEFAULT '0',
  skonto_tage INTEGER NOT NULL DEFAULT 0,
  zahlungsziel_tage INTEGER NOT NULL DEFAULT 0,
  zahlungsbedingung TEXT NOT NULL DEFAULT '', -- Text mit Platzhaltern #ZN# #SK# ...
  netto_rechnung INTEGER NOT NULL DEFAULT 0,  -- Steuerschuldnerschaft des Leistungsempfängers (§13b)
  lohn_ausweisen INTEGER NOT NULL DEFAULT 0,  -- Lohnanteil nach §35a EStG ausweisen
  status TEXT NOT NULL DEFAULT 'entwurf',     -- entwurf, gedruckt, festgeschrieben, storniert
  vorgaenger_id INTEGER REFERENCES beleg (id) ON DELETE SET NULL,
  netto TEXT NOT NULL DEFAULT '0',            -- nach Rabatt/Zuschlägen
  mwst TEXT NOT NULL DEFAULT '0',
  brutto TEXT NOT NULL DEFAULT '0',
  abzug TEXT NOT NULL DEFAULT '0',            -- abgezogene Abschlagszahlungen (brutto)
  zahlbetrag TEXT NOT NULL DEFAULT '0',       -- Rechnungsbetrag nach Abzügen = Forderung
  bezahlt TEXT NOT NULL DEFAULT '0',
  bezahlt_am TEXT NOT NULL DEFAULT '',
  mahnstufe INTEGER NOT NULL DEFAULT 0,
  gedruckt_am TEXT NOT NULL DEFAULT '',
  waehrung TEXT NOT NULL DEFAULT 'EUR',
  alt_slot TEXT NOT NULL DEFAULT '',          -- GHFakt: LV-Endung (051..060)
  alt_betrag TEXT NOT NULL DEFAULT '',        -- GHFakt: RechBetrag aus dem Rechnungsbuch
  angelegt TEXT NOT NULL DEFAULT '',
  geaendert TEXT NOT NULL DEFAULT ''
);
CREATE INDEX beleg_projekt ON beleg (projekt_id);
CREATE INDEX beleg_kunde ON beleg (kunde_id);
CREATE INDEX beleg_datum ON beleg (datum);
CREATE INDEX beleg_nummer ON beleg (nummer);

CREATE TABLE position (
  id INTEGER PRIMARY KEY,
  beleg_id INTEGER NOT NULL REFERENCES beleg (id) ON DELETE CASCADE,
  pos INTEGER NOT NULL,
  art TEXT NOT NULL DEFAULT 'N',      -- N normal, G Gewerk, T Titel, K Text, A Alternativ, B Bedarf, S Zwischensumme
  posnr TEXT NOT NULL DEFAULT '',
  text TEXT NOT NULL DEFAULT '',      -- erste Zeile = Kurztext
  menge TEXT NOT NULL DEFAULT '0',
  einheit TEXT NOT NULL DEFAULT '',
  ep TEXT NOT NULL DEFAULT '0',
  rabatt TEXT NOT NULL DEFAULT '0',   -- Positionsrabatt in %
  gp TEXT NOT NULL DEFAULT '0',
  lohn TEXT NOT NULL DEFAULT '0',     -- Lohnanteil am EP (für §35a)
  artikel_id INTEGER REFERENCES artikel (id) ON DELETE SET NULL,
  menge_aus_aufmass INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX position_beleg ON position (beleg_id, pos);

CREATE TABLE zuschlag (
  id INTEGER PRIMARY KEY,
  beleg_id INTEGER NOT NULL REFERENCES beleg (id) ON DELETE CASCADE,
  pos INTEGER NOT NULL,
  text TEXT NOT NULL DEFAULT '',
  wert TEXT NOT NULL DEFAULT '0',
  art TEXT NOT NULL DEFAULT 'prozent_netto' -- prozent_netto, betrag_netto, prozent_brutto, betrag_brutto
);
CREATE INDEX zuschlag_beleg ON zuschlag (beleg_id);

CREATE TABLE abschlag_abzug (
  id INTEGER PRIMARY KEY,
  beleg_id INTEGER NOT NULL REFERENCES beleg (id) ON DELETE CASCADE,
  pos INTEGER NOT NULL,
  text TEXT NOT NULL DEFAULT '',
  nummer TEXT NOT NULL DEFAULT '',
  datum TEXT NOT NULL DEFAULT '',
  netto TEXT NOT NULL DEFAULT '0',
  mwst TEXT NOT NULL DEFAULT '0',
  brutto TEXT NOT NULL DEFAULT '0',
  bezahlt TEXT NOT NULL DEFAULT '0',
  quelle_id INTEGER REFERENCES beleg (id) ON DELETE SET NULL
);
CREATE INDEX abschlag_abzug_beleg ON abschlag_abzug (beleg_id);

CREATE TABLE formel (
  id INTEGER PRIMARY KEY,
  nummer INTEGER NOT NULL,
  name TEXT NOT NULL,
  ausdruck TEXT NOT NULL,             -- mit a, b, c (Maß 1–3), z. B. "a*b"
  hinweis TEXT NOT NULL DEFAULT ''
);

CREATE TABLE aufmass (
  id INTEGER PRIMARY KEY,
  beleg_id INTEGER NOT NULL REFERENCES beleg (id) ON DELETE CASCADE,
  pos INTEGER NOT NULL,
  posnr TEXT NOT NULL DEFAULT '',     -- Bezug auf position.posnr
  art TEXT NOT NULL DEFAULT 'M',      -- M Maßzeile, K Kommentar, R Rechenweg
  text TEXT NOT NULL DEFAULT '',
  anzahl TEXT NOT NULL DEFAULT '1',
  formel INTEGER NOT NULL DEFAULT 0,  -- 0 = Produkt der Maße
  m1 TEXT NOT NULL DEFAULT '',
  m2 TEXT NOT NULL DEFAULT '',
  m3 TEXT NOT NULL DEFAULT '',
  ergebnis TEXT NOT NULL DEFAULT '0',
  abzug INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX aufmass_beleg ON aufmass (beleg_id, pos);

CREATE TABLE zahlung (
  id INTEGER PRIMARY KEY,
  beleg_id INTEGER NOT NULL REFERENCES beleg (id) ON DELETE CASCADE,
  datum TEXT NOT NULL DEFAULT '',
  betrag TEXT NOT NULL DEFAULT '0',
  abzug TEXT NOT NULL DEFAULT '0',    -- Skonto/Einbehalt, gilt als ausgeglichen
  notiz TEXT NOT NULL DEFAULT ''
);
CREATE INDEX zahlung_beleg ON zahlung (beleg_id);

CREATE TABLE mahnung (
  id INTEGER PRIMARY KEY,
  beleg_id INTEGER NOT NULL REFERENCES beleg (id) ON DELETE CASCADE,
  stufe INTEGER NOT NULL,
  datum TEXT NOT NULL DEFAULT '',
  frist TEXT NOT NULL DEFAULT '',
  gebuehr TEXT NOT NULL DEFAULT '0',
  text TEXT NOT NULL DEFAULT ''
);
CREATE INDEX mahnung_beleg ON mahnung (beleg_id);

CREATE TABLE regie (
  id INTEGER PRIMARY KEY,
  projekt_id INTEGER NOT NULL REFERENCES projekt (id) ON DELETE CASCADE,
  datum TEXT NOT NULL DEFAULT '',
  titel TEXT NOT NULL DEFAULT '',
  notiz TEXT NOT NULL DEFAULT '',
  beleg_id INTEGER REFERENCES beleg (id) ON DELETE SET NULL, -- Beleg mit den Positionen (LV)
  abgerechnet_id INTEGER REFERENCES beleg (id) ON DELETE SET NULL
);
CREATE INDEX regie_projekt ON regie (projekt_id);

CREATE TABLE regie_zeile (
  id INTEGER PRIMARY KEY,
  regie_id INTEGER NOT NULL REFERENCES regie (id) ON DELETE CASCADE,
  pos INTEGER NOT NULL,
  art TEXT NOT NULL DEFAULT 'T',      -- T Text, L Lohn, M Material, G Gerät, P Menge zu einer LV-Position
  posnr TEXT NOT NULL DEFAULT '',
  text TEXT NOT NULL DEFAULT '',
  menge TEXT NOT NULL DEFAULT '0',
  einheit TEXT NOT NULL DEFAULT '',
  ep TEXT NOT NULL DEFAULT '0',
  mitarbeiter_id INTEGER REFERENCES mitarbeiter (id) ON DELETE SET NULL,
  artikel_id INTEGER REFERENCES artikel (id) ON DELETE SET NULL
);
CREATE INDEX regie_zeile_regie ON regie_zeile (regie_id, pos);

CREATE TABLE artikel (
  id INTEGER PRIMARY KEY,
  nummer TEXT NOT NULL DEFAULT '',
  suchtext TEXT NOT NULL DEFAULT '',
  text TEXT NOT NULL DEFAULT '',      -- erste Zeile = Kurztext
  gewerk TEXT NOT NULL DEFAULT '',
  einheit TEXT NOT NULL DEFAULT '',
  ek TEXT NOT NULL DEFAULT '0',
  vk1 TEXT NOT NULL DEFAULT '0',
  vk2 TEXT NOT NULL DEFAULT '0',
  vk3 TEXT NOT NULL DEFAULT '0',
  lohn TEXT NOT NULL DEFAULT '0',
  rabattgruppe TEXT NOT NULL DEFAULT '',
  rabatt TEXT NOT NULL DEFAULT '0',
  lieferant_id INTEGER REFERENCES kunde (id) ON DELETE SET NULL,
  lieferant TEXT NOT NULL DEFAULT '',
  lief_nr TEXT NOT NULL DEFAULT '',
  ean TEXT NOT NULL DEFAULT '',
  gewicht TEXT NOT NULL DEFAULT '',
  abmessung TEXT NOT NULL DEFAULT '',
  warengruppe TEXT NOT NULL DEFAULT '',
  preiseinheit TEXT NOT NULL DEFAULT '1',
  lager_aktiv INTEGER NOT NULL DEFAULT 0,
  bestand TEXT NOT NULL DEFAULT '0',
  mindestbestand TEXT NOT NULL DEFAULT '0',
  bestellmenge TEXT NOT NULL DEFAULT '0',
  lagerplatz TEXT NOT NULL DEFAULT '',
  datum_preis TEXT NOT NULL DEFAULT '',
  notiz TEXT NOT NULL DEFAULT '',
  quelle TEXT NOT NULL DEFAULT '',    -- manuell, ghfakt, datanorm
  alt_id TEXT NOT NULL DEFAULT ''
);
CREATE INDEX artikel_nummer ON artikel (nummer);
CREATE INDEX artikel_suchtext ON artikel (suchtext);

CREATE TABLE lager_bewegung (
  id INTEGER PRIMARY KEY,
  artikel_id INTEGER NOT NULL REFERENCES artikel (id) ON DELETE CASCADE,
  datum TEXT NOT NULL DEFAULT '',
  menge TEXT NOT NULL DEFAULT '0',    -- + Zugang, - Abgang
  text TEXT NOT NULL DEFAULT '',
  beleg_id INTEGER REFERENCES beleg (id) ON DELETE SET NULL
);
CREATE INDEX lager_bewegung_artikel ON lager_bewegung (artikel_id);

CREATE TABLE mitarbeiter (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  qualifikation TEXT NOT NULL DEFAULT '',
  stundensatz TEXT NOT NULL DEFAULT '0',  -- Verrechnungssatz
  kosten TEXT NOT NULL DEFAULT '0',       -- Kosten je Stunde
  aktiv INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE kalkulation (
  id INTEGER PRIMARY KEY,
  position_id INTEGER NOT NULL REFERENCES position (id) ON DELETE CASCADE,
  pos INTEGER NOT NULL,
  art TEXT NOT NULL DEFAULT 'L',      -- L Lohn (Minuten), M Material, G Gerät, F Fremdleistung, S Sonstiges
  text TEXT NOT NULL DEFAULT '',
  menge TEXT NOT NULL DEFAULT '0',
  einheit TEXT NOT NULL DEFAULT '',
  preis TEXT NOT NULL DEFAULT '0',
  zuschlag TEXT NOT NULL DEFAULT '',  -- leer = Zuschlag der Art aus den Einstellungen
  artikel_id INTEGER REFERENCES artikel (id) ON DELETE SET NULL
);
CREATE INDEX kalkulation_position ON kalkulation (position_id, pos);

CREATE TABLE zeit (
  id INTEGER PRIMARY KEY,
  projekt_id INTEGER NOT NULL REFERENCES projekt (id) ON DELETE CASCADE,
  mitarbeiter_id INTEGER REFERENCES mitarbeiter (id) ON DELETE SET NULL,
  datum TEXT NOT NULL DEFAULT '',
  stunden TEXT NOT NULL DEFAULT '0',
  posnr TEXT NOT NULL DEFAULT '',
  text TEXT NOT NULL DEFAULT ''
);
CREATE INDEX zeit_projekt ON zeit (projekt_id);

CREATE TABLE brief (
  id INTEGER PRIMARY KEY,
  kunde_id INTEGER NOT NULL REFERENCES kunde (id) ON DELETE CASCADE,
  projekt_id INTEGER REFERENCES projekt (id) ON DELETE SET NULL,
  datum TEXT NOT NULL DEFAULT '',
  adresse TEXT NOT NULL DEFAULT '',
  betreff TEXT NOT NULL DEFAULT '',
  text TEXT NOT NULL DEFAULT '',
  angelegt TEXT NOT NULL DEFAULT ''
);
CREATE INDEX brief_kunde ON brief (kunde_id);

CREATE TABLE textbaustein (
  id INTEGER PRIMARY KEY,
  kategorie TEXT NOT NULL DEFAULT '', -- vortext, nachtext, position, brief, zahlung
  name TEXT NOT NULL DEFAULT '',
  text TEXT NOT NULL DEFAULT ''
);
