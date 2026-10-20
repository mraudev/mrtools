mod altdaten;
mod artikel;
mod aufmass;
mod ausdruck;
mod belege;
mod briefe;
mod db;
mod einstellungen;
mod gaeb;
mod journal;
mod kalk;
mod kunden;
mod modelle;
mod pdf;
mod rechnen;
mod regie;
mod state;
mod zugferd;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            pdf::vorladen();
            let state = state::AppState::neu(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            state::db_info,
            state::sichern,
            state::db_wechseln,
            state::wiederherstellen,
            einstellungen::einstellungen_laden,
            einstellungen::einstellungen_speichern,
            einstellungen::bild_setzen,
            einstellungen::bild_laden,
            kunden::kunden_liste,
            kunden::kunde_laden,
            kunden::kunde_speichern,
            kunden::kunde_loeschen,
            kunden::plz_suche,
            kunden::projekt_speichern,
            kunden::projekt_loeschen,
            kunden::projekte_suchen,
            belege::beleg_laden,
            belege::beleg_speichern,
            belege::beleg_summen,
            belege::beleg_neu,
            belege::beleg_umwandeln,
            belege::beleg_loeschen,
            belege::beleg_festschreiben,
            belege::beleg_stornieren,
            belege::belege_projekt,
            belege::abzuege_vorschlagen,
            journal::rechnungsbuch,
            journal::zahlung_buchen,
            journal::zahlung_loeschen,
            journal::mahnung_erstellen,
            journal::mahnung_loeschen,
            journal::mahnvorschlag,
            journal::umsatz,
            journal::rechnungsbuch_csv,
            aufmass::aufmass_laden,
            aufmass::aufmass_speichern,
            aufmass::aufmass_rechnen,
            aufmass::formeln_liste,
            aufmass::formeln_speichern,
            aufmass::aufmass_positionen,
            regie::regie_liste,
            regie::regie_speichern,
            regie::regie_loeschen,
            regie::regie_abrechnen,
            artikel::artikel_liste,
            artikel::artikel_speichern,
            artikel::artikel_loeschen,
            artikel::lager_buchen,
            artikel::lager_bewegungen,
            artikel::bestellvorschlag,
            artikel::datanorm_import,
            artikel::artikel_etiketten,
            artikel::kunden_etiketten,
            kalk::mitarbeiter_liste,
            kalk::mitarbeiter_speichern,
            kalk::mitarbeiter_loeschen,
            kalk::kalkulation_laden,
            kalk::kalkulation_rechnen,
            kalk::kalkulation_speichern,
            kalk::kalkulation_artikel,
            kalk::zeiten_liste,
            kalk::zeit_speichern,
            kalk::zeit_loeschen,
            kalk::nachkalkulation,
            gaeb::gaeb_import,
            gaeb::gaeb_export,
            briefe::briefe_liste,
            briefe::brief_laden,
            briefe::brief_neu,
            briefe::brief_speichern,
            briefe::brief_loeschen,
            briefe::textbausteine,
            briefe::textbaustein_speichern,
            briefe::textbaustein_loeschen,
            briefe::suche,
            pdf::druck::pdf_vorschau,
            pdf::druck::pdf_speichern,
            pdf::druck::erechnung_xml,
            pdf::druck::datei_oeffnen,
            pdf::druck::ordner_zeigen,
            altdaten::befehl::altdaten_vorschlag,
            altdaten::befehl::altdaten_importieren,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app.state::<state::AppState>();
                let behalten = state.mit_db(|db| Ok(einstellungen::allgemein(db)?.sicherungen)).unwrap_or(30);
                let _ = state.sichern(behalten);
            }
        });
}
