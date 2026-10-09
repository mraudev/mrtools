# mrphone

SIP-Softphone für Asterisk-Telefonanlagen – hier die **Tauri-Version im Aufbau**. Die bisherige
Electron-Version (Windows und Linux) liegt in [mraudev/sipphone](https://github.com/mraudev/sipphone) und
bekommt dort weiter Updates, bis diese Version gleichwertig ist. Linux bleibt bei Electron.

## Aufbau

```
public/            Oberfläche (HTML/CSS/JS ohne Build-Schritt) – dieselbe wie in der Electron-Version
src-tauri/src/
  bridge.js        stellt window.phone bereit (wie preload.js der Electron-Version), über invoke/listen
  commands.rs      Befehle hinter window.phone
  config.rs        config.json (gleiches Format wie die Electron-Version)
  secrets.rs       Zugangsdaten verschlüsselt wie Electrons safeStorage (DPAPI + AES-256-GCM, „v10“)
  contacts.rs      Telefonbuch (contacts.json)
  history.rs       Gesprächsverlauf (history.json)
  paths.rs         Datenordner
```

Die Oberfläche bleibt unverändert; nur die Brücke `window.phone` ist neu. Audio (Mikrofon, Geräteauswahl,
AudioWorklet) läuft weiter in der WebView (WebView2 = Chromium).

## Stand

| Stufe | Inhalt | Stand |
| --- | --- | --- |
| 0 | Machbarkeit: Audio in WebView2, binäre IPC im 20-ms-Takt, Electron-Passwörter, Windows-Meldungen mit Knöpfen, WebHID | erledigt |
| 1 | Brücke, Einstellungen, Konten, Kontakte, Verlauf, Kurzwahl, Klingelton | erledigt |
| 2 | SIP-Anmeldung (mehrere Konten, Übernahme, BLF) | offen |
| 3 | Gespräche (RTP, Opus/G.722/G.711, DTMF, Halten, Weiterleiten) | offen |
| 4 | CTI, Sicherung, Importe/Exporte | offen |
| 5 | Tray, Meldungen, Bildschirmsperre, Headset, Titelleiste, Protokoll | offen |
| 6 | Signierte Updates, Umstieg aus der Electron-Version | offen |

## Daten

Die Electron-Version speichert in `%APPDATA%\SIP Phone`. Solange diese Version im Aufbau ist, arbeitet sie
auf einer **Kopie** in `%APPDATA%\mrphone-tauri-test` (beim ersten Start angelegt) – die installierte
Electron-App bleibt unberührt. `MRPHONE_DATA_DIR` setzt einen anderen Ordner (Selbsttests).

## Entwicklung

```bash
npm run tauri build -w apps/mrphone    # Installer nach target/release/bundle/nsis
cargo test -p mrphone
```
