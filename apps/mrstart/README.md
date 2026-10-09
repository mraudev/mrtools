# mrstart

Desktop-Launcher für Entwicklungsprojekte – der Nachfolger von radstart.

## Funktionen

- **Projektkacheln** gruppiert nach Kategorie (jede Kategorie ist ein Tab), sortiert nach Version
- **Überwachte Ordner** (beliebig viele) – jeder Unterordner erscheint automatisch als Kachel im Tab
  des Ordners; mit dem Tab-Namen einer Projekt-Kategorie teilen sie sich einen Tab mit den Projekten
- **Schnellaktionen** je Projekt: Editor, Explorer, Git Bash, Terminal (Befehle in den Einstellungen anpassbar)
- **Apps** (`.sln`, `.exe`, `.bat` …) und **eigene Befehle** pro Projekt; Standard-Apps per Muster wie `*.sln`
- **Git**: aktueller Branch, Pull (`fetch` + `rebase --rebase-merges --autostash`) und Push mit Live-Konsole,
  Fork bzw. TortoiseGit für Änderungen/Log
- **Pull Requests in der Kachel**: offener PR des ausgecheckten Branches öffnen bzw. neuen PR im
  Browser erstellen (GitHub und ein Gitea-Server)
- **Pull-Request-Dashboard** (Gitea und GitHub): eigene offene PRs mit Status (veraltet, Konflikte,
  aktuell …), Branch per Merge oder Rebase aktualisieren, Liste der angeforderten Reviews; dazu
  Kennzahlen und Diagramme (Status, Alter, Verteilung auf Repositories) mit Tabellenansicht
- **Review mit Claude**: bereitet für einen angeforderten Review einen Auftrag samt Diff vor und öffnet
  ihn in Claude Desktop (vorausgefüllt, Senden nach Bestätigung) oder startet Claude Code im Terminal im
  Auto-Modus (einstellbar); bisherige Reviews und Kommentare des PRs werden berücksichtigt
- **Filter** über alle Projekte (<kbd>Strg</kbd>+<kbd>F</kbd>, ↑/↓ wählen, <kbd>Enter</kbd> öffnet im Editor,
  <kbd>Umschalt</kbd>+<kbd>Enter</kbd> im Terminal), <kbd>F5</kbd> aktualisiert
- **Favoriten** (Stern) oben im Tab, **Tabs** per Drag & Drop sortieren und per Doppelklick umbenennen,
  kompakte Kachelansicht
- In der Kachel: Abgleich mit dem Remote (↑/↓), Zahl der ungesicherten Änderungen, Hinweis bei
  fehlendem Ordner oder Git-Problemen (Klick kopiert die Lösung)
- Optionales **automatisches Fetch** im Hintergrund (standardmäßig aus), **Benachrichtigung** bei neuen
  Review-Anfragen, **CI-Status** je Pull Request, **Filter** im Dashboard
- Nach einem Update zeigt mrstart die **Änderungen der neuen Version**; Konfiguration **exportieren und
  importieren** (ohne Tokens); nur eine laufende Instanz
- Dunkles/helles Design, Akzentfarbe wählbar
- **Automatische Updates** über GitHub Releases (beim Start und alle 6 Stunden)

Die Konfiguration liegt in `%APPDATA%\de.mraudev.mrstart\config.json`.

## Technik

