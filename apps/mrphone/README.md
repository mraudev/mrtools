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
  phone.rs         mehrere SIP-Konten; eigener Thread mit einer Ereignisschleife
  sip/msg.rs       SIP-Nachrichten lesen/schreiben
  sip/ua.rs        Benutzeragent: Anmeldung, Digest, Übernahme-Erkennung, Besetztlampenfeld, Gespräche
                   (ein-/ausgehend, Halten, Weiterleiten mit/ohne Rückfrage, Tastentöne)
  sip/sdp.rs       Codec-Aushandlung (Opus, G.722, G.711, telephone-event)
  sip/rtp.rs       Sprachstrom: im Mikrofontakt senden, RFC-4733-Tastentöne, Jitter/Verlust, RTCP
  sip/g722.rs      G.722-Codec (bitgenau wie die Electron-Version); Opus über libopus (Crate opus)
  cti.rs           CTI-Server der Anlage (optional je Konto): Nicht stören, Abwesend, Kurzwahl-Status,
                   Konferenzteilnehmer; Wiederverbinden mit Wartezeiten und Begrenzung wie die Electron-Version
  backup.rs        verschlüsselte Sicherung (scrypt + AES-256-GCM), in beide Richtungen kompatibel zur
                   Electron-Version (testdata/electron-backup.mrphone stammt von deren backup.js)
  imports.rs       Kontakte: CSV-Import (Outlook-Exporte, UTF-8/ANSI), CSV-Export, klassisches Outlook per COM
  desktop.rs       Windows: Tray, Meldungen (Anruf mit Annehmen/Ablehnen, verpasster Anruf), Fenster
                   hervorholen, Abmelden bei gesperrtem PC, Fensterknöpfe der eigenen Titelleiste
  updater.rs       automatische Updates (signiert, Kanal mrphone-latest) – nur in der installierten App
  migration.rs     erkennt die installierte Electron-Version (Registry)
  logger.rs        Protokoll sipphone.log im Datenordner
src-tauri/installer-hooks.nsh   Installer entfernt die Electron-Version (Daten bleiben)
```

Die Oberfläche ist die der Electron-Version, nur die Brücke `window.phone` ist neu. Dazu kommt das Design
„mrtools“ (`public/mr.css`, Standard) im Aussehen der übrigen mr-Apps; das bisherige Design bleibt unter
Einstellungen → Allgemein → Design als „Klassisch“ wählbar. Logo: `assets/logo.svg` (wie die anderen Apps,
Icons mit `npx tauri icon assets/logo.svg -o src-tauri/icons`). Audio (Mikrofon, Geräteauswahl,
AudioWorklet) läuft weiter in der WebView (WebView2 = Chromium).

## Stand

| Stufe | Inhalt | Stand |
| --- | --- | --- |
| 0 | Machbarkeit: Audio in WebView2, binäre IPC im 20-ms-Takt, Electron-Passwörter, Windows-Meldungen mit Knöpfen, WebHID | erledigt |
| 1 | Brücke, Einstellungen, Konten, Kontakte, Verlauf, Kurzwahl, Klingelton | erledigt |
| 2 | SIP-Anmeldung (mehrere Konten, Übernahme, BLF) | erledigt |
| 3 | Gespräche (RTP, Opus/G.722/G.711, DTMF, Halten, Weiterleiten) | erledigt – Fenster in den Vordergrund und Windows-Meldung bei Anrufen folgen in Stufe 5 |
| 4 | CTI, Sicherung, Importe/Exporte (CSV, Outlook, PhonerLite) | erledigt |
| 5 | Tray, Meldungen, Bildschirmsperre, Headset, Titelleiste, Protokoll | erledigt |
| 6 | Signierte Updates, Umstieg aus der Electron-Version | umgesetzt – Testphase mit dem ersten Release |

## Daten und Umstieg aus der Electron-Version

Die Daten liegen wie in der Electron-Version in `%APPDATA%\SIP Phone` (config.json, contacts.json,
history.json, „Local State“ mit dem Schlüssel der Zugangsdaten). Der Installer dieser Version entfernt eine
installierte Electron-Version still (`installer-hooks.nsh`, über ihren festen Uninstall-Eintrag) – die Daten
bleiben und gelten direkt weiter, auch die verschlüsselten Passwörter. Startmenü-Verknüpfung und
Firewall-Freigabe legt der Installer bzw. Windows neu an (Windows fragt beim ersten Gespräch einmal nach).

Ist die Electron-Version noch installiert (z. B. Start der Exe aus dem Build-Ordner), arbeitet diese Version
auf einer **Kopie** in `%APPDATA%\mrphone-tauri-test` (beim ersten Start angelegt) und lässt die installierte
App unberührt; die Version zeigt dann „(Tauri-Test)“. `MRPHONE_DATA_DIR` setzt einen anderen Ordner
(Selbsttests).

Auf der Testkopie startet die Telefonie **vorsichtig**: Ist das Konto schon an einem anderen Gerät angemeldet
(z. B. der installierten Electron-App), zeigt mrphone „An anderem Gerät“ und meldet sich erst nach
*Übernehmen* an. Ist `MRPHONE_DATA_DIR` gesetzt (Selbsttests), zeigt mrphone keine Windows-Meldungen, holt das Fenster nicht
hervor und läuft auch neben einer anderen Instanz – Meldungen stehen dann nur im Protokoll. Für Selbsttests: `MRPHONE_CAUTIOUS=0/1`, `MRPHONE_SIP_BIND=127.0.0.1` (nur lokal lauschen,
keine Firewall-Abfrage), `SIP_TRACE=1` (SIP-Mitschnitt im Protokoll), `MRPHONE_MUTE=1` (Fenster stumm).

## Updates und Release

Wie mrstart und mrtools: Die installierte App prüft beim Start und alle 4 Stunden
`https://github.com/mraudev/mrtools/releases/download/mrphone-latest/latest.json`, lädt ein Update still
herunter und bietet dann „Neu starten“ an (nie mitten im Gespräch; vorher wird abgemeldet). Wer nicht neu
startet, bekommt das Update beim Beenden über das Tray. Die Pakete sind mit dem gemeinsamen Schlüssel der
mr-Apps signiert (Secrets im Environment `release`, siehe Root-README).

```bash
npm run release -- mrphone <patch|minor|major|x.y.z>   # im Repository-Root, dann: git push --follow-tags
```

Die Version muss über der letzten Electron-Version liegen (die steht in mraudev/sipphone), damit Nutzer
nicht verwirrt werden.

## Entwicklung

```bash
npm run tauri build -w apps/mrphone    # Installer nach target/release/bundle/nsis (braucht den Signaturschlüssel)
npm run tauri build -w apps/mrphone -- --config '{"bundle":{"createUpdaterArtifacts":false}}'   # ohne Schlüssel
cargo test -p mrphone
```

Opus kommt aus libopus und wird beim ersten Build per CMake übersetzt (`.cargo/config.toml` setzt dafür
`CMAKE_POLICY_VERSION_MINIMUM`). Mit Visual Studio 2026 und einem älteren CMake im PATH kennt CMake den
Generator noch nicht – dann `CMAKE` auf das CMake von Visual Studio setzen, z. B.
`C:\Program Files\Microsoft Visual Studio\18\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe`.
