# mrfakt

Angebote, Rechnungen, Aufmaß und Kunden für den Handwerksbetrieb – Nachfolger der alten Branchensoftware
GHFakt (Malergeschäft Seydt), mit allen Altdaten.

## Funktionen

- **Kunden & Lieferanten** – Suche über Name, Ort, Nummer, Telefon; Anschrift, Kontakte, Bank, USt-IdNr.,
  Leitweg-ID, § 13b; PLZ → Ort; offene Posten und Umsatz je Kunde; Adressetiketten
- **Projekte (Aufträge)** mit allen Belegen, Regieberichten und Zeiten
- **Belege**: Angebot, Auftragsbestätigung, Lieferschein, Abschlags-, Schluss- und Rechnung, Gutschrift,
  Aufstellung. Vor-/Nachtext mit Formatierung und Textbausteinen, Positionen mit Titeln, Gewerken, Texten,
  Alternativ-/Bedarfspositionen und Zwischensummen, Positionsrabatt, Rabatt und Zu-/Abschläge (netto oder
  brutto), Zahlungsbedingungen mit Platzhaltern (`#ZN#`, `#SK#` …), Lohnanteil nach § 35a, § 13b,
  Abzug der Abschlagsrechnungen in der Schlussrechnung. Weiterführen (Angebot → Auftrag → Abschlag →
  Schlussrechnung, inkl. Aufmaß und Kalkulation), Festschreiben mit lückenloser Rechnungsnummer, Storno
- **PDF** im Briefpapier (DIN 5008, Logo, Fußzeile, Leistungsleiste), Vorschau, Speichern je Jahr in
  `Dokumente\mrfakt`
- **E-Rechnung**: Rechnungen als ZUGFeRD 2 / Factur-X (Profil EN 16931, PDF/A-3 mit eingebetteter XML),
  XML auch einzeln exportierbar
- **Aufmaß** je Position mit Formelkatalog (Rechteck, Trapez, Zylinder …), Rechenausdrücken in den Maßen,
  Abzügen; Mengen in den Beleg übernehmen; Aufmaßblatt drucken
- **Regieberichte** (Text, Lohn je Mitarbeiter, Material, Gerät, Mengen zu LV-Positionen), drucken und als
  Regierechnung abrechnen
- **Rechnungsbuch**: offene/überfällige/bezahlte Belege, Zahlungen mit Skonto/Abzug, Zahlungserinnerung und
  drei Mahnstufen mit Gebühr und Frist, CSV-Export
- **Auswertung**: Umsatz je Monat, Jahr und Kunde, offene Posten, Angebote
- **Artikel & Lager**: Artikelstamm mit EK/VK, Lagerbestand, Mindestbestand, Bestellvorschlag, Abbuchung über
  Lieferscheine, Etiketten, DATANORM-4/5-Import
- **Kalkulation** je Position (Lohnminuten, Material, Gerät, Fremdleistung mit Zuschlägen) → EP und
  Lohnanteil; Mitarbeiter mit Stundensätzen; **Nachkalkulation** (Ist-Stunden gegen Kalkulation)
- **GAEB**: Leistungsverzeichnisse als GAEB DA XML (X81/X83/X84/X86) oder GAEB 90 (D81/D83) einlesen,
  Angebote als X84 ausgeben
- **Briefe** an Kunden im Briefpapier
- Globale Suche (<kbd>Strg</kbd>+<kbd>K</kbd>), Speichern (<kbd>Strg</kbd>+<kbd>S</kbd>), Drucken
  (<kbd>Strg</kbd>+<kbd>P</kbd>), helles/dunkles Design, automatische Updates

## Daten

SQLite-Datenbank `%APPDATA%\de.mraudev.mrfakt\mrfakt.db` (Pfad in den Einstellungen änderbar). Beträge und
Mengen sind Dezimalzahlen (kein Fließkomma). Beim Beenden wird automatisch nach `Sicherungen\` neben der
Datenbank gesichert (die letzten 30); Sicherungen lassen sich in den Einstellungen zurückspielen.

## Übernahme aus GHFakt

*Einstellungen → Daten & GHFakt-Import* liest einen GHFakt-Ordner (z. B. `C:\Entwicklung\GHFakt`) nur lesend:

| Quelle | Inhalt |
|---|---|
| `KUNDEN\Kunden02.dat`, `Buch_02.dat`, `PLZ_NEU.mdb`, `ARTIKEL\Artikl03.Dat` | Access (Jet 4): Kunden, Rechnungsbuch, PLZ, Artikel |
| `PROJEKTE\G<Kunde>\AUFTRAGI.DAT` | Projektliste je Kunde |
| `PROJEKTE\G<Kunde>\<Auftrag>VP/LV/NPN/NP/EZ/AU.0xx` | Belege: Vorspann, Positionen, Nachspann, Abschläge, Aufmaß (Endung = LV-Slot) |
| `PROJEKTE\G<Kunde>\<Auftrag>Re.MDB` | Regieberichte (Jet 3) |
| `PROJEKTE\G<Kunde>\00000GHT.0xx` | Briefe |
| `EINSTELL\*` | Absender, Briefkopf, Steuernummer, Zahlungs- und Mahntexte, Einheiten, Nummernkreise, Textbausteine, Aufmaß-Formeln |
| `logo.jpg`, `fusszeile.jpg` | Briefpapier |

Die Access-Dateien liest ein PowerShell-Skript über den 32-bit-Provider `Microsoft.Jet.OLEDB.4.0`
(ACE öffnet diese alten Formate nicht), alles andere liest Rust direkt. Der Import ersetzt alle Daten in
mrfakt (vorher wird gesichert) und zeigt einen Bericht: Anzahlen und Belege, deren neu berechneter Betrag
vom GHFakt-Rechnungsbuch abweicht – beim Altbestand 36 von 9.853, fast alle DM-Belege oder von Hand
geänderte Beträge. Im Rechnungsbuch gilt immer der Betrag aus GHFakt; übernommene Rechnungen sind
festgeschrieben.

## Technik

[Tauri 2](https://v2.tauri.app) mit Vue 3, TypeScript, Tailwind CSS 4 und reka-ui – Aufbau wie
[mrstart](../mrstart). Rust: `rusqlite` (SQLite, Migrationen über `PRAGMA user_version`), `rust_decimal`,
Typst als Bibliothek für die PDFs (Vorlagen in `src-tauri/src/pdf/vorlagen`), `lopdf` für die
Factur-X-Metadaten, `quick-xml` für GAEB. Texte bearbeitet der gemeinsame Editor `@mrtools/textedit`.

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrfakt`). Der vollständige Import eines echten GHFakt-Ordners läuft als Test nur
auf Wunsch:

```bash
MRFAKT_GHFAKT='C:\Entwicklung\GHFakt' cargo test -p mrfakt voller_import -- --ignored --nocapture
```
