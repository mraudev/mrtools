# mrprocs

Zeigt alle laufenden Prozesse – ähnlich wie Task-Manager oder Process Explorer.

## Funktionen

- **Prozessliste** mit Symbol, PID, CPU, Arbeitsspeicher (privater Arbeitssatz wie im Task-Manager),
  Datenträger-Durchsatz, Threads und Benutzer; nach jeder Spalte sortierbar, Lastwerte farblich hinterlegt
- **Baumansicht** mit Eltern-/Kindprozessen (<kbd>Strg</kbd>+<kbd>T</kbd>), auf- und zuklappbar
- **Suche** nach Name, PID, Pfad, Befehlszeile oder Benutzer (<kbd>Strg</kbd>+<kbd>F</kbd>)
- Neue Prozesse leuchten kurz grün auf, beendete rot
- **Live-Aktualisierung** alle 0,5 / 1 / 2 / 5 s, anhaltbar; <kbd>F5</kbd> aktualisiert sofort
- **Details**: Verlauf von CPU und Arbeitsspeicher, Pfad, Befehlszeile, Elternprozess, Startzeit, Laufzeit,
  CPU-Zeit, Priorität, erhöhte Rechte, Threads, Handles, Speicherkennzahlen
- **Module** (geladene DLLs) und **Netzwerkverbindungen** (TCP/UDP, IPv4/IPv6) des Prozesses
- **Aktionen** (Kontextmenü, Detailbereich, Tastatur):
  - Prozess beenden (<kbd>Entf</kbd>) oder ganze Prozessstruktur beenden (<kbd>⇧</kbd>+<kbd>Entf</kbd>),
    mit Rückfrage und Warnung bei Windows-Systemprozessen
  - Anhalten und Fortsetzen
  - Priorität ändern
  - Dateipfad öffnen, Eigenschaften, online suchen, Name/PID/Pfad/Befehlszeile kopieren
- **Systemübersicht**: CPU- und Speicherverlauf, Zahl der Prozesse, Threads und Handles, Spitzenreiter
- Neustart **als Administrator** per Knopfdruck – nötig für Details und Aktionen bei Systemprozessen
- Dunkles/helles Design

## Technik

[Tauri 2](https://v2.tauri.app) (Rust, `sysinfo` plus Windows-APIs aus `ntdll`, `iphlpapi` und `shell32`)
mit Vue 3, TypeScript, Tailwind CSS 4 und reka-ui – Aufbau und Aussehen wie [mrstart](https://github.com/mraudev/mrstart).

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrprocs`).
