# mrports

Zeigt, welcher Prozess welchen Port belegt – Schluss mit „Port 1420 is already in use“.

## Funktionen

- **Lauschende Ports** (TCP und UDP) mit Prozess, PID, Symbol und dem **Ordner, in dem der Prozess läuft** –
  bei `node.exe`, `python.exe` & Co. meist das Projekt. IPv4 und IPv6 desselben Ports stehen in einer Zeile
- **Erreichbar**: „Nur dieser PC“ (127.0.0.1/::1) oder „Alle Netzwerke“ (0.0.0.0/::) – aus dem Netz
  erreichbare Ports sind hervorgehoben
- **Dienst**-Hinweis für bekannte Ports (PostgreSQL, Redis, Vite, Remotedesktop, …)
- **Alle Verbindungen** mit Gegenstelle und Status als zweite Ansicht
- **Suche** nach Port, Programm, Ordner, Befehlszeile oder Adresse (<kbd>Strg</kbd>+<kbd>F</kbd>);
  eine freie Portnummer wird als „Port … ist frei“ angezeigt
- **Aktionen**: Prozess beenden (<kbd>Entf</kbd>, mit Rückfrage und Liste der Ports, die dabei frei werden),
  `http://localhost:<port>` im Browser öffnen (<kbd>Enter</kbd> oder Doppelklick), Ordner im Explorer zeigen,
  Port, PID oder Befehlszeile kopieren
- **Details** zum Prozess: Programm, Ordner, Befehlszeile und alle seine Ports
- Live-Aktualisierung alle 2 s, anhaltbar; <kbd>F5</kbd> aktualisiert sofort
- Neustart **als Administrator** per Knopfdruck – nötig, um Prozesse anderer Benutzer und Dienste zu beenden
  und ihre Ordner zu sehen
- Dunkles/helles Design
- **Automatische Updates**: Prüfung beim Start und alle 4 Stunden, stiller Download, danach „Neu starten“
  oder Installation beim Beenden

## Technik

[Tauri 2](https://v2.tauri.app) (Rust, `GetExtendedTcpTable`/`GetExtendedUdpTable` aus `iphlpapi`, `sysinfo` für
Befehlszeile und Arbeitsordner) mit Vue 3, TypeScript, Tailwind CSS 4 und reka-ui – Aufbau und Aussehen wie
[mrstart](../mrstart).

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrports`).
