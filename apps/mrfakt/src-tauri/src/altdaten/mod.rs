//! Übernahme der Daten aus GHFakt.
//!
//! Die Access-Dateien (Jet 3/4) liest ein PowerShell-Skript über den 32-bit-Provider
//! `Microsoft.Jet.OLEDB.4.0` und schreibt sie als JSON; die Textdateien der Belege liest Rust direkt.
//! Der Altbestand wird nur gelesen. Ein erneuter Import ersetzt alle Daten in mrfakt.

pub mod befehl;
pub mod einstell;
pub mod rtf;
pub mod texte;

use crate::belege::arten::{art_aus_slot, art_aus_titel};
use crate::db::{self, err, Datensatz, Res};
use crate::modelle::*;
use crate::rechnen;
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use texte::{dekodieren, zahl};

const JET_SKRIPT: &str = include_str!("jet_export.ps1");

#[derive(Debug, Default, Serialize, Clone)]
pub struct Bericht {
    pub anzahl: BTreeMap<String, i64>,
    pub hinweise: Vec<String>,
    /// Belege, deren neu berechneter Betrag vom Rechnungsbuch abweicht.
    pub abweichungen: Vec<Abweichung>,
    pub abweichungen_gesamt: i64,
    pub dauer_s: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct Abweichung {
    pub beleg: String,
    pub kunde: String,
    pub datum: String,
    pub berechnet: Decimal,
    pub rechnungsbuch: Decimal,
    pub waehrung: String,
}

impl Bericht {
    fn zaehlen(&mut self, was: &str) {
        *self.anzahl.entry(was.to_string()).or_default() += 1;
    }
    fn hinweis(&mut self, text: String) {
        if self.hinweise.len() < 300 {
            self.hinweise.push(text);
        }
    }
}

pub trait Fortschritt {
    fn melden(&self, text: &str, anteil: f64);
}

impl<F: Fn(&str, f64)> Fortschritt for F {
    fn melden(&self, text: &str, anteil: f64) {
        self(text, anteil)
    }
}

fn s(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(t)) => t.trim().to_string(),
        Some(Value::Bool(b)) => if *b { "1".into() } else { "0".into() },
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn dez(v: &Value, k: &str) -> Decimal {
    let t = s(v, k);
    if t.contains(',') {
        zahl(&t)
    } else {
        Decimal::from_str(&t).unwrap_or_default()
    }
}

fn ganz(v: &Value, k: &str) -> i64 {
    s(v, k).parse::<f64>().map(|f| f as i64).unwrap_or(0)
}

fn iso(v: &Value, k: &str) -> String {
    s(v, k).get(0..10).unwrap_or("").to_string()
}

/// Prüft, ob der Ordner ein GHFakt-Verzeichnis ist.
pub fn pruefen(ordner: &Path) -> Res<()> {
    let kunden = ordner.join("KUNDEN");
    if !kunden.is_dir() || !ordner.join("PROJEKTE").is_dir() {
        return Err(format!("{} enthält keine GHFakt-Daten (KUNDEN und PROJEKTE fehlen).", ordner.display()));
    }
    Ok(())
}

/// Erste vorhandene Datei aus der Liste (Groß-/Kleinschreibung egal, Windows).
fn erste(ordner: &Path, namen: &[&str]) -> Option<PathBuf> {
    namen.iter().map(|n| ordner.join(n)).find(|p| p.is_file())
}

/// Exportiert die Jet-Dateien als JSON in `ziel` und gibt die Pfade der JSON-Dateien zurück (gleiche Reihenfolge).
fn jet_export(quellen: &[PathBuf], ziel: &Path) -> Res<Vec<PathBuf>> {
    std::fs::create_dir_all(ziel).map_err(err)?;
    let skript = ziel.join("jet_export.ps1");
    // Mit BOM, damit Windows PowerShell 5.1 die Umlaute im Skript richtig liest.
    std::fs::write(&skript, format!("\u{feff}{JET_SKRIPT}")).map_err(err)?;
    let mut jobs = String::new();
    let mut ausgaben = vec![];
    for (i, q) in quellen.iter().enumerate() {
        let out = ziel.join(format!("{i}.json"));
        jobs.push_str(&format!("{}|{}\n", q.display(), out.display()));
        ausgaben.push(out);
    }
    let jobdatei = ziel.join("jobs.txt");
    std::fs::write(&jobdatei, jobs).map_err(err)?;
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    let ps32 = Path::new(&windir).join(r"SysWOW64\WindowsPowerShell\v1.0\powershell.exe");
    let ps = if ps32.is_file() { ps32 } else { PathBuf::from("powershell.exe") };
    let mut cmd = std::process::Command::new(ps);
    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]).arg(&skript).arg("-Jobs").arg(&jobdatei);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().map_err(|e| format!("PowerShell konnte nicht gestartet werden: {e}"))?;
    if !out.status.success() {
        return Err(format!("Export der Access-Dateien fehlgeschlagen: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(ausgaben)
}

fn json_lesen(pfad: &Path) -> Res<Value> {
    if let Ok(fehler) = std::fs::read_to_string(pfad.with_extension("json.error")) {
        return Err(fehler);
    }
    let text = std::fs::read_to_string(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", pfad.display()))
}

fn tabelle(v: &Value, name: &str) -> Vec<Value> {
    v.get(name).and_then(|t| t.as_array()).cloned().unwrap_or_default()
}

/// Dateien eines Kundenordners: Auftragsnummer → Dateiart (VP, LV, …) → Slot (Endung) → Pfad.
type Dateien = BTreeMap<String, BTreeMap<String, BTreeMap<String, PathBuf>>>;

fn dateien(ordner: &Path) -> Dateien {
    let mut out: Dateien = BTreeMap::new();
    let Ok(rd) = std::fs::read_dir(ordner) else { return out };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_uppercase();
        let ziffern: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
        if ziffern.is_empty() {
            continue;
        }
        let rest = &name[ziffern.len()..];
        let Some((art, ext)) = rest.split_once('.') else { continue };
        if art.ends_with("_SIK") {
            continue;
        }
        out.entry(ziffern).or_default().entry(art.to_string()).or_default().insert(ext.to_string(), e.path());
    }
    out
}

fn lesen(p: &Path) -> String {
    std::fs::read(p).map(|b| dekodieren(&b)).unwrap_or_default()
}

/// Ordnername `G0402150.706` → Kunden-ID `0402150706`.
fn kunden_id(ordner: &str) -> Option<String> {
    let rest = ordner.strip_prefix('G').or_else(|| ordner.strip_prefix('g'))?;
    let id: String = rest.chars().filter(|c| *c != '.').collect();
    (id.len() >= 8 && id.chars().all(|c| c.is_ascii_digit())).then_some(id)
}

const TABELLEN: &[&str] = &[
    "kalkulation", "zeit", "regie_zeile", "regie", "aufmass", "zahlung", "mahnung", "abschlag_abzug", "zuschlag", "lager_bewegung",
    "position", "beleg", "brief", "projekt", "artikel", "kunde", "plz", "formel", "textbaustein",
];

pub fn importieren(db: &rusqlite::Connection, ordner: &Path, arbeitsordner: &Path, fortschritt: &dyn Fortschritt) -> Res<Bericht> {
    let start = std::time::Instant::now();
    pruefen(ordner)?;
    let mut bericht = Bericht::default();
    let kunden_dir = ordner.join("KUNDEN");
    let artikel_dir = ordner.join("ARTIKEL");
    let projekte_dir = ordner.join("PROJEKTE");

    // 1. Access-Dateien exportieren
    fortschritt.melden("Access-Datenbanken lesen …", 0.02);
    let kunden_db = erste(&kunden_dir, &["Kunden02.dat", "Kunden01.mdb"]).ok_or("Kundendatei nicht gefunden")?;
    let buch_db = erste(&kunden_dir, &["Buch_02.dat", "Buch_01.mdb"]);
    let artikel_db = erste(&artikel_dir, &["Artikl03.Dat", "Artikl02.mdb"]);
    let plz_db = erste(&kunden_dir, &["PLZ_NEU.mdb"]);
    let mut quellen = vec![kunden_db];
    let mut idx_buch = None;
    let mut idx_artikel = None;
    let mut idx_plz = None;
    for (opt, idx) in [(&buch_db, &mut idx_buch), (&artikel_db, &mut idx_artikel), (&plz_db, &mut idx_plz)] {
        if let Some(p) = opt {
            *idx = Some(quellen.len());
            quellen.push(p.clone());
        }
    }
    // Regieberichte (NNNNNRe.MDB) in den Kundenordnern
    let mut kundenordner: Vec<(String, PathBuf)> = vec![];
    for e in std::fs::read_dir(&projekte_dir).map_err(err)?.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if e.path().is_dir() {
            if let Some(id) = kunden_id(&name) {
                kundenordner.push((id, e.path()));
            }
        }
    }
    kundenordner.sort();
    let mut regie_quellen: Vec<(String, String, usize)> = vec![]; // (kunden_id, auftrag, index)
    for (id, pfad) in &kundenordner {
        for (auftrag, arten) in dateien(pfad) {
            if let Some(p) = arten.get("RE").and_then(|m| m.get("MDB")) {
                regie_quellen.push((id.clone(), auftrag.clone(), quellen.len()));
                quellen.push(p.clone());
            }
        }
    }
    let json = jet_export(&quellen, &arbeitsordner.join("jet"))?;

    fortschritt.melden("Vorhandene Daten leeren …", 0.25);
    let tx = db.unchecked_transaction().map_err(err)?;
    for t in TABELLEN {
        tx.execute(&format!("DELETE FROM {t}"), []).map_err(err)?;
    }

    // 2. Einstellungen
    fortschritt.melden("Einstellungen übernehmen …", 0.27);
    let firma = einstell::firma(ordner);
    db::einstellung_setzen(&tx, "firma", &firma)?;
    db::einstellung_setzen(&tx, "allgemein", &einstell::allgemein(ordner))?;
    for (name, datei) in [("logo", "logo.jpg"), ("fusszeile", "fusszeile.jpg")] {
        if let Ok(daten) = std::fs::read(ordner.join(datei)) {
            tx.execute(
                "INSERT INTO datei (name, mime, daten) VALUES (?1, 'image/jpeg', ?2) ON CONFLICT(name) DO UPDATE SET mime = excluded.mime, daten = excluded.daten",
                rusqlite::params![name, daten],
            )
            .map_err(err)?;
        }
    }
    for mut t in einstell::textbausteine(ordner) {
        t.speichern(&tx)?;
        bericht.zaehlen("Textbausteine");
    }
    for mut f in einstell::formeln(ordner) {
        f.speichern(&tx)?;
        bericht.zaehlen("Aufmaß-Formeln");
    }

    // 3. PLZ
    if let Some(i) = idx_plz {
        fortschritt.melden("Postleitzahlen …", 0.3);
        match json_lesen(&json[i]) {
            Ok(v) => {
                for r in tabelle(&v, "Plz") {
                    let mut p = Plz { plz: s(&r, "Plz"), ort: s(&r, "Ort"), zusatz: s(&r, "Zusatz"), land: s(&r, "Land"), ..Default::default() };
                    if !p.plz.is_empty() {
                        p.speichern(&tx)?;
                        bericht.zaehlen("Postleitzahlen");
                    }
                }
            }
            Err(e) => bericht.hinweis(format!("PLZ: {e}")),
        }
    }

    // 4. Kunden
    fortschritt.melden("Kunden …", 0.33);
    let v = json_lesen(&json[0])?;
    let mut kunden: HashMap<String, i64> = HashMap::new();
    let mut kunden_namen: HashMap<i64, String> = HashMap::new();
    let jetzt = db::jetzt();
    for r in tabelle(&v, "Kunden") {
        let mut k = Kunde {
            alt_id: s(&r, "ID"),
            nummer: s(&r, "KdNummer"),
            typ: Some(s(&r, "Typ")).filter(|t| !t.is_empty()).unwrap_or("Kunde".into()),
            suchname: s(&r, "Suchname"),
            anrede: s(&r, "Name1"),
            name1: s(&r, "Name2"),
            name2: s(&r, "Name3"),
            ortsteil: s(&r, "Ortsteil"),
            strasse: s(&r, "Strasse"),
            plz: s(&r, "PLZ"),
            ort: s(&r, "Ort"),
            land: s(&r, "LandRegion"),
            telefon: s(&r, "Telefon"),
            telefon_privat: s(&r, "TelefonPrivat"),
            telefax: s(&r, "Telefax"),
            mobil: s(&r, "Mobiltelefon"),
            mobil_privat: s(&r, "MobiltelefonPrivat"),
            email: s(&r, "EMail"),
            email_privat: s(&r, "EMailPrivat"),
            internet: s(&r, "Internet"),
            ansprechpartner: s(&r, "Ansprech"),
            position: s(&r, "PositionInFirma"),
            geschlecht: Some(s(&r, "Geschlecht")).filter(|g| !g.is_empty()).unwrap_or(s(&r, "Anrede")),
            geburtstag: iso(&r, "Geburtstag"),
            bank: s(&r, "Bank"),
            blz: s(&r, "Blz"),
            konto: s(&r, "Konto"),
            ustid: s(&r, "USTID"),
            unsere_kdnr: s(&r, "KdNummerBLief"),
            ihre_kdnr: s(&r, "EigKontoNrBUber"),
            gruppe: s(&r, "Gruppe"),
            skonto: s(&r, "Skonto"),
            rabatt: s(&r, "Rabatt"),
            rechnungsart: s(&r, "RechnungsArt"),
            ust_art: s(&r, "UStArt"),
            sonstiges1: s(&r, "Sonstiges1"),
            sonstiges2: s(&r, "Sonstiges2"),
            sonstiges3: s(&r, "Sonstiges3"),
            sonstiges4: s(&r, "Sonstiges4"),
            sonstiges5: s(&r, "Sonstiges5"),
            bemerkung1: s(&r, "Bemerkung1"),
            bemerkung2: s(&r, "Bemerkung2"),
            notiz: [s(&r, "Memofeld1"), s(&r, "ErsatzMemo")].into_iter().filter(|t| !t.is_empty()).collect::<Vec<_>>().join("\n"),
            angelegt: jetzt.clone(),
            geaendert: Some(s(&r, "LetzteAenderung").replace('T', " ")).filter(|t| !t.is_empty()).unwrap_or(jetzt.clone()),
            zugriff: s(&r, "LetzterZugriff").replace('T', " "),
            ..Default::default()
        };
        if [&k.suchname, &k.name1, &k.name2, &k.strasse, &k.ort, &k.telefon].iter().all(|t| t.trim().is_empty()) {
            // In GHFakt gelöschte Kunden bleiben als leere Sätze stehen.
            bericht.zaehlen("Leere Kundensätze übersprungen");
            continue;
        }
        if k.zahlungsbedingung.starts_with("Standard") {
            k.zahlungsbedingung.clear();
        }
        let id = k.speichern(&tx)?;
        kunden_namen.insert(id, k.anzeigename());
        kunden.insert(k.alt_id.clone(), id);
        bericht.zaehlen("Kunden");
    }

    // 5. Artikel
    if let Some(i) = idx_artikel {
        fortschritt.melden("Artikel …", 0.36);
        match json_lesen(&json[i]) {
            Ok(v) => {
                for r in tabelle(&v, "Daten") {
                    let lang = s(&r, "Langtext").lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n");
                    let text = [s(&r, "Kurztext"), s(&r, "Text2"), s(&r, "Text3"), lang.trim().to_string()]
                        .into_iter()
                        .filter(|t| !t.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                    let mut a = Artikel {
                        alt_id: s(&r, "ID"),
                        nummer: s(&r, "PosNr"),
                        suchtext: s(&r, "Suchtext"),
                        text,
                        gewerk: s(&r, "Gewerk"),
                        einheit: s(&r, "Einheit"),
                        ek: dez(&r, "Sk_A"),
                        vk1: dez(&r, "Vk1_A"),
                        vk2: dez(&r, "Vk2_A"),
                        vk3: dez(&r, "Vk3_A"),
                        rabattgruppe: s(&r, "RabattGruppe"),
                        rabatt: dez(&r, "Rabatt1"),
                        lieferant: s(&r, "Lieferant"),
                        lief_nr: s(&r, "LiefNr"),
                        gewicht: s(&r, "Gewicht"),
                        abmessung: s(&r, "Abmessung"),
                        warengruppe: s(&r, "Typ"),
                        preiseinheit: Decimal::ONE,
                        lager_aktiv: s(&r, "LagerAktiv") == "1",
                        bestand: dez(&r, "LagerBestand"),
                        mindestbestand: dez(&r, "LagerMinimal"),
                        bestellmenge: dez(&r, "LagerBestMenge"),
                        lagerplatz: s(&r, "LagerPlatz"),
                        datum_preis: iso(&r, "DatumPreis"),
                        notiz: [s(&r, "Memofeld"), s(&r, "LagerBemerk").trim_matches('-').to_string()].into_iter().filter(|t| !t.is_empty()).collect::<Vec<_>>().join("\n"),
                        quelle: "ghfakt".into(),
                        ..Default::default()
                    };
                    a.speichern(&tx)?;
                    bericht.zaehlen("Artikel");
                }
            }
            Err(e) => bericht.hinweis(format!("Artikel: {e}")),
        }
    }

    // 6. Rechnungsbuch einlesen: (Auftrag, Slot) → Eintrag
    let mut journal: HashMap<(String, i64), Value> = HashMap::new();
    if let Some(i) = idx_buch {
        match json_lesen(&json[i]) {
            Ok(v) => {
                for r in tabelle(&v, "Buchungen") {
                    journal.insert((s(&r, "AuftrNummer"), ganz(&r, "LVTyp")), r);
                }
            }
            Err(e) => bericht.hinweis(format!("Rechnungsbuch: {e}")),
        }
    }
    bericht.anzahl.insert("Rechnungsbuch-Einträge (GHFakt)".into(), journal.len() as i64);

    // 7. Projekte und Belege je Kundenordner
    let mut belege_nach_slot: HashMap<(String, String), i64> = HashMap::new(); // (Auftrag, Slot) → beleg_id
    let mut projekte: HashMap<String, (i64, i64)> = HashMap::new(); // Auftrag → (projekt_id, kunde_id)
    let anzahl_ordner = kundenordner.len().max(1);
    for (n, (alt_id, pfad)) in kundenordner.iter().enumerate() {
        if n % 25 == 0 {
            fortschritt.melden(&format!("Belege … ({}/{})", n, anzahl_ordner), 0.4 + 0.45 * n as f64 / anzahl_ordner as f64);
        }
        let ordner_dateien = dateien(pfad);
        let hat_inhalt = ordner_dateien.iter().any(|(a, arten)| a != "00000" && (arten.contains_key("VP") || arten.contains_key("RE")))
            || ordner_dateien.get("00000").is_some_and(|a| a.contains_key("GHT"));
        let kunde_id = match kunden.get(alt_id) {
            Some(id) => *id,
            None if !hat_inhalt => continue,
            None => {
                // Ordner ohne Kundensatz: Kunde aus dem neuesten Vorspann anlegen.
                let mut k = Kunde { alt_id: alt_id.clone(), typ: "Kunde".into(), angelegt: jetzt.clone(), geaendert: jetzt.clone(), ..Default::default() };
                let vp = dateien(pfad).values().rev().find_map(|a| a.get("VP").and_then(|m| m.values().next().cloned()));
                if let Some(vp) = vp {
                    let t = lesen(&vp);
                    let l: Vec<&str> = t.split('\n').map(|x| x.trim()).collect();
                    let g = |i: usize| l.get(i).map(|x| x.to_string()).unwrap_or_default();
                    (k.anrede, k.name1, k.name2, k.ortsteil, k.strasse, k.plz, k.ort) = (g(0), g(1), g(2), g(3), g(4), g(5), g(6));
                    k.suchname = k.name1.split_whitespace().last().unwrap_or("").to_string();
                }
                if k.suchname.is_empty() {
                    k.suchname = format!("(ohne Kundensatz {alt_id})");
                }
                k.notiz = "In GHFakt ohne Kundensatz – Anschrift aus dem letzten Beleg übernommen".into();
                let id = k.speichern(&tx)?;
                kunden.insert(alt_id.clone(), id);
                kunden_namen.insert(id, k.suchname.clone());
                bericht.hinweis(format!("Ordner G{alt_id}: kein Kundensatz, Kunde angelegt"));
                bericht.zaehlen("Kunden (aus Projektordnern ergänzt)");
                id
            }
        };
        let projektliste: HashMap<String, texte::AltProjekt> = std::fs::read(pfad.join("AUFTRAGI.DAT"))
            .map(|b| texte::projekte(&dekodieren(&b)).into_iter().map(|p| (p.nummer.clone(), p)).collect())
            .unwrap_or_default();
        let dateien = ordner_dateien;
        let mut auftraege: Vec<&String> = dateien.keys().filter(|a| a.as_str() != "00000").collect();
        for a in projektliste.keys() {
            if !dateien.contains_key(a) {
                auftraege.push(a);
            }
        }
        auftraege.sort();
        auftraege.dedup();
        for auftrag in auftraege {
            let arten = dateien.get(auftrag);
            let vp_slots: Vec<&String> = arten.and_then(|a| a.get("VP")).map(|m| m.keys().collect()).unwrap_or_default();
            let regie_da = arten.is_some_and(|a| a.contains_key("RE"));
            let alt = projektliste.get(auftrag);
            if vp_slots.is_empty() && !regie_da {
                // Projekte ohne Belege legt notfalls das Rechnungsbuch an (Schritt 8).
                continue;
            }
            let projekt_id = match projekte.get(auftrag) {
                Some((id, _)) => *id,
                None => {
                    let journal_text = journal.iter().find(|((a, _), _)| a == auftrag).map(|(_, r)| s(r, "AuftrText"));
                    let mut p = Projekt {
                        kunde_id,
                        nummer: auftrag.clone(),
                        bezeichnung: alt.map(|p| p.bezeichnung.clone()).or(journal_text).unwrap_or_default(),
                        datum: alt.map(|p| p.datum.clone()).unwrap_or_default(),
                        status: "abgeschlossen".into(),
                        ..Default::default()
                    };
                    let id = p.speichern(&tx)?;
                    projekte.insert(auftrag.clone(), (id, kunde_id));
                    bericht.zaehlen("Projekte");
                    id
                }
            };
            let Some(arten) = arten else { continue };
            for slot in vp_slots {
                let Ok(slot_nr) = slot.parse::<i64>() else { continue };
                let datei = |art: &str| arten.get(art).and_then(|m| m.get(slot));
                let vp = texte::vorspann(&lesen(datei("VP").unwrap()));
                let lv = datei("LV").map(|p| texte::lv(&lesen(p))).unwrap_or_default();
                let npn = datei("NPN").or(datei("NPX")).map(|p| texte::nachspann(&lesen(p)));
                let np_text = datei("NP").map(|p| texte::nachspann_alt(&lesen(p))).unwrap_or_default();
                let eintrag = journal.remove(&(auftrag.clone(), slot_nr));
                let beleg_id = beleg_anlegen(&tx, &mut bericht, &kunden_namen, BelegQuelle {
                    projekt_id, kunde_id, auftrag, slot, slot_nr, vp: Some(vp), lv, npn, np_text,
                    abschlaege: datei("EZ").or(datei("AZ")).map(|p| texte::abschlaege(&lesen(p))).unwrap_or_default(),
                    eintrag,
                })?;
                belege_nach_slot.insert((auftrag.clone(), slot.clone()), beleg_id);
                if let Some(au) = datei("AU") {
                    for (pos, a) in texte::aufmass(&lesen(au)).into_iter().enumerate() {
                        let mut z = Aufmass {
                            beleg_id, pos: pos as i64, posnr: a.posnr, art: a.art, text: a.text, anzahl: a.anzahl, formel: a.formel,
                            m1: a.m1, m2: a.m2, m3: a.m3, ergebnis: a.ergebnis, abzug: a.ergebnis < Decimal::ZERO, ..Default::default()
                        };
                        z.speichern(&tx)?;
                        bericht.zaehlen("Aufmaßzeilen");
                    }
                }
            }
        }
        // Briefe (00000GHT.0xx)
        if let Some(ght) = dateien.get("00000").and_then(|a| a.get("GHT")) {
            for p in ght.values() {
                let b = texte::brief(&lesen(p));
                let mut brief = Brief {
                    kunde_id, datum: b.datum.clone(), adresse: b.adresse.join("\n"), betreff: b.titel, text: b.text, angelegt: jetzt.clone(), ..Default::default()
                };
                brief.speichern(&tx)?;
                bericht.zaehlen("Briefe");
            }
        }
    }

    // 8. Rechnungsbuch-Einträge ohne Belegdatei
    fortschritt.melden("Rechnungsbuch ergänzen …", 0.86);
    let mut rest: Vec<((String, i64), Value)> = journal.into_iter().collect();
    rest.sort_by(|a, b| a.0.cmp(&b.0));
    for ((auftrag, slot_nr), r) in rest {
        let kunde_id = match kunden.get(&s(&r, "KundenID")).or_else(|| projekte.get(&auftrag).map(|(_, k)| k)) {
            Some(&id) => id,
            None => {
                // Kein Kundensatz mehr: aus der Anschrift im Rechnungsbuch anlegen (gleiche Anschrift = gleicher Kunde).
                let schluessel = format!("rb:{}|{}|{}", s(&r, "Name2"), s(&r, "Strasse"), s(&r, "Ort"));
                match kunden.get(&schluessel) {
                    Some(&id) => id,
                    None => {
                        let mut k = Kunde {
                            nummer: s(&r, "KdNummerEigen"), typ: "Kunde".into(), suchname: s(&r, "KdSuche"), anrede: s(&r, "Name1"),
                            name1: s(&r, "Name2"), name2: s(&r, "Name3"), strasse: s(&r, "Strasse"), ortsteil: s(&r, "Ortsteil"),
                            plz: s(&r, "Plz"), ort: s(&r, "Ort"), notiz: "Aus dem Rechnungsbuch von GHFakt übernommen".into(),
                            angelegt: jetzt.clone(), geaendert: jetzt.clone(), ..Default::default()
                        };
                        let id = k.speichern(&tx)?;
                        kunden_namen.insert(id, k.anzeigename());
                        kunden.insert(schluessel, id);
                        bericht.zaehlen("Kunden (aus dem Rechnungsbuch ergänzt)");
                        id
                    }
                }
            }
        };
        let projekt_id = match projekte.get(&auftrag) {
            Some((id, _)) => *id,
            None => {
                let mut p = Projekt { kunde_id, nummer: auftrag.clone(), bezeichnung: s(&r, "AuftrText"), datum: texte::datum(&s(&r, "AuftrDatum")), status: "abgeschlossen".into(), ..Default::default() };
                let id = p.speichern(&tx)?;
                projekte.insert(auftrag.clone(), (id, kunde_id));
                bericht.zaehlen("Projekte");
                id
            }
        };
        let slot = format!("{slot_nr:03}");
        let id = beleg_anlegen(&tx, &mut bericht, &kunden_namen, BelegQuelle {
            projekt_id, kunde_id, auftrag: &auftrag, slot: &slot, slot_nr, vp: None, lv: Default::default(), npn: None, np_text: String::new(), abschlaege: vec![], eintrag: Some(r),
        })?;
        belege_nach_slot.insert((auftrag.clone(), slot), id);
        bericht.zaehlen("Belege nur aus dem Rechnungsbuch");
    }

    // 9. Regieberichte
    fortschritt.melden("Regieberichte …", 0.9);
    for (_, auftrag, i) in &regie_quellen {
        let Some(&(projekt_id, _)) = projekte.get(auftrag) else { continue };
        let v = match json_lesen(&json[*i]) {
            Ok(v) => v,
            Err(e) => {
                bericht.hinweis(format!("Regie {auftrag}: {e}"));
                continue;
            }
        };
        let mut zeilen = tabelle(&v, "Rapporte");
        zeilen.sort_by_key(|z| (dez(z, "Seite"), s(z, "Zeile_asc")));
        let mut seiten: BTreeMap<Decimal, Vec<Value>> = BTreeMap::new();
        for z in zeilen {
            seiten.entry(dez(&z, "Seite")).or_default().push(z);
        }
        for (_, zeilen) in seiten {
            let kopf = zeilen.iter().find(|z| s(z, "Zeile_asc") == "20000");
            let slot = zeilen.first().map(|z| s(z, "LvNr")).unwrap_or_default();
            let mut regie = Regie {
                projekt_id,
                datum: kopf.map(|k| texte::datum(&s(k, "Kommentar"))).unwrap_or_default(),
                titel: kopf.map(|k| s(k, "Menge")).unwrap_or_default(),
                beleg_id: belege_nach_slot.get(&(auftrag.clone(), slot)).copied(),
                ..Default::default()
            };
            let regie_id = regie.speichern(&tx)?;
            bericht.zaehlen("Regieberichte");
            let mut pos = 0;
            for z in &zeilen {
                let nr = s(z, "Zeile_asc");
                let text = s(z, "Kommentar");
                if nr == "20000" {
                    continue;
                }
                let (art, posnr, menge) = if s(z, "ZA") == "N" {
                    ("P", s(z, "PosNr"), dez(z, "Menge"))
                } else {
                    ("T", String::new(), Decimal::ZERO)
                };
                if art == "T" && text.is_empty() {
                    continue;
                }
                let mut rz = RegieZeile { regie_id, pos, art: art.into(), posnr, text, menge, ..Default::default() };
                rz.speichern(&tx)?;
                pos += 1;
            }
        }
    }

    // 10. Nummernkreise
    for (kreis, n) in einstell::nummernkreise(ordner) {
        tx.execute(
            "INSERT INTO nummernkreis (name, naechste, format) VALUES (?1, ?2, '{n}') ON CONFLICT(name) DO UPDATE SET naechste = MAX(naechste, excluded.naechste)",
            rusqlite::params![kreis, n],
        )
        .map_err(err)?;
    }
    let max_projekt: i64 = tx.query_row("SELECT COALESCE(MAX(CAST(nummer AS INTEGER)), 0) FROM projekt", [], |r| r.get(0)).map_err(err)?;
    tx.execute(
        "INSERT INTO nummernkreis (name, naechste) VALUES ('projekt', ?1) ON CONFLICT(name) DO UPDATE SET naechste = MAX(naechste, excluded.naechste)",
        [max_projekt + 1],
    )
    .map_err(err)?;

    fortschritt.melden("Speichern …", 0.97);
    tx.commit().map_err(err)?;
    bericht.abweichungen.sort_by(|a, b| b.datum.cmp(&a.datum));
    bericht.abweichungen_gesamt = bericht.abweichungen.len() as i64;
    bericht.abweichungen.truncate(500);
    bericht.dauer_s = start.elapsed().as_secs_f64();
    fortschritt.melden("Fertig", 1.0);
    Ok(bericht)
}

struct BelegQuelle<'a> {
    projekt_id: i64,
    kunde_id: i64,
    auftrag: &'a str,
    slot: &'a str,
    slot_nr: i64,
    vp: Option<texte::Vorspann>,
    lv: texte::Lv,
    npn: Option<texte::Nachspann>,
    np_text: String,
    abschlaege: Vec<texte::AltAbschlag>,
    eintrag: Option<Value>,
}

fn beleg_anlegen(tx: &rusqlite::Connection, bericht: &mut Bericht, kunden_namen: &HashMap<i64, String>, q: BelegQuelle) -> Res<i64> {
    let r = q.eintrag.as_ref();
    let jr = |k: &str| r.map(|r| s(r, k)).unwrap_or_default();
    let titel = q.vp.as_ref().map(|v| v.belegart.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| jr("LV_Bez"));
    let art = if titel.is_empty() { art_aus_slot(q.slot_nr) } else { art_aus_titel(&titel) };
    let art = if art == "aufstellung" && q.slot_nr == 54 { "rechnung" } else { art };
    let datum = q
        .vp
        .as_ref()
        .map(|v| v.datum.clone())
        .filter(|d| !d.is_empty())
        .or_else(|| r.map(|r| texte::seriendatum(ganz(r, "ReDatumI"))))
        .unwrap_or_default();
    let mwst_journal = r.map(|r| dez(r, "MwSt"));
    let mwst = q.npn.as_ref().and_then(|n| n.mwst).or(mwst_journal).unwrap_or(Decimal::from(19));
    let netto_rechnung = r.is_some_and(|r| s(r, "Nettorechnung") == "-1" || s(r, "Nettorechnung") == "1");
    let journal_status = r.map(|r| ganz(r, "Status")).unwrap_or(0);
    let forderung = crate::belege::arten::art(art).forderung;
    let status = match (journal_status, forderung) {
        (2, true) => "festgeschrieben",
        (2, false) => "gedruckt",
        _ => "entwurf",
    };
    let nummer = q.vp.as_ref().map(|v| v.nummer.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| jr("ReNummer"));
    let adresse = match &q.vp {
        Some(v) => v.adresse.join("\n"),
        None => [jr("Name1"), jr("Name2"), jr("Name3"), jr("Strasse"), format!("{} {}", jr("Plz"), jr("Ort")).trim().to_string()]
            .into_iter()
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
    };
    let npn = q.npn.unwrap_or_default();
    let nachtext = if npn.text.trim().is_empty() { q.np_text.clone() } else { npn.text.clone() };
    let mut beleg = Beleg {
        projekt_id: q.projekt_id,
        kunde_id: q.kunde_id,
        art: art.into(),
        titel: if titel.is_empty() { crate::belege::arten::art(art).titel.to_string() } else { titel },
        nummer,
        datum,
        adresse,
        vortext: q.vp.as_ref().map(|v| v.vortext.clone()).unwrap_or_default(),
        nachtext,
        leistung: jr("LieferDatumText"),
        mwst_satz: if netto_rechnung && mwst.is_zero() { Decimal::from(19) } else { mwst },
        rabatt_text: if npn.rabatt_text.is_empty() { "Rabatt".into() } else { npn.rabatt_text.clone() },
        rabatt_prozent: npn.rabatt,
        skonto_prozent: npn.skonto,
        skonto_tage: npn.skonto_tage,
        zahlungsziel_tage: npn.ziel_tage,
        netto_rechnung: netto_rechnung || (mwst.is_zero() && forderung && q.vp.is_some()),
        status: status.into(),
        waehrung: if q.lv.waehrung.is_empty() { "EUR".into() } else { q.lv.waehrung.clone() },
        alt_slot: q.slot.to_string(),
        alt_betrag: jr("RechBetrag"),
        mahnstufe: r.map(|r| ganz(r, "MahnStufe")).filter(|m| *m != 99).unwrap_or(0),
        gedruckt_am: r.map(|r| texte::seriendatum(ganz(r, "Dru_DatumI"))).unwrap_or_default(),
        angelegt: r.map(|r| texte::seriendatum(ganz(r, "Speich_DatumI"))).unwrap_or_default(),
        geaendert: db::jetzt(),
        ..Default::default()
    };
    if !beleg.datum.is_empty() && beleg.waehrung == "EUR" && beleg.datum.as_str() < "2002-01-01" {
        beleg.waehrung = "DM".into();
    }
    let beleg_id = beleg.speichern(tx)?;
    bericht.zaehlen("Belege");

    let mut positionen = vec![];
    for (i, p) in q.lv.positionen.iter().enumerate() {
        let mut pos = Position {
            beleg_id, pos: i as i64, art: p.art.clone(), posnr: p.posnr.clone(), text: p.text.clone(), menge: p.menge, einheit: p.einheit.clone(),
            ep: p.ep, rabatt: p.rabatt, ..Default::default()
        };
        if !matches!(pos.art.as_str(), "N" | "G" | "T" | "K" | "A" | "B") {
            pos.art = "K".into();
        }
        pos.gp = if matches!(pos.art.as_str(), "N" | "A" | "B") { rechnen::gp(&pos) } else { Decimal::ZERO };
        if pos.art == "N" && pos.gp != p.gp && !p.gp.is_zero() {
            // Gedruckt wurde der Gesamtpreis aus GHFakt – den behalten.
            pos.gp = p.gp;
            bericht.zaehlen("Positionen mit GP aus GHFakt (anders gerundet)");
        }
        pos.speichern(tx)?;
        positionen.push(pos);
        bericht.zaehlen("Positionen");
    }
    let mut zuschlaege = vec![];
    for (i, z) in npn.zuschlaege.iter().enumerate() {
        let art = match z.modus {
            1 => "betrag_netto",
            2 => "prozent_brutto",
            3 => "betrag_brutto",
            _ => "prozent_netto",
        };
        let mut zu = Zuschlag { beleg_id, pos: i as i64, text: z.text.clone(), wert: z.wert, art: art.into(), ..Default::default() };
        zu.speichern(tx)?;
        zuschlaege.push(zu);
    }
    let mut abzuege = vec![];
    if matches!(art, "schluss" | "abschlag" | "rechnung") {
        for (i, a) in q.abschlaege.iter().enumerate() {
            let mut ab = AbschlagAbzug {
                beleg_id, pos: i as i64, text: a.text.clone(), nummer: a.nummer.clone(), datum: a.datum.clone(), netto: a.netto, mwst: a.mwst, brutto: a.brutto, bezahlt: a.brutto, ..Default::default()
            };
            ab.speichern(tx)?;
            abzuege.push(ab);
        }
    }

    // Summen berechnen; maßgeblich für das Rechnungsbuch bleibt der Betrag aus GHFakt.
    let summen = rechnen::summen(&beleg, &positionen, &zuschlaege, &abzuege);
    beleg.netto = summen.netto;
    beleg.mwst = summen.mwst;
    beleg.brutto = summen.gesamt;
    beleg.abzug = summen.abzug;
    beleg.zahlbetrag = summen.zahlbetrag;
    if let Some(r) = r {
        let alt = dez(r, "RechBetrag");
        if (alt - summen.zahlbetrag).abs() > Decimal::new(1, 2) {
            if q.vp.is_some() {
                bericht.abweichungen.push(Abweichung {
                    beleg: format!("{} {} (Auftrag {}.{})", beleg.titel, beleg.nummer, q.auftrag, q.slot),
                    kunde: kunden_namen.get(&q.kunde_id).cloned().unwrap_or_default(),
                    datum: beleg.datum.clone(),
                    berechnet: summen.zahlbetrag,
                    rechnungsbuch: alt,
                    waehrung: beleg.waehrung.clone(),
                });
            }
            if positionen.is_empty() || q.vp.is_none() {
                beleg.zahlbetrag = alt;
                beleg.brutto = alt + summen.abzug;
            } else {
                beleg.zahlbetrag = alt;
            }
        }
        // Zahlung: MahnStufe 99 = bezahlt
        let zahl_betrag = dez(r, "ZahlBetrag");
        let abzug = dez(r, "Abzug");
        if ganz(r, "MahnStufe") == 99 || !zahl_betrag.is_zero() {
            let mut z = Zahlung {
                beleg_id,
                datum: texte::seriendatum(ganz(r, "ZahlDatumI")),
                betrag: if zahl_betrag.is_zero() && ganz(r, "MahnStufe") == 99 { beleg.zahlbetrag - abzug } else { zahl_betrag },
                abzug,
                notiz: "aus GHFakt".into(),
                ..Default::default()
            };
            if ganz(r, "MahnStufe") == 99 && z.betrag + z.abzug < beleg.zahlbetrag {
                // In GHFakt als bezahlt markiert: Rest (Rundung, Skonto) gilt als Abzug.
                z.abzug = beleg.zahlbetrag - z.betrag;
            }
            z.speichern(tx)?;
            beleg.bezahlt = z.betrag + z.abzug;
            beleg.bezahlt_am = z.datum.clone();
            bericht.zaehlen("Zahlungen");
        }
    } else if crate::belege::arten::art(art).forderung && q.slot_nr >= 51 {
        bericht.zaehlen("Belege ohne Rechnungsbuch-Eintrag");
    }
    beleg.speichern(tx)?;
    Ok(beleg_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordnername_zu_kunden_id() {
        assert_eq!(kunden_id("G0402150.706").as_deref(), Some("0402150706"));
        assert_eq!(kunden_id("Gxx"), None);
        assert_eq!(kunden_id("99986"), None);
    }

    /// Voller Import eines echten GHFakt-Ordners: `MRFAKT_GHFAKT=C:\Entwicklung\GHFakt cargo test voller_import -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn voller_import() {
        let ordner = std::env::var("MRFAKT_GHFAKT").expect("MRFAKT_GHFAKT setzen");
        let arbeit = std::env::temp_dir().join("mrfakt-import-test");
        let ziel = std::env::var("MRFAKT_DB").map(PathBuf::from).unwrap_or(arbeit.join("test.db"));
        let _ = std::fs::remove_file(&ziel);
        let db = db::oeffnen(&ziel).unwrap();
        let melden = |t: &str, a: f64| eprintln!("{:>3.0} % {t}", a * 100.0);
        let b = importieren(&db, Path::new(&ordner), &arbeit, &melden).unwrap();
        eprintln!("{}", serde_json::to_string_pretty(&b.anzahl).unwrap());
        eprintln!("Abweichungen: {}", b.abweichungen_gesamt);
        for a in b.abweichungen.iter().take(40) {
            eprintln!("  {} {} {} berechnet {} statt {} {}", a.datum, a.beleg, a.kunde, a.berechnet, a.rechnungsbuch, a.waehrung);
        }
        for h in b.hinweise.iter().take(40) {
            eprintln!("  ! {h}");
        }
        eprintln!("Dauer {:.1} s, Datenbank {}", b.dauer_s, ziel.display());
    }
}
