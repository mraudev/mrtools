# mrremotemanager

Remote-Verbindungen speichern und mit einem Doppelklick öffnen.

## Funktionen

- **Verbindungsarten**:
  - **Remotedesktop** – öffnet `mstsc.exe` mit einer `.rdp`-Datei (Host, Port, Benutzer, Vollbild, alle Monitore)
  - **SSH** – öffnet `ssh.exe` in einem neuen Konsolenfenster (bzw. Windows Terminal, wenn es das
    Standard-Terminal ist), optional mit Port, Benutzer und Schlüsseldatei
  - **Web** – öffnet die Adresse im Standardbrowser (ohne Schema wird `https://` ergänzt)
- **Aktive Verbindungen**: offene Remotedesktop- und SSH-Sitzungen erscheinen oben unter „Aktiv – nicht
  gespeichert“ und lassen sich mit „Speichern“ übernehmen (vorausgefüllt mit Host, Port, Benutzer und Schlüssel).
  Erkannt werden `mstsc.exe` (Host aus Befehlszeile, `.rdp`-Datei oder Fenstertitel, Benutzer aus dem
  Remotedesktop-Verlauf), `ssh.exe` (aus der Befehlszeile) und andere Programme mit einer Verbindung zu Port 3389
  oder 22 (PuTTY, WinSCP, …, dann mit IP-Adresse). Gespeicherte Verbindungen, die gerade offen sind, haben einen
  grünen Punkt. Aktualisierung alle 3 s
- **Gruppen** (z. B. „Arbeit“, „Zuhause“) mit Vorschlägen beim Eingeben, Notizen je Verbindung
- **Suche** nach Name, Host, Benutzer, Gruppe oder Notiz (<kbd>Strg</kbd>+<kbd>F</kbd>)
- **Aktionen**: Verbinden (<kbd>Enter</kbd> oder Doppelklick), Neu (<kbd>Strg</kbd>+<kbd>N</kbd>),
  Bearbeiten (<kbd>F2</kbd>), Duplizieren, Adresse kopieren, Löschen (<kbd>Entf</kbd>, mit Rückfrage)
- „Zuletzt verbunden“ je Verbindung
- Dunkles/helles Design
- **Automatische Updates**: Prüfung beim Start und alle 4 Stunden, stiller Download, danach „Neu starten“
  oder Installation beim Beenden

**Passwörter** speichert mrremotemanager nicht. Remotedesktop merkt sie sich auf Wunsch selbst
(„Anmeldedaten speichern“ in der Windows-Anmeldeinformationsverwaltung), für SSH am besten einen Schlüssel nutzen.

Die Verbindungen liegen in `%APPDATA%\de.mraudev.mrremotemanager\connections.json`.

## Technik

[Tauri 2](https://v2.tauri.app) (Rust, `GetExtendedTcpTable` aus `iphlpapi`, `sysinfo` für die Befehlszeilen) mit Vue 3, TypeScript, Tailwind CSS 4 und reka-ui – Aufbau und Aussehen wie
[mrstart](../mrstart). Host, Benutzer und Schlüsseldatei werden vor dem Start geprüft, damit nichts aus einer
`.rdp`-Zeile ausbrechen oder als ssh-Option gelesen werden kann.

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrremotemanager`).
