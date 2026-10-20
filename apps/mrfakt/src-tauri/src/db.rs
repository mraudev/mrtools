//! SQLite-Datenbank: Öffnen, Schema-Migrationen und ein kleines Mapping von Structs auf Tabellen.
//!
//! Beträge und Mengen sind `Decimal` und liegen als TEXT in der Datenbank, damit nichts durch
//! Fließkomma-Rundung verloren geht. Datumswerte sind Text im Format `JJJJ-MM-TT`.

use rusqlite::types::{Value, ValueRef};
use rusqlite::{Connection, OptionalExtension, Row};
use rust_decimal::Decimal;
use std::path::Path;
use std::str::FromStr;

pub type Res<T> = Result<T, String>;

/// Wandelt beliebige Fehler in die Fehlermeldung für die Oberfläche.
pub fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// Ein Wert, der in einer Spalte gespeichert werden kann.
pub trait Feld: Sized {
    fn to_value(&self) -> Value;
    fn from_ref(v: ValueRef<'_>) -> Self;
}

impl Feld for String {
    fn to_value(&self) -> Value {
        Value::Text(self.clone())
    }
    fn from_ref(v: ValueRef<'_>) -> Self {
        match v {
            ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
            ValueRef::Integer(i) => i.to_string(),
            ValueRef::Real(f) => f.to_string(),
            _ => String::new(),
        }
    }
}

impl Feld for i64 {
    fn to_value(&self) -> Value {
        Value::Integer(*self)
    }
    fn from_ref(v: ValueRef<'_>) -> Self {
        match v {
            ValueRef::Integer(i) => i,
            ValueRef::Real(f) => f as i64,
            ValueRef::Text(t) => String::from_utf8_lossy(t).trim().parse().unwrap_or(0),
            _ => 0,
        }
    }
}

impl Feld for bool {
    fn to_value(&self) -> Value {
        Value::Integer(*self as i64)
    }
    fn from_ref(v: ValueRef<'_>) -> Self {
        i64::from_ref(v) != 0
    }
}

impl Feld for Option<i64> {
    fn to_value(&self) -> Value {
        match self {
            Some(i) => Value::Integer(*i),
            None => Value::Null,
        }
    }
    fn from_ref(v: ValueRef<'_>) -> Self {
        match v {
            ValueRef::Null => None,
            v => Some(i64::from_ref(v)),
        }
    }
}

impl Feld for Decimal {
    fn to_value(&self) -> Value {
        Value::Text(self.normalize().to_string())
    }
    fn from_ref(v: ValueRef<'_>) -> Self {
        match v {
            ValueRef::Text(t) => Decimal::from_str(String::from_utf8_lossy(t).trim()).unwrap_or_default(),
            ValueRef::Integer(i) => Decimal::from(i),
            ValueRef::Real(f) => Decimal::try_from(f).unwrap_or_default(),
            _ => Decimal::ZERO,
        }
    }
}

/// Eine Tabelle mit `id INTEGER PRIMARY KEY` und den Feldern des Structs als Spalten.
pub trait Datensatz: Sized {
    const TABELLE: &'static str;
    const SPALTEN: &'static [&'static str];
    fn id(&self) -> i64;
    fn set_id(&mut self, id: i64);
    fn werte(&self) -> Vec<Value>;
    fn aus_zeile(row: &Row<'_>) -> rusqlite::Result<Self>;

    fn select() -> String {
        format!("SELECT id, {} FROM {}", Self::SPALTEN.join(", "), Self::TABELLE)
    }

    /// Alle Datensätze, die `bedingung` erfüllen (z. B. `"WHERE kunde_id = ?1 ORDER BY datum"`).
    fn alle(db: &Connection, bedingung: &str, params: &[&dyn rusqlite::ToSql]) -> Res<Vec<Self>> {
        let sql = format!("{} {}", Self::select(), bedingung);
        let mut stmt = db.prepare_cached(&sql).map_err(err)?;
        let rows = stmt.query_map(params, |r| Self::aus_zeile(r)).map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    fn laden(db: &Connection, id: i64) -> Res<Self> {
        let sql = format!("{} WHERE id = ?1", Self::select());
        db.query_row(&sql, [id], |r| Self::aus_zeile(r))
            .optional()
            .map_err(err)?
            .ok_or_else(|| format!("Datensatz {id} in {} nicht gefunden", Self::TABELLE))
    }

    /// Legt den Datensatz an (bei `id == 0`) oder schreibt ihn zurück. Gibt die Id zurück.
    fn speichern(&mut self, db: &Connection) -> Res<i64> {
        let werte = self.werte();
        if self.id() == 0 {
            let platzhalter: Vec<String> = (1..=werte.len()).map(|i| format!("?{i}")).collect();
            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                Self::TABELLE,
                Self::SPALTEN.join(", "),
                platzhalter.join(", ")
            );
            db.prepare_cached(&sql).map_err(err)?.execute(rusqlite::params_from_iter(werte)).map_err(err)?;
            self.set_id(db.last_insert_rowid());
        } else {
            let sets: Vec<String> = Self::SPALTEN.iter().enumerate().map(|(i, s)| format!("{s} = ?{}", i + 1)).collect();
            let sql = format!("UPDATE {} SET {} WHERE id = ?{}", Self::TABELLE, sets.join(", "), werte.len() + 1);
            let mut alle = werte;
            alle.push(Value::Integer(self.id()));
            db.prepare_cached(&sql).map_err(err)?.execute(rusqlite::params_from_iter(alle)).map_err(err)?;
        }
        Ok(self.id())
    }

    fn loeschen(db: &Connection, id: i64) -> Res<()> {
        db.execute(&format!("DELETE FROM {} WHERE id = ?1", Self::TABELLE), [id]).map_err(err)?;
        Ok(())
    }
}

/// Erzeugt ein Struct samt `Datensatz`-Implementierung. Feldnamen = Spaltennamen.
#[macro_export]
macro_rules! datensatz {
    ($(#[$meta:meta])* $name:ident, $tabelle:literal { $($feld:ident : $typ:ty),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        #[serde(default)]
        pub struct $name {
            pub id: i64,
            $(pub $feld: $typ,)*
        }

        impl $crate::db::Datensatz for $name {
            const TABELLE: &'static str = $tabelle;
            const SPALTEN: &'static [&'static str] = &[$(stringify!($feld)),*];
            fn id(&self) -> i64 { self.id }
            fn set_id(&mut self, id: i64) { self.id = id; }
            fn werte(&self) -> Vec<rusqlite::types::Value> {
                vec![$($crate::db::Feld::to_value(&self.$feld)),*]
            }
            #[allow(unused_assignments)]
            fn aus_zeile(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
                let mut i = 1;
                Ok(Self {
                    id: row.get(0)?,
                    $($feld: { let v = <$typ as $crate::db::Feld>::from_ref(row.get_ref(i)?); i += 1; v },)*
                })
            }
        }
    };
}

const MIGRATIONEN: &[&str] = &[include_str!("schema/001.sql")];

pub fn oeffnen(pfad: &Path) -> Res<Connection> {
    if let Some(dir) = pfad.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    let db = Connection::open(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    einrichten(&db)?;
    Ok(db)
}

pub fn einrichten(db: &Connection) -> Res<()> {
    db.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")
        .map_err(err)?;
    let version = db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0)).map_err(err)? as usize;
    if version > MIGRATIONEN.len() {
        return Err("Die Datenbank stammt von einer neueren mrfakt-Version.".into());
    }
    for (i, sql) in MIGRATIONEN.iter().enumerate().skip(version) {
        let tx = db.unchecked_transaction().map_err(err)?;
        tx.execute_batch(sql).map_err(|e| format!("Migration {}: {e}", i + 1))?;
        tx.pragma_update(None, "user_version", (i + 1) as i64).map_err(err)?;
        tx.commit().map_err(err)?;
    }
    Ok(())
}

