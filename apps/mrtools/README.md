# mrtools

Übersicht und Starter für die eigenen Apps.

## Funktionen

- **Jeder Unterordner** des App-Ordners (Standard: der Ordner, in dem das `mrtools`-Repository liegt)
  erscheint als Kachel mit Logo, Name und Beschreibung aus `package.json` bzw. `src-tauri/tauri.conf.json`
- **Monorepos** (Ordner mit npm-Workspaces wie dieses Repository) werden aufgeklappt: jede Tauri-App darin
  bekommt eine eigene Kachel, ihr Release wird über das Tag-Präfix `<app>-v` gefunden
- **Versionen**: installiert (aus den Windows-Deinstallationseinträgen), Quelle (`package.json`) und
  letztes **GitHub-Release**; Status *Aktuell*, *Update verfügbar* oder *Nicht installiert*
- **Starten** installierter Apps, **GitHub-Link** (aus dem `origin`-Remote oder `repository` in
  `package.json`), Ordner im Explorer öffnen
- **Filter** (<kbd>Strg</kbd>+<kbd>F</kbd>), <kbd>F5</kbd> aktualisiert, App-Ordner wählbar
- Dunkles/helles Design

## Technik

[Tauri 2](https://v2.tauri.app) (Rust) mit Vue 3, TypeScript, Tailwind CSS 4 und reka-ui – Aufbau und
Aussehen wie [mrstart](https://github.com/mraudev/mrstart).

```
src/                  Vue-Frontend
src-tauri/src/
  apps.rs             Ordner scannen, package.json/tauri.conf.json/Git-Remote lesen
  installed.rs        Installierte Apps aus der Registry, Starten
  github.rs           Letztes Release (öffentliche GitHub-API, ohne Token)
```

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrtools`).