[Tauri 2](https://v2.tauri.app) (Rust) mit Vue 3, TypeScript, Tailwind CSS 4 und reka-ui. mrstart liegt im
Monorepo [mraudev/mrtools](https://github.com/mraudev/mrtools) und nutzt dessen gemeinsame Oberfläche
`@mrtools/ui`; bis Version 1.7.0 lag es in [mraudev/mrstart](https://github.com/mraudev/mrstart).
Der Installer ist ein NSIS-Setup ohne Admin-Rechte (Installation pro Benutzer).

```
src/                  Vue-Frontend
  components/         Ansichten, Kacheln, Dialoge (ui/ = eigene Basisbausteine)
  lib/                Store, API-Wrapper, Git-Konsole, Updater, Theme
src-tauri/            Rust-Backend
  src/config.rs       Laden/Speichern der Konfiguration
  src/launch.rs       Programme, Befehle, Explorer, Fork/TortoiseGit starten
  src/git.rs          Branch-Abfrage, Pull/Push mit gestreamter Ausgabe
  src/pulls.rs        Offene Pull Requests (GitHub/Gitea)
  src/projects.rs     Überwachte Ordner, Standard-Apps
```

## Entwicklung

Siehe [README im Repository-Root](../../README.md):

```bash
npm install                          # im Repository-Root
npm run tauri dev -w apps/mrstart
```

## Sicherheit

- **Tokens** (GitHub/Gitea) liegen verschlüsselt in der Windows-Anmeldeinformationsverwaltung
  (nur auf diesem PC), nie in der Konfigurationsdatei. Die Oberfläche kann Tokens nur setzen oder
  löschen, nicht auslesen. Unbekannte Felder in der Konfiguration (z. B. Tokens aus einer
  radstart-Datei) werden beim Laden verworfen.
- Tokens gehen ausschließlich per HTTPS an `api.github.com` bzw. den eingetragenen Gitea-Host,
  ohne Weiterleitungen. Owner/Repo aus den Git-Remotes werden validiert, PR-Links nur geöffnet,
  wenn sie auf denselben Host zeigen.
- Strikte Content-Security-Policy, keine entfernten Skripte, keine HTML-Ausgabe von Fremddaten.
- Updates werden nur installiert, wenn ihre Signatur zum eingebauten öffentlichen Schlüssel passt.
- Die GitHub-Actions sind auf Commit-SHAs festgelegt, laufen mit minimalen Rechten, ohne
  Install-Skripte (`npm ci --ignore-scripts`) und – im Release – ohne Build-Cache. Nur der
  eigentliche Build-Schritt sieht den Signaturschlüssel.

## Release & Auto-Update

**Einmalig – Signaturschlüssel (mit Passwort) erzeugen:**

```bash
npx tauri signer generate -f -w C:\Users\MRau9\.tauri\mrstart.key
```

Den Inhalt von `mrstart.key.pub` als `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`
eintragen. Schlüsseldatei und Passwort zusätzlich im Passwort-Manager sichern – ohne sie lassen
sich keine Updates mehr signieren und installierte Versionen würden nicht mehr aktualisiert.
Danach kann die lokale Schlüsseldatei gelöscht werden.

**Einmalig – GitHub einrichten:** Im Repository `mraudev/mrtools` unter *Settings → Environments* ein
Environment `release` anlegen, bei *Deployment branches and tags* nur Tags `*-v*` erlauben und dort zwei
Environment-Secrets setzen: `TAURI_SIGNING_PRIVATE_KEY` (Inhalt von `mrstart.key`) und
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Der Release-Workflow gibt sie nur beim Release von mrstart weiter.

**Neue Version veröffentlichen** (im Repository-Root):

```bash
npm run release -- mrstart minor   # oder patch/major – erhöht package.json, committet und taggt mrstart-vX.Y.Z
git push --follow-tags
```

Der Workflow `release.yml` baut den Installer, signiert die Update-Pakete und veröffentlicht das
GitHub-Release `mrstart X.Y.Z` inklusive `latest.json`. Die `latest.json` kopiert er zusätzlich in das
feste Release `mrstart-latest`; dort fragen installierte Versionen nach Updates:
`https://github.com/mraudev/mrtools/releases/download/mrstart-latest/latest.json`. Die
Commit-Nachrichten seit dem letzten Release, die `apps/mrstart` oder `packages/ui` betreffen, werden zu
den Release-Notizen, die auch im Update-Dialog erscheinen. Die App-Version kommt allein aus
`package.json` (`tauri.conf.json` verweist darauf).

**Übergang vom alten Repository (einmalig, beim ersten Release aus dem Monorepo):** Versionen bis 1.7.0
fragen noch `mraudev/mrstart` nach Updates. Damit sie das neue Release finden, dessen `latest.json` dort
als Release anlegen, z. B. für 1.8.0:

```bash
gh release download mrstart-v1.8.0 --repo mraudev/mrtools --pattern latest.json
gh release create v1.8.0 latest.json --repo mraudev/mrstart --title "mrstart 1.8.0" --notes "mrstart ist nach https://github.com/mraudev/mrtools umgezogen."
```

Die Download-Adressen in der `latest.json` zeigen auf `mraudev/mrtools`. Nach dem Update auf 1.8.0
fragen die Installationen nur noch den neuen Update-Kanal ab, und `mraudev/mrstart` kann archiviert werden.
