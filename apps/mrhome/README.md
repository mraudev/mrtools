# mrhome

Licht und Heizung steuern: Philips Hue und Govee **lokal** im Heimnetz, tado° X über die tado-Cloud.

## Licht (lokal, ohne Cloud)

- **Philips Hue** über die lokale API der Hue Bridge: Bridge wird im Netz gefunden (mDNS, wie die Hue-App) oder
  per IP eingetragen; zum Koppeln einmal den Knopf auf der Bridge drücken. Der Schlüssel liegt verschlüsselt in der
  Windows-Anmeldeinformationsverwaltung, das Zertifikat der Bridge wird beim Koppeln festgehalten – danach spricht
  mrhome nur noch mit genau dieser Bridge
- Räume und Zonen wie in der Hue-App: alle Lampen eines Raums schalten und dimmen, **Szenen** per Klick,
  jede Lampe einzeln mit Helligkeit, Farbe und Weißton; nicht erreichbare Lampen sind markiert
- **Live**: Änderungen aus der Hue-App, per Schalter oder Sprachassistent erscheinen sofort (Event-Stream der Bridge)
- **Govee** über Govees LAN-API: Geräte werden im Netz gesucht und direkt angesprochen – an/aus, Helligkeit,
  Farbe, Weißton. Govee liefert dabei keine Namen; per Doppelklick lassen sich die Geräte benennen.
  Voraussetzung: „LAN-Steuerung“ ist in der Govee-Home-App beim Gerät eingeschaltet (nur neuere Modelle), und
  Windows darf mrhome im privaten Netzwerk kommunizieren lassen

## Heizung (tado° X, Cloud)

- **Anmeldung** über tados Anmeldeseite im Browser (Code bestätigen); mrhome sieht das Passwort nie.
  Das Refresh-Token liegt verschlüsselt in der Windows-Anmeldeinformationsverwaltung
- **Räume** als Karten: Ist-Temperatur, Luftfeuchte, Soll-Temperatur, Heizleistung, Modus (Zeitplan,
  manuell bis …, Boost, aus), nächste Planänderung, „Fenster offen“ und nicht erreichbare Thermostate
- **Steuern**: Soll-Temperatur in 0,5°-Schritten (5–25 °C), Heizung aus/an, zurück zum Zeitplan,
  Fenster-offen-Modus beenden. Ob eine Änderung bis zur nächsten Planänderung, für 1/2/3 Stunden oder
  dauerhaft gilt, wird in den Einstellungen gewählt
- **Ganzes Zuhause**: Zuhause / Abwesend / Automatisch (Geofencing), Boost überall (30 min), alles aus,
  überall zurück zum Zeitplan

### Tageslimit von tado

Ohne Auto-Assist-Abo erlaubt tado **100 Zugriffe pro Tag** (mit Abo 20.000), zurückgesetzt um 12:00 Uhr.
mrhome geht deshalb sparsam damit um:

- tado wird erst geladen, wenn der Bereich „Heizung“ geöffnet wird
- Die Statusleiste zeigt den echten Restbestand („Heute noch 87 von 100 Zugriffen“) aus tados Antworten
- Automatisch aktualisiert wird nur, solange die Heizung sichtbar ist – einstellbar (nie, 15 min bis 2 h,
  Standard 30 min); unter 15 übrigen Zugriffen gar nicht mehr, damit für Änderungen genug bleibt
- Mehrere Klicks auf +/− werden zu **einem** Zugriff zusammengefasst; Änderungen erscheinen sofort, ohne
  den Raum danach neu zu laden

Die tado-X-API ist nicht offiziell dokumentiert (Endpunkte nach der Community, u. a. PyTado); tado kann sie
jederzeit ändern. Lokal ließe sich tado X nur über Matter/Thread steuern (z. B. mit Home Assistant).

## Allgemein

- Dunkles/helles Design, **automatische Updates**

## Technik

[Tauri 2](https://v2.tauri.app) (Rust: `reqwest`/`rustls` mit Zertifikats-Pinning für die Hue Bridge, `mdns-sd`,
UDP für Govee, Windows-Anmeldeinformationsverwaltung über `keyring-core`) mit Vue 3, TypeScript, Tailwind CSS 4
und reka-ui – Aufbau und Aussehen wie [mrstart](../mrstart).

Netzwerk-Tests gegen echte Geräte (nur lesend, nicht in der CI):

```bash
cargo test -p mrhome --lib network -- --ignored --nocapture --test-threads=1
```

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrhome`).
