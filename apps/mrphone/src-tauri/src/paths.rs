// Datenordner. Die Electron-Version speichert in %APPDATA%\SIP Phone (Name aus der Zeit vor mrphone).
// Solange die Tauri-Version im Aufbau ist, arbeitet sie auf einer Kopie in einem eigenen Ordner –
// die installierte App bleibt unberührt. MRPHONE_DATA_DIR setzt den Ordner fest (Selbsttests).
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const LEGACY_DIR: &str = "SIP Phone";
const TEST_DIR: &str = "mrphone-tauri-test";
const COPY_FILES: [&str; 4] = [
    "config.json",
    "contacts.json",
    "history.json",
    "Local State",
];

fn appdata() -> PathBuf {
    PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default())
}

// Liefert den Ordner und ob es die Testkopie neben der installierten App ist (dann startet die
// Telefonie vorsichtig und verdrängt keine Anmeldung eines anderen Geräts).
pub fn data_dir() -> io::Result<(PathBuf, bool)> {
    if let Some(dir) = std::env::var_os("MRPHONE_DATA_DIR") {
        let dir = PathBuf::from(dir);
        fs::create_dir_all(&dir)?;
        return Ok((dir, false));
    }
    let dir = appdata().join(TEST_DIR);
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
        copy_legacy_data(&appdata().join(LEGACY_DIR), &dir)?;
    }
    Ok((dir, true))
}

// Einmalig beim ersten Start: Konten, Kontakte, Verlauf, Schlüssel ("Local State") und eigener Klingelton.
fn copy_legacy_data(src: &Path, dst: &Path) -> io::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    for name in COPY_FILES {
        let file = src.join(name);
        if file.is_file() {
            fs::copy(&file, dst.join(name))?;
        }
    }
    for entry in fs::read_dir(src)?.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("ringtone.") && entry.path().is_file() {
            fs::copy(entry.path(), dst.join(&name))?;
        }
    }
    Ok(())
}
