# mrremotemanager

Remote-Verbindungen speichern und mit einem Doppelklick öffnen.

## Funktionen

- **Verbindungsarten**:
  - **Remotedesktop** – öffnet `mstsc.exe` mit einer `.rdp`-Datei: Host, Port, Benutzer, Vollbild oder feste
    Fenstergröße, alle Monitore, Zwischenablage und Laufwerke durchreichen, RD-Gateway, Admin-Sitzung (`/admin`)
  - **SSH** – öffnet `ssh.exe` in einem neuen Konsolenfenster (bzw. Windows Terminal, wenn es das
    Standard-Terminal ist): Port, Benutzer, Schlüsseldatei, Jump-Host (`-J`), Portweiterleitungen (`-L`/`-R`/`-D`).
    Mit installiertem WinSCP zusätzlich „SFTP“ für die Dateien des Servers
  - **Web** – öffnet die Adresse im Standardbrowser (ohne Schema wird `https://` ergänzt)
  - **VNC** – im installierten Viewer (TigerVNC, RealVNC, UltraVNC oder TightVNC)
  - **Telnet** und **Seriell** (COM-Schnittstelle mit Baudrate) – in PuTTY
  - **Hyper-V** – öffnet die VM in `vmconnect.exe` (mit UAC-Abfrage, falls nötig)
- **Passwörter für Remotedesktop** landen in der Windows-Anmeldeinformationsverwaltung (`TERMSRV/<host>`, wie
  „Anmeldedaten speichern“ in Remotedesktop) – nie in der Datei von mrremotemanager und nie in der Oberfläche
- **Erreichbarkeit**: alle 30 s ein kurzer Verbindungsversuch je Host; graue Punkte zeigen, was nicht antwortet
- **Aktive Verbindungen**: offene Sitzungen erscheinen oben unter „Aktiv – nicht gespeichert“ und lassen sich mit
  „Speichern“ übernehmen. Erkannt werden `mstsc.exe` (Host aus Befehlszeile, `.rdp`-Datei oder Fenstertitel,
  Benutzer aus dem Remotedesktop-Verlauf), `ssh.exe` (aus der Befehlszeile) und andere Programme mit einer
  Verbindung zu Port 3389, 22, 23 oder 5900–5909 (PuTTY, WinSCP, VNC-Viewer, …). Gespeicherte Verbindungen, die
  gerade offen sind, haben einen grünen Punkt – auch wenn das Programm nur die IP kennt (der gespeicherte Name wird
  aufgelöst). Offene Sitzungen lassen sich in den Vordergrund holen oder beenden
- **Import von diesem PC**: Remotedesktop-Verlauf, `~/.ssh/config`, PuTTY-Sitzungen und `.rdp`-Dateien, mit
  Auswahl; schon gespeicherte werden erkannt
- **Sicherung**: alle Verbindungen als JSON exportieren und wieder importieren (ergänzt fehlende)
- **Favoriten** oben in der Liste, Sortierung nach Name oder „Zuletzt verwendet“
- **Gruppen** mit Vorschlägen beim Eingeben, Notizen je Verbindung
- **Suche** nach Name, Host, Benutzer, Gruppe oder Notiz (<kbd>Strg</kbd>+<kbd>F</kbd>)
- **Aktionen**: Verbinden (<kbd>Enter</kbd> oder Doppelklick), Neu (<kbd>Strg</kbd>+<kbd>N</kbd>),
  Bearbeiten (<kbd>F2</kbd>), Duplizieren, Adresse kopieren, Löschen (<kbd>Entf</kbd>, mit Rückfrage)
- Dunkles/helles Design
- **Automatische Updates**: Prüfung beim Start und alle 4 Stunden, stiller Download, danach „Neu starten“
  oder Installation beim Beenden

Die Verbindungen liegen in `%APPDATA%\de.mraudev.mrremotemanager\connections.json`.

## Technik

[Tauri 2](https://v2.tauri.app) (Rust: `GetExtendedTcpTable` aus `iphlpapi`, `sysinfo` für die Befehlszeilen,
`CredWrite`/`CredRead` für Passwörter, die Registry für den Import) mit Vue 3, TypeScript, Tailwind CSS 4 und
reka-ui – Aufbau und Aussehen wie [mrstart](../mrstart). Alle Werte werden vor dem Start geprüft, damit nichts aus
einer `.rdp`-Zeile ausbrechen, als Option von ssh/PuTTY gelesen werden oder aus einem Argument von vmconnect
ausbrechen kann.

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrremotemanager`). Der Test, der wirklich ein Passwort in Windows speichert (und
wieder entfernt), läuft nur auf Wunsch: `cargo test -p mrremotemanager -- --ignored`.
