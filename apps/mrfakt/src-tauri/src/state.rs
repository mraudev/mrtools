//! Die offene Datenbank und wo sie liegt.

use crate::db::{self, err, Res};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub db: Mutex<Option<Connection>>,
    pub pfad: Mutex<PathBuf>,
    pub config_dir: PathBuf,
}

#[derive(Default, Serialize, Deserialize)]
struct Konfig {
    datenbank: Option<PathBuf>,
}

impl AppState {
    pub fn neu(app: &AppHandle) -> Res<Self> {
        let config_dir = app.path().app_config_dir().map_err(err)?;
        let data_dir = app.path().app_data_dir().map_err(err)?;
        let konfig: Konfig = std::fs::read_to_string(config_dir.join("mrfakt.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let pfad = konfig.datenbank.unwrap_or_else(|| data_dir.join("mrfakt.db"));
        let conn = db::oeffnen(&pfad)?;
        Ok(Self { db: Mutex::new(Some(conn)), pfad: Mutex::new(pfad), config_dir })
    }

    pub fn mit_db<T>(&self, f: impl FnOnce(&Connection) -> Res<T>) -> Res<T> {
        let guard = self.db.lock().map_err(|_| "Datenbank gesperrt".to_string())?;
        let db = guard.as_ref().ok_or("Keine Datenbank geöffnet")?;
        f(db)
    }

    pub fn pfad(&self) -> PathBuf {
        self.pfad.lock().map(|p| p.clone()).unwrap_or_default()
    }

    fn konfig_speichern(&self, pfad: &Path) -> Res<()> {
        std::fs::create_dir_all(&self.config_dir).map_err(err)?;
        let text = serde_json::to_string_pretty(&Konfig { datenbank: Some(pfad.to_path_buf()) }).map_err(err)?;
        std::fs::write(self.config_dir.join("mrfakt.json"), text).map_err(err)
    }

    pub fn sicherungsordner(&self) -> PathBuf {
        self.pfad().parent().map(|p| p.join("Sicherungen")).unwrap_or_else(|| PathBuf::from("Sicherungen"))
    }

    /// Kopie der Datenbank in den Sicherungsordner; alte Sicherungen über `behalten` werden gelöscht.
    pub fn sichern(&self, behalten: usize) -> Res<PathBuf> {
        let ordner = self.sicherungsordner();
        std::fs::create_dir_all(&ordner).map_err(err)?;
        let ziel = ordner.join(format!("mrfakt-{}.db", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")));
        self.mit_db(|db| {
            db.execute("VACUUM INTO ?1", [ziel.to_string_lossy()]).map_err(err)?;
            Ok(())
        })?;
        let mut alle: Vec<PathBuf> = std::fs::read_dir(&ordner)
            .map_err(err)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("mrfakt-")))
            .collect();
        alle.sort();
        while alle.len() > behalten.max(1) {
            let _ = std::fs::remove_file(alle.remove(0));
        }
        Ok(ziel)
    }
}

#[derive(Serialize)]
pub struct DbInfo {
    pub pfad: String,
    pub sicherungen: String,
    pub groesse: u64,
    pub kunden: i64,
    pub belege: i64,
}

#[tauri::command]
pub fn db_info(state: State<AppState>) -> Res<DbInfo> {
    let pfad = state.pfad();
    let groesse = std::fs::metadata(&pfad).map(|m| m.len()).unwrap_or(0);
    state.mit_db(|db| {
        let zahl = |t: &str| db.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0)).unwrap_or(0);
        Ok(DbInfo {
            pfad: pfad.to_string_lossy().into_owned(),
            sicherungen: state.sicherungsordner().to_string_lossy().into_owned(),
            groesse,
            kunden: zahl("kunde"),
            belege: zahl("beleg"),
        })
    })
}

#[tauri::command]
pub fn sichern(state: State<AppState>) -> Res<String> {
    let behalten = state.mit_db(|db| Ok(crate::einstellungen::allgemein(db)?.sicherungen))?;
    Ok(state.sichern(behalten.max(10))?.to_string_lossy().into_owned())
}

/// Eine andere Datenbankdatei öffnen (oder neu anlegen) und sich merken.
#[tauri::command]
pub fn db_wechseln(state: State<AppState>, pfad: String) -> Res<()> {
    let pfad = PathBuf::from(pfad);
    let conn = db::oeffnen(&pfad)?;
    *state.db.lock().map_err(|_| "Datenbank gesperrt")? = Some(conn);
    *state.pfad.lock().map_err(|_| "Datenbank gesperrt")? = pfad.clone();
    state.konfig_speichern(&pfad)
}

/// Sicherung zurückspielen: die aktuelle Datenbank wird vorher selbst gesichert.
#[tauri::command]
pub fn wiederherstellen(state: State<AppState>, quelle: String) -> Res<()> {
    let quelle = PathBuf::from(quelle);
    // Prüfen, dass es eine mrfakt-Datenbank ist.
    db::oeffnen(&quelle)?;
    state.sichern(1000)?;
    let ziel = state.pfad();
    let mut guard = state.db.lock().map_err(|_| "Datenbank gesperrt")?;
    *guard = None;
    for ext in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{ext}", ziel.to_string_lossy()));
    }
    let ergebnis = std::fs::copy(&quelle, &ziel).map_err(err);
    *guard = Some(db::oeffnen(&ziel)?);
    ergebnis.map(|_| ())
}
