//! SQLite persistence for app state (scan roots, pinned projects, preferences,
//! change events). Never touches Claude Code config files.

use crate::errors::AppResult;
use crate::models::{DashboardStats, Preferences, RecentChange};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct Storage {
    conn: Connection,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS scan_roots (
    path TEXT PRIMARY KEY,
    added_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS pinned_projects (
    path TEXT PRIMARY KEY,
    added_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS change_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL,
    kind TEXT NOT NULL,
    ts TEXT NOT NULL,
    hash TEXT
);
CREATE INDEX IF NOT EXISTS idx_change_events_ts ON change_events(ts DESC);
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

const MAX_CHANGE_EVENTS: i64 = 2000;

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

impl Storage {
    /// Open (creating if needed) the database file and run migrations.
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> AppResult<Self> {
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")
            .or_else(|_| conn.execute_batch("PRAGMA foreign_keys = ON;"))?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    // ------------------------------------------------------- preferences

    pub fn get_pref(&self, key: &str) -> AppResult<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM preferences WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn set_pref(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO preferences(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_preferences(&self) -> AppResult<Preferences> {
        let mut p = Preferences::default();
        if let Some(t) = self.get_pref("theme")? {
            p.theme = t;
        }
        if let Some(v) = self.get_pref("watchOnStart")? {
            p.watch_on_start = v == "true";
        }
        if let Some(v) = self.get_pref("notifications")? {
            p.notifications = v == "true";
        }
        if let Some(v) = self.get_pref("allowEditing")? {
            p.allow_editing = v == "true";
        }
        Ok(p)
    }

    pub fn set_preferences(&self, p: &Preferences) -> AppResult<()> {
        self.set_pref("theme", &p.theme)?;
        self.set_pref("watchOnStart", &p.watch_on_start.to_string())?;
        self.set_pref("notifications", &p.notifications.to_string())?;
        self.set_pref("allowEditing", &p.allow_editing.to_string())?;
        Ok(())
    }

    // ------------------------------------------------------- path lists

    fn list(&self, table: &str) -> AppResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT path FROM {table} ORDER BY path"))?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn add(&self, table: &str, path: &str) -> AppResult<()> {
        self.conn.execute(
            &format!("INSERT OR IGNORE INTO {table}(path, added_at) VALUES(?1, ?2)"),
            params![path, now()],
        )?;
        Ok(())
    }

    fn remove(&self, table: &str, path: &str) -> AppResult<bool> {
        Ok(self
            .conn
            .execute(&format!("DELETE FROM {table} WHERE path = ?1"), [path])?
            > 0)
    }

    pub fn list_scan_roots(&self) -> AppResult<Vec<String>> {
        self.list("scan_roots")
    }
    pub fn add_scan_root(&self, path: &str) -> AppResult<()> {
        self.add("scan_roots", path)
    }
    pub fn remove_scan_root(&self, path: &str) -> AppResult<bool> {
        self.remove("scan_roots", path)
    }
    pub fn list_projects(&self) -> AppResult<Vec<String>> {
        self.list("pinned_projects")
    }
    pub fn add_project(&self, path: &str) -> AppResult<()> {
        self.add("pinned_projects", path)
    }
    pub fn remove_project(&self, path: &str) -> AppResult<bool> {
        self.remove("pinned_projects", path)
    }

    // ------------------------------------------------------- change events

    pub fn record_change(&self, path: &str, kind: &str, hash: Option<&str>) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO change_events(path, kind, ts, hash) VALUES(?1, ?2, ?3, ?4)",
            params![path, kind, now(), hash],
        )?;
        // Keep the table bounded.
        self.conn.execute(
            "DELETE FROM change_events WHERE id <= (SELECT MAX(id) FROM change_events) - ?1",
            [MAX_CHANGE_EVENTS],
        )?;
        Ok(())
    }

    pub fn recent_changes(&self, limit: usize) -> AppResult<Vec<RecentChange>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, kind, ts, hash FROM change_events ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |r| {
            Ok(RecentChange {
                id: r.get(0)?,
                path: r.get(1)?,
                kind: r.get(2)?,
                ts: r.get(3)?,
                hash: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // ------------------------------------------------------- last snapshot meta

    pub fn save_last_scan(&self, stats: &DashboardStats) -> AppResult<()> {
        let json = serde_json::to_string(stats)?;
        self.conn.execute(
            "INSERT INTO meta(key, value) VALUES('last_snapshot', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [json],
        )?;
        Ok(())
    }

    pub fn load_last_scan(&self) -> AppResult<Option<DashboardStats>> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = 'last_snapshot'", [], |r| r.get(0))
            .optional()?;
        Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefs_roundtrip() {
        let s = Storage::open_in_memory().unwrap();
        assert_eq!(s.get_preferences().unwrap().theme, "system");
        let p = Preferences {
            theme: "dark".into(),
            watch_on_start: true,
            notifications: true,
            allow_editing: true,
        };
        s.set_preferences(&p).unwrap();
        let got = s.get_preferences().unwrap();
        assert_eq!(got.theme, "dark");
        assert!(got.watch_on_start && got.notifications && got.allow_editing);
        assert!(!Preferences::default().allow_editing);
        s.set_pref("theme", "light").unwrap();
        assert_eq!(s.get_pref("theme").unwrap().as_deref(), Some("light"));
    }

    #[test]
    fn roots_and_projects() {
        let s = Storage::open_in_memory().unwrap();
        s.add_scan_root("/b").unwrap();
        s.add_scan_root("/a").unwrap();
        s.add_scan_root("/a").unwrap();
        assert_eq!(s.list_scan_roots().unwrap(), vec!["/a", "/b"]);
        assert!(s.remove_scan_root("/a").unwrap());
        assert!(!s.remove_scan_root("/a").unwrap());
        s.add_project("/p").unwrap();
        assert_eq!(s.list_projects().unwrap(), vec!["/p"]);
        assert!(s.remove_project("/p").unwrap());
        assert!(s.list_projects().unwrap().is_empty());
    }

    #[test]
    fn changes_and_last_scan() {
        let s = Storage::open_in_memory().unwrap();
        s.record_change("/x", "modified", Some("abc")).unwrap();
        s.record_change("/y", "created", None).unwrap();
        let r = s.recent_changes(10).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].path, "/y");
        assert_eq!(r[1].hash.as_deref(), Some("abc"));
        assert_eq!(s.recent_changes(1).unwrap().len(), 1);

        assert!(s.load_last_scan().unwrap().is_none());
        let st = DashboardStats { total_artifacts: 7, ..Default::default() };
        s.save_last_scan(&st).unwrap();
        assert_eq!(s.load_last_scan().unwrap().unwrap().total_artifacts, 7);
    }

    #[test]
    fn file_db_persists() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("sub/db.sqlite");
        Storage::open(&p).unwrap().add_scan_root("/r").unwrap();
        assert_eq!(Storage::open(&p).unwrap().list_scan_roots().unwrap(), vec!["/r"]);
    }
}
