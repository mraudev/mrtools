# mrtools

Kleine Windows-Tools – alle mit [Tauri 2](https://v2.tauri.app) (Rust), Vue 3, TypeScript, Tailwind CSS 4
und reka-ui, Aufbau und Aussehen wie [mrstart](apps/mrstart).

| App                                | Zweck                                     | Dev-Port |
| ---------------------------------- | ----------------------------------------- | -------- |
| [mrtools](apps/mrtools)            | Übersicht und Starter für die eigenen Apps | 1430     |
| [mrprocs](apps/mrprocs)            | Zeigt alle laufenden Prozesse             | 1440     |
| [mrdiskspace](apps/mrdiskspace)    | Zeigt, wo der Plattenplatz bleibt         | 1450     |
| [mrstart](apps/mrstart)            | Desktop-Launcher für Entwicklungsprojekte | 1420     |

Jede App hat ihre eigene Version (in `apps/<app>/package.json`) und eigene Releases.

## Aufbau

```
apps/<app>/           eine Tauri-App (Frontend in src/, Rust in src-tauri/)
packages/ui/          gemeinsame Oberfläche: Styles (theme.css), Tip, Dialog, Toaster,
                      Segmented, WindowControls, ThemeToggle, Design und Toasts
scripts/release.mjs   Version erhöhen und Release-Tag setzen
package.json          npm-Workspaces (ein node_modules für alles)
Cargo.toml            Cargo-Workspace (ein target/ für alle Apps)
```

In den Apps wird die gemeinsame Oberfläche so eingebunden:

```ts
import Tip from "@mrtools/ui/components/Tip";
import { toast } from "@mrtools/ui/lib/toast";
```

```css
/* apps/<app>/src/style.css */
@import "tailwindcss";
@import "@mrtools/ui/theme.css";
```

## Entwicklung

Voraussetzungen: Node.js 22, Rust (stable, MSVC), Visual Studio C++ Build Tools, WebView2.

```bash
npm install
npm run tauri dev -w apps/mrprocs      # eine App starten
npm run tauri build -w apps/mrprocs    # NSIS-Installer einer App (pro Benutzer, ohne Admin-Rechte)
```

Jede App hat einen eigenen Dev-Port, deshalb können mehrere gleichzeitig laufen.

Prüfungen wie in der CI (alle Apps):

```bash
npm run build && npm test
cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

## Neue App anlegen

Eine bestehende App nach `apps/<neu>` kopieren, dann Name, `identifier`, Logo (`src/assets/logo.svg`,
Icons mit `npx tauri icon src/assets/logo.svg -o src-tauri/icons`), Dev-Port (`vite.config.ts` und
`devUrl` in `tauri.conf.json`) und die Tabelle oben anpassen. Danach `npm install`.

## Release

```bash
npm run release -- mrprocs minor    # oder patch/major/x.y.z – erhöht die Version, committet, taggt mrprocs-v0.2.0
git push --follow-tags
```

Der Workflow `release.yml` baut nur die App aus dem Tag und veröffentlicht ihren Installer als
GitHub-Release `mrprocs 0.2.0`. Die Release-Notizen sind die Commits seit dem letzten Release dieser App,
die `apps/mrprocs` oder `packages/ui` betreffen. mrtools findet das Release über das Tag-Präfix.

Der Release-Job läuft im Environment `release` (*Settings → Environments*, nur Tags `*-v*` erlaubt).

**Auto-Update:** Bisher hat nur mrstart einen Auto-Updater. Weil sich alle Apps die „Latest“-Release des
Repositorys teilen, fragt mrstart eine feste Adresse ab: das Release `mrstart-latest`, in das der Workflow
nach jedem mrstart-Release die `latest.json` kopiert. Die Update-Pakete werden mit dem Schlüssel aus den
Secrets des Environments signiert – Einzelheiten in [apps/mrstart/README.md](apps/mrstart/README.md).
