// Protokoll für die Fehlersuche in <Datenordner>/sipphone.log – wie src/logger.js der Electron-Version:
// eine Zeile je Meldung mit Zeitstempel, bei 512 KB wird einmal rotiert (sipphone.log.1).
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
};

const MAX_BYTES: u64 = 512 * 1024;
static FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn init(dir: &std::path::Path) {
    *FILE.lock().unwrap() = Some(dir.join("sipphone.log"));
}

fn timestamp() -> String {
    // Ortszeit ohne zusätzliche Bibliothek: Windows liefert sie direkt.
    use windows::Win32::System::SystemInformation::GetLocalTime;
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

fn write(level: &str, text: &str) {
    let line = format!("{} {level} {text}\n", timestamp());
    eprint!("{line}");
    let guard = FILE.lock().unwrap();
    let Some(file) = guard.as_ref() else { return };
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(file) {
        let _ = f.write_all(line.as_bytes());
    }
    if fs::metadata(file).map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
        let backup = file.with_extension("log.1");
        let _ = fs::remove_file(&backup);
        let _ = fs::rename(file, backup);
    }
}

pub fn info(text: &str) {
    write("LOG", text);
}

pub fn warn(text: &str) {
    write("WARN", text);
}
