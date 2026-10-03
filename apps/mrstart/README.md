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
  ihn in Claude Desktop oder Claude Code im Terminal (einstellbar); gesendet wird erst nach Bestätigung
- **Filter** über alle Projekte (<kbd>Strg</kbd>+<kbd>F</kbd>), <kbd>F5</kbd> aktualisiert
- Dunkles/helles Design, Akzentfarbe wählbar
- **Automatische Updates** über GitHub Releases (beim Start und alle 6 Stunden)

Die Konfiguration liegt in `%APPDATA%\de.mraudev.mrstart\config.json`.

## Technik

[Tauri 2](https://v2.tauri.app) (Rust) mit Vue 3, TypeScript, Tailwind CSS 4 und reka-ui.
Der Installer ist ein NSIS-Setup ohne Admin-Rechte (Installation pro Benutzer).

```
src/                  Vue-Frontend
  components/         Ansichten, Kacheln, Dialoge (ui/ = Basisbausteine)
  lib/                Store, API-Wrapper, Git-Konsole, Updater, Theme
src-tauri/            Rust-Backend
  src/config.rs       Laden/Speichern der Konfiguration
  src/launch.rs       Programme, Befehle, Explorer, Fork/TortoiseGit starten
  src/git.rs          Branch-Abfrage, Pull/Push mit gestreamter Ausgabe
  src/pulls.rs        Offene Pull Requests (GitHub/Gitea)
  src/projects.rs     Überwachte Ordner, Standard-Apps
.github/workflows/    CI (Build, Clippy, Tests) und Release
```

## Entwicklung

Voraussetzungen: Node.js 22, Rust (stable, MSVC), Visual Studio C++ Build Tools, WebView2.

```bash
npm install
npm run tauri dev
```

Tests und Prüfungen wie in der CI:

```bash
npm run build
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
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

**Einmalig – GitHub einrichten:** Unter *Settings → Environments* ein Environment `release` anlegen,
bei *Deployment branches and tags* nur Tags `v*` erlauben und dort zwei Environment-Secrets setzen:
`TAURI_SIGNING_PRIVATE_KEY` (Inhalt von `mrstart.key`) und `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

**Neue Version veröffentlichen:**

```bash
npm version minor        # oder patch/major – erhöht package.json, committet und taggt vX.Y.Z
git push --follow-tags
```

Der Workflow `release.yml` baut den Installer, signiert die Update-Pakete und veröffentlicht ein
GitHub-Release inklusive `latest.json`; die Commit-Nachrichten seit dem letzten Tag werden zu den
Release-Notizen, die auch im Update-Dialog erscheinen. Die App-Version kommt allein aus
`package.json` (`tauri.conf.json` verweist darauf).
