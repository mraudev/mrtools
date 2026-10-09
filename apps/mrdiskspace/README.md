# mrdiskspace

Zeigt, wo der Plattenplatz bleibt – ähnlich wie TreeSize.

## Funktionen

- **Laufwerksübersicht** mit Belegung, oder beliebigen **Ordner** wählen (<kbd>Strg</kbd>+<kbd>O</kbd>)
- **Schneller paralleler Scan** mit Live-Fortschritt, jederzeit abbrechbar; <kbd>F5</kbd> scannt neu
- **Verzeichnisbaum** nach Größe sortiert: Größe, Anteil am übergeordneten Ordner, Zahl der Dateien und
  Ordner, letzte Änderung; Bedienung auch per Tastatur (↑/↓, ←/→ zum Auf- und Zuklappen)
- **Treemap** des gewählten Ordners – Klick auf eine Kachel springt hinein
- **Dateitypen**: Platz je Dateiendung
- **Größte Dateien** unterhalb des gewählten Ordners, Klick zeigt die Datei im Baum
- Kontextmenü: im Explorer zeigen, Pfad kopieren, Ordner einzeln scannen
- Nicht lesbare Ordner (fehlende Rechte) werden markiert und gezählt
- Dunkles/helles Design

Gezählt wird die Dateigröße (nicht der belegte Platz auf der Platte). Symbolische Links und
Junctions werden übersprungen, Hardlinks (z. B. in `C:\Windows\WinSxS`) mehrfach gezählt – die Summe
kann deshalb über dem belegten Platz des Laufwerks liegen.

## Technik

[Tauri 2](https://v2.tauri.app) (Rust, rayon) mit Vue 3, TypeScript, Tailwind CSS 4 und reka-ui –
Aufbau und Aussehen wie [mrstart](https://github.com/mraudev/mrstart). Der Scan-Baum bleibt im Rust-Prozess; das Frontend lädt
Ordner nur beim Aufklappen.

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrdiskspace`).
