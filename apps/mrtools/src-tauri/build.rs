//! Bettet den Katalog der Apps aus `apps/` ins Programm ein (ohne mrtools
//! selbst), damit mrtools ohne Quellcode auf der Platte auskommt.

use serde_json::{json, Value};
use std::{env, fs, path::Path};

fn read_json(path: &Path) -> Value {
    println!("cargo:rerun-if-changed={}", path.display());
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}

fn main() {
    let apps_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    println!("cargo:rerun-if-changed={}", apps_dir.display());

    let mut dirs: Vec<_> = fs::read_dir(&apps_dir)
        .expect("apps/ nicht lesbar")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("src-tauri").join("tauri.conf.json").is_file())
        .collect();
    dirs.sort();

    let mut catalog = Vec::new();
    for dir in dirs {
        let folder = dir.file_name().unwrap().to_string_lossy().into_owned();
        if folder == "mrtools" {
            continue;
        }
        let package = read_json(&dir.join("package.json"));
        let tauri = read_json(&dir.join("src-tauri").join("tauri.conf.json"));
        let logo = dir.join("src").join("assets").join("logo.svg");
        println!("cargo:rerun-if-changed={}", logo.display());
        catalog.push(json!({
            "folder": folder,
            "name": tauri["productName"].as_str().unwrap_or(&folder),
            "description": package["description"],
            "icon": fs::read_to_string(&logo).ok(),
        }));
    }

    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("catalog.json");
    fs::write(out, serde_json::to_string(&catalog).unwrap()).unwrap();

    tauri_build::build()
}