/// Datenbank im Speicher mit aktuellem Schema (für Tests).
#[cfg(test)]
pub fn test_db() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    einrichten(&db).unwrap();
    db
}

/// Liest eine Einstellung (JSON) oder gibt `None` zurück.
pub fn einstellung<T: serde::de::DeserializeOwned>(db: &Connection, schluessel: &str) -> Res<Option<T>> {
    let text: Option<String> = db
        .query_row("SELECT wert FROM einstellung WHERE schluessel = ?1", [schluessel], |r| r.get(0))
        .optional()
        .map_err(err)?;
    match text {
        Some(t) => serde_json::from_str(&t).map(Some).map_err(err),
        None => Ok(None),
    }
}

pub fn einstellung_setzen<T: serde::Serialize>(db: &Connection, schluessel: &str, wert: &T) -> Res<()> {
    let text = serde_json::to_string(wert).map_err(err)?;
    db.execute(
        "INSERT INTO einstellung (schluessel, wert) VALUES (?1, ?2)
         ON CONFLICT(schluessel) DO UPDATE SET wert = excluded.wert",
        [schluessel, &text],
    )
    .map_err(err)?;
    Ok(())
}

/// Nächste Nummer eines Nummernkreises ziehen (und hochzählen).
pub fn naechste_nummer(db: &Connection, kreis: &str) -> Res<String> {
    let (n, format): (i64, String) = db
        .query_row("SELECT naechste, format FROM nummernkreis WHERE name = ?1", [kreis], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()
        .map_err(err)?
        .unwrap_or((1, "{n}".into()));
    db.execute(
        "INSERT INTO nummernkreis (name, naechste, format) VALUES (?1, ?2, ?3)
         ON CONFLICT(name) DO UPDATE SET naechste = excluded.naechste",
        rusqlite::params![kreis, n + 1, format],
    )
    .map_err(err)?;
    let jahr = chrono::Local::now().format("%Y").to_string();
    Ok(format.replace("{n}", &n.to_string()).replace("{jahr}", &jahr))
}

pub fn heute() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn jetzt() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    datensatz!(Probe, "nummernkreis_probe" { name: String, wert: Decimal, aktiv: bool, ref_id: Option<i64> });

    #[test]
    fn schema_laesst_sich_anlegen_und_erneut_oeffnen() {
        let db = test_db();
        einrichten(&db).unwrap();
        let v: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v as usize, MIGRATIONEN.len());
    }

    #[test]
    fn datensatz_speichern_und_laden() {
        let db = test_db();
        db.execute_batch("CREATE TABLE nummernkreis_probe (id INTEGER PRIMARY KEY, name TEXT, wert TEXT, aktiv INTEGER, ref_id INTEGER)").unwrap();
        let mut p = Probe { name: "a".into(), wert: Decimal::new(1234, 2), aktiv: true, ..Default::default() };
        let id = p.speichern(&db).unwrap();
        let mut q = Probe::laden(&db, id).unwrap();
        assert_eq!(q.wert, Decimal::new(1234, 2));
        assert!(q.aktiv && q.ref_id.is_none());
        q.ref_id = Some(7);
        q.speichern(&db).unwrap();
        assert_eq!(Probe::laden(&db, id).unwrap().ref_id, Some(7));
        assert_eq!(Probe::alle(&db, "", &[]).unwrap().len(), 1);
    }

    #[test]
    fn nummernkreis_zaehlt_hoch() {
        let db = test_db();
        db.execute("INSERT INTO nummernkreis (name, naechste, format) VALUES ('rechnung', 8298, '{n}')", []).unwrap();
        assert_eq!(naechste_nummer(&db, "rechnung").unwrap(), "8298");
        assert_eq!(naechste_nummer(&db, "rechnung").unwrap(), "8299");
        assert_eq!(naechste_nummer(&db, "neu").unwrap(), "1");
    }
}
