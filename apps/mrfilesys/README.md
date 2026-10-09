# mrfilesys

Dateien und Ordner durchsuchen und verwalten – ähnlich wie der Windows-Explorer.

## Funktionen

- **Seitenleiste** mit „Dieser PC“, eigenen **Favoriten**, Schnellzugriff (Desktop, Downloads, Dokumente, …)
  und Laufwerken mit Belegung
- **Favoriten** für Ordner und Dateien: per Kontextmenü oder durch Ziehen auf „Favoriten“ bzw. zwischen zwei
  Favoriten anheften (auch aus dem Explorer), per Ziehen umsortieren. Mitten auf einen Favoriten-Ordner gezogen
  wird wie gewohnt hineinverschoben bzw. -kopiert
- **Adressleiste** mit Breadcrumbs – Klick ins Feld (<kbd>Strg</kbd>+<kbd>L</kbd>) zum Eintippen eines Pfads,
  auch mit Umgebungsvariablen wie `%appdata%`, `%temp%` oder `%userprofile%\Desktop`
- **Liste** (sortierbar nach Name, Datum, Typ, Größe) oder **Kacheln**; auch sehr große Ordner bleiben flüssig
- **Filter** im aktuellen Ordner (<kbd>Strg</kbd>+<kbd>F</kbd>), versteckte Dateien ein-/ausblenden (<kbd>Strg</kbd>+<kbd>H</kbd>)
- **Detailbereich** (<kbd>Alt</kbd>+<kbd>P</kbd>) mit Vorschau für Bilder, Videos, Audio und Text, Eigenschaften
  und Berechnung der Ordnergröße
- **Kontextmenü**: Öffnen, Öffnen mit …, Terminal hier öffnen, Im Explorer zeigen, Favoriten, Ausschneiden/Kopieren/
  Einfügen, Pfad kopieren, Umbenennen, Papierkorb, endgültig löschen, Windows-Eigenschaften
- **Neu**: Ordner und Textdokument, direkt im Umbenennen-Modus
- **Endgültig löschen** (<kbd>Umschalt</kbd>+<kbd>Entf</kbd>) ohne vorheriges Durchzählen und parallel – bei sehr vielen
  Dateien (z. B. `node_modules`) deutlich schneller als im Explorer. Fortschritt und Abbrechen in der Statusleiste;
  schreibgeschützte Dateien werden mitgelöscht, Verknüpfungen (Symlinks, Junctions) nur als Verweis entfernt
- **Schnelles Kopieren und Verschieben** (Einfügen, Drag & Drop): parallel statt Datei für Datei – bei vielen
  kleinen Dateien rund 20-mal schneller als im Explorer. Verschieben auf demselben Laufwerk ist ein Umbenennen,
  auf ein anderes Laufwerk wird kopiert und die Quelle nur gelöscht, wenn alles geklappt hat. Gibt es Namen im
  Ziel schon, wird einmal gefragt: Ersetzen (Ordner zusammenführen), Überspringen oder Beide behalten.
  Einfügen in denselben Ordner legt „x - Kopie“ an. Fortschritt mit „x von y Dateien“ und Abbrechen in der Statusleiste
- Papierkorb über Windows selbst (wiederherstellbar). Die **Zwischenablage ist mit dem Explorer geteilt**:
  in mrfilesys kopieren, im Explorer einfügen und umgekehrt
- **Drag & Drop** wie im Explorer: auf Ordner in der Liste, in der Seitenleiste oder in der Adressleiste ziehen –
  auf demselben Laufwerk wird verschoben, sonst kopiert (<kbd>Strg</kbd> kopiert, <kbd>Umschalt</kbd> verschiebt).
  Aus dem Fenster heraus in Explorer, Desktop oder andere Apps ziehen, und Dateien von dort hineinziehen
- Ordner werden beim Zurückkehren ins Fenster neu eingelesen
- Dunkles/helles Design
- **Automatische Updates**: Prüfung beim Start und alle 4 Stunden, stiller Download, danach „Neu starten“
  oder Installation beim Beenden

## Tastatur

| Taste                              | Aktion                               |
| ---------------------------------- | ------------------------------------ |
| ↑ ↓ ← → Pos1 Ende Bild↑ Bild↓      | Auswahl bewegen (mit Umschalt: erweitern) |
| Buchstaben                         | Zum Eintrag springen                 |
| Enter / Rücktaste                  | Öffnen / übergeordneter Ordner       |
| Alt+← / Alt+→ / Alt+↑, Maustasten 4/5 | Zurück / Vorwärts / Nach oben     |
| F2                                 | Umbenennen                           |
| Entf / Umschalt+Entf               | Papierkorb / endgültig löschen       |
| Strg+C / Strg+X / Strg+V           | Kopieren / Ausschneiden / Einfügen   |
| Strg+Umschalt+C                    | Pfad kopieren                        |
| Strg+Umschalt+N                    | Neuer Ordner                         |
| Strg+A, Strg+T, Alt+Enter, F5      | Alles auswählen, Terminal, Eigenschaften, Aktualisieren |

## Technik

[Tauri 2](https://v2.tauri.app) (Rust, Windows-Shell-API über `windows-sys`) mit Vue 3, TypeScript,
Tailwind CSS 4 und reka-ui – Aufbau und Aussehen wie [mrstart](../mrstart). Die Vorschau lädt Dateien über
das Asset-Protokoll von Tauri.

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrfilesys`).
