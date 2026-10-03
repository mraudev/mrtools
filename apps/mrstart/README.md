# mrstart

Desktop-Launcher für Entwicklungsprojekte – der Nachfolger von radstart.

## Funktionen

- **Projektkacheln** gruppiert nach Kategorie (jede Kategorie ist ein Tab), sortiert nach Version
- **Überwachte Ordner** – jeder Unterordner erscheint automatisch als Kachel
- **Schnellaktionen** je Projekt: Editor, Explorer, Git Bash, Terminal (Befehle in den Einstellungen anpassbar)
- **Apps** (`.sln`, `.exe`, `.bat` …) und **eigene Befehle** pro Projekt; Standard-Apps per Muster wie `*.sln`
- **Git**: aktueller Branch, Pull (`fetch` + `rebase --rebase-merges --autostash`) und Push mit Live-Konsole,
  Fork bzw. TortoiseGit für Änderungen/Log
- **Offene Pull Requests** des ausgecheckten Branches (GitHub und ein Gitea-Server)
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

## Release & Auto-Update

**Einmalig:** Im Repository unter *Settings → Secrets and variables → Actions* das Secret
`TAURI_SIGNING_PRIVATE_KEY` mit dem Inhalt von `%USERPROFILE%\.tauri\mrstart.key` anlegen.
Hat der Schlüssel ein Passwort, zusätzlich `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

> Den privaten Schlüssel sicher aufbewahren: Ohne ihn lassen sich keine Updates mehr signieren,
> und installierte Versionen würden nicht mehr aktualisiert. Der öffentliche Schlüssel steht in
> `src-tauri/tauri.conf.json`.

**Neue Version veröffentlichen:**

```bash
npm version minor        # oder patch/major – erhöht package.json, committet und taggt vX.Y.Z
git push --follow-tags
```

Der Workflow `release.yml` baut den Installer, signiert die Update-Pakete und veröffentlicht ein
GitHub-Release inklusive `latest.json`; die Commit-Nachrichten seit dem letzten Tag werden zu den
Release-Notizen, die auch im Update-Dialog erscheinen. Die App-Version kommt allein aus
`package.json` (`tauri.conf.json` verweist darauf).
