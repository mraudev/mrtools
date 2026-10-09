# mrtools

Installiert, aktualisiert und startet die Apps aus diesem Repository.

## Funktionen

- **Alle Apps des Repositorys** (außer mrtools selbst) als Kacheln mit Logo, Name und Beschreibung. Die
  Liste wird beim Bauen aus `apps/` übernommen – mrtools braucht keinen Quellcode auf der Platte.
- **Versionen**: installiert (aus den Windows-Deinstallationseinträgen) und verfügbar (neuestes
  GitHub-Release der App, Tag `<app>-v<version>`); Status *Aktuell*, *Update* oder *Nicht installiert*
- **Installieren und Aktualisieren** mit einem Klick: lädt den Installer des neuesten Releases, prüft ihn
  gegen die SHA-256-Prüfsumme von GitHub und führt ihn im passiven Modus aus (nur Fortschrittsbalken,
  eine laufende Instanz der App wird dabei beendet)
- **Starten** installierter Apps, Link zum Quellcode auf GitHub
- **Menü je App**: im Explorer anzeigen (Installationsordner, Programm markiert) und deinstallieren
  (still, nach Rückfrage; eine laufende Instanz wird vorher beendet, die Einstellungen der App bleiben)
- **Filter** (<kbd>Strg</kbd>+<kbd>F</kbd>), <kbd>F5</kbd> aktualisiert, dunkles/helles Design

## Technik

[Tauri 2](https://v2.tauri.app) (Rust) mit Vue 3, TypeScript, Tailwind CSS 4 und reka-ui – Aufbau und
Aussehen wie [mrstart](../mrstart).

```
src/                  Vue-Frontend
src-tauri/
  build.rs            Katalog der Apps aus apps/ einbetten
  src/catalog.rs      Katalog plus Installationsstatus
  src/installed.rs    Installierte Apps aus der Registry, Starten
  src/releases.rs     Releases (öffentliche GitHub-API, ohne Token), Installer laden, prüfen, ausführen
```

Neue Apps erscheinen in mrtools, sobald mrtools neu gebaut und veröffentlicht wurde.

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrtools`).
