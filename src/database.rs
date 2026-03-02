use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{LayerfsError, Result};
use crate::model::{Layer, LayerRole, LayerSource, Layout, MountStepDef};

/// SQLite-backed persistence for layers and layouts.
///
/// All mutations are performed through individual SQL statements or
/// transactions; the schema is auto-migrated on [`open`](Self::open).
pub struct LayoutDatabase {
    conn: Connection,
}

impl LayoutDatabase {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS layouts (
                name       TEXT PRIMARY KEY,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS layers (
                name         TEXT PRIMARY KEY,
                source_type  TEXT NOT NULL,
                source_value TEXT NOT NULL,
                mount_point  TEXT NOT NULL,
                role         TEXT NOT NULL DEFAULT 'writable',
                upper_dir    TEXT NOT NULL,
                work_dir     TEXT NOT NULL,
                created_at   TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS mount_steps (
                layout_name TEXT    NOT NULL REFERENCES layouts(name) ON DELETE CASCADE,
                position    INTEGER NOT NULL,
                step_type   TEXT    NOT NULL,
                step_json   TEXT    NOT NULL,
                PRIMARY KEY (layout_name, position)
            );",
        )?;
        Ok(())
    }

    // --- Layout CRUD ---

    pub fn create_layout(&self, name: &str) -> Result<()> {
        let changed = self
            .conn
            .execute("INSERT OR IGNORE INTO layouts (name) VALUES (?1)", [name])?;
        if changed == 0 {
            return Err(LayerfsError::LayoutAlreadyExists(name.to_string()));
        }
        Ok(())
    }

    pub fn remove_layout(&self, name: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM mount_steps WHERE layout_name = ?1", [name])?;
        let changed = self
            .conn
            .execute("DELETE FROM layouts WHERE name = ?1", [name])?;
        if changed == 0 {
            return Err(LayerfsError::LayoutNotFound(name.to_string()));
        }
        Ok(())
    }

    pub fn list_layouts(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM layouts ORDER BY name")?;
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(names)
    }

    pub fn save_layout(&self, layout: &Layout) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT OR IGNORE INTO layouts (name) VALUES (?1)",
            [&layout.name],
        )?;

        tx.execute(
            "DELETE FROM mount_steps WHERE layout_name = ?1",
            [&layout.name],
        )?;

        let mut stmt = tx.prepare(
            "INSERT INTO mount_steps (layout_name, position, step_type, step_json) VALUES (?1, ?2, ?3, ?4)",
        )?;

        for (i, step) in layout.steps.iter().enumerate() {
            let (step_type, step_json) = serialize_step(step)?;
            stmt.execute(rusqlite::params![
                &layout.name,
                i as i64,
                step_type,
                step_json
            ])?;
        }

        drop(stmt);
        tx.commit()?;
        Ok(())
    }

    pub fn load_layout(&self, name: &str) -> Result<Layout> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM layouts WHERE name = ?1)",
            [name],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(LayerfsError::LayoutNotFound(name.to_string()));
        }

        let mut stmt = self.conn.prepare(
            "SELECT step_json FROM mount_steps WHERE layout_name = ?1 ORDER BY position",
        )?;

        let steps: Vec<MountStepDef> = stmt
            .query_map([name], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })?
            .map(|r| {
                let json = r.map_err(LayerfsError::Database)?;
                serde_json::from_str(&json).map_err(LayerfsError::Serialization)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Layout {
            name: name.to_string(),
            steps,
        })
    }

    pub fn layout_exists(&self, name: &str) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM layouts WHERE name = ?1)",
            [name],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    // --- Layer CRUD ---

    pub fn create_layer(&self, layer: &Layer) -> Result<()> {
        let (source_type, source_value) = serialize_source(&layer.source);
        let changed = self.conn.execute(
            "INSERT OR IGNORE INTO layers (name, source_type, source_value, mount_point, role, upper_dir, work_dir) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                &layer.name,
                source_type,
                source_value,
                layer.mount_point.to_str().unwrap_or(""),
                layer.role.to_string(),
                layer.upper_dir.to_str().unwrap_or(""),
                layer.work_dir.to_str().unwrap_or(""),
            ],
        )?;
        if changed == 0 {
            return Err(LayerfsError::LayerAlreadyExists(layer.name.clone()));
        }
        Ok(())
    }

    pub fn remove_layer(&self, name: &str) -> Result<()> {
        let changed = self
            .conn
            .execute("DELETE FROM layers WHERE name = ?1", [name])?;
        if changed == 0 {
            return Err(LayerfsError::LayerNotFound(name.to_string()));
        }
        Ok(())
    }

    pub fn load_layer(&self, name: &str) -> Result<Layer> {
        let mut stmt = self.conn.prepare(
            "SELECT source_type, source_value, mount_point, role, upper_dir, work_dir FROM layers WHERE name = ?1",
        )?;

        stmt.query_row([name], |row| {
            let source_type: String = row.get(0)?;
            let source_value: String = row.get(1)?;
            let mount_point: String = row.get(2)?;
            let role_str: String = row.get(3)?;
            let upper_dir: String = row.get(4)?;
            let work_dir: String = row.get(5)?;

            Ok((
                source_type,
                source_value,
                mount_point,
                role_str,
                upper_dir,
                work_dir,
            ))
        })
        .map_err(|_| LayerfsError::LayerNotFound(name.to_string()))
        .map(
            |(source_type, source_value, mount_point, role_str, upper_dir, work_dir)| {
                let source = deserialize_source(&source_type, &source_value);
                let role = match role_str.as_str() {
                    "locked" => LayerRole::Locked,
                    _ => LayerRole::Writable,
                };
                Layer {
                    name: name.to_string(),
                    source,
                    mount_point: PathBuf::from(mount_point),
                    role,
                    upper_dir: PathBuf::from(upper_dir),
                    work_dir: PathBuf::from(work_dir),
                }
            },
        )
    }

    pub fn save_layer(&self, layer: &Layer) -> Result<()> {
        let (source_type, source_value) = serialize_source(&layer.source);
        let changed = self.conn.execute(
            "UPDATE layers SET source_type = ?1, source_value = ?2, mount_point = ?3, role = ?4, upper_dir = ?5, work_dir = ?6 WHERE name = ?7",
            rusqlite::params![
                source_type,
                source_value,
                layer.mount_point.to_str().unwrap_or(""),
                layer.role.to_string(),
                layer.upper_dir.to_str().unwrap_or(""),
                layer.work_dir.to_str().unwrap_or(""),
                &layer.name,
            ],
        )?;
        if changed == 0 {
            return Err(LayerfsError::LayerNotFound(layer.name.clone()));
        }
        Ok(())
    }

    pub fn list_layers(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT name FROM layers ORDER BY name")?;
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(names)
    }

    /// Return names of layers that reference `name` as their source.
    pub fn layer_children(&self, name: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM layers WHERE source_type = 'layer' AND source_value = ?1 ORDER BY name",
        )?;
        let names: Vec<String> = stmt
            .query_map([name], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(names)
    }

    pub fn layer_exists(&self, name: &str) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM layers WHERE name = ?1)",
            [name],
            |row| row.get(0),
        )?;
        Ok(exists)
    }
}

fn serialize_step(step: &MountStepDef) -> Result<(&'static str, String)> {
    let step_type = match step {
        MountStepDef::Layer(_) => "layer",
        MountStepDef::Bind { .. } => "bind",
    };
    let json = serde_json::to_string(step)?;
    Ok((step_type, json))
}

fn serialize_source(source: &LayerSource) -> (&'static str, String) {
    match source {
        LayerSource::Directory(p) => ("directory", p.to_string_lossy().to_string()),
        LayerSource::Layer(name) => ("layer", name.clone()),
    }
}

fn deserialize_source(source_type: &str, source_value: &str) -> LayerSource {
    match source_type {
        "layer" => LayerSource::Layer(source_value.to_string()),
        _ => LayerSource::Directory(PathBuf::from(source_value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> LayoutDatabase {
        LayoutDatabase::open_in_memory().unwrap()
    }

    fn sample_layer(name: &str) -> Layer {
        Layer {
            name: name.to_string(),
            source: LayerSource::Directory(PathBuf::from("/opt/src")),
            mount_point: PathBuf::from("/mnt/target"),
            role: LayerRole::Writable,
            upper_dir: PathBuf::from("/data/upper"),
            work_dir: PathBuf::from("/data/work"),
        }
    }

    #[test]
    fn layer_round_trip() {
        let db = test_db();
        let layer = sample_layer("base");
        db.create_layer(&layer).unwrap();

        let loaded = db.load_layer("base").unwrap();
        assert_eq!(loaded.name, "base");
        assert_eq!(loaded.mount_point, PathBuf::from("/mnt/target"));
        assert_eq!(loaded.role, LayerRole::Writable);
    }

    #[test]
    fn layer_duplicate_rejected() {
        let db = test_db();
        let layer = sample_layer("dup");
        db.create_layer(&layer).unwrap();
        assert!(matches!(
            db.create_layer(&layer),
            Err(LayerfsError::LayerAlreadyExists(_))
        ));
    }

    #[test]
    fn layer_not_found() {
        let db = test_db();
        assert!(matches!(
            db.load_layer("nope"),
            Err(LayerfsError::LayerNotFound(_))
        ));
    }

    #[test]
    fn layer_save_updates_role() {
        let db = test_db();
        let mut layer = sample_layer("x");
        db.create_layer(&layer).unwrap();

        layer.role = LayerRole::Locked;
        db.save_layer(&layer).unwrap();

        let loaded = db.load_layer("x").unwrap();
        assert_eq!(loaded.role, LayerRole::Locked);
    }

    #[test]
    fn layout_round_trip_with_steps() {
        let db = test_db();
        db.create_layout("env").unwrap();

        let layout = Layout {
            name: "env".into(),
            steps: vec![
                MountStepDef::Layer("base".into()),
                MountStepDef::Bind {
                    source: PathBuf::from("/a"),
                    target: PathBuf::from("/b"),
                },
            ],
        };
        db.save_layout(&layout).unwrap();

        let loaded = db.load_layout("env").unwrap();
        assert_eq!(loaded.steps.len(), 2);
        assert!(matches!(&loaded.steps[0], MountStepDef::Layer(n) if n == "base"));
        assert!(matches!(&loaded.steps[1], MountStepDef::Bind { .. }));
    }

    #[test]
    fn layout_duplicate_rejected() {
        let db = test_db();
        db.create_layout("dup").unwrap();
        assert!(matches!(
            db.create_layout("dup"),
            Err(LayerfsError::LayoutAlreadyExists(_))
        ));
    }

    #[test]
    fn remove_nonexistent_layout() {
        let db = test_db();
        assert!(matches!(
            db.remove_layout("nope"),
            Err(LayerfsError::LayoutNotFound(_))
        ));
    }

    #[test]
    fn list_layers_sorted() {
        let db = test_db();
        db.create_layer(&sample_layer("beta")).unwrap();
        db.create_layer(&sample_layer("alpha")).unwrap();
        let names = db.list_layers().unwrap();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn layer_children_finds_references() {
        let db = test_db();
        let base = sample_layer("base");
        db.create_layer(&base).unwrap();

        let child = Layer {
            name: "child".into(),
            source: LayerSource::Layer("base".into()),
            mount_point: PathBuf::from("/mnt/child"),
            role: LayerRole::Writable,
            upper_dir: PathBuf::from("/data/child/upper"),
            work_dir: PathBuf::from("/data/child/work"),
        };
        db.create_layer(&child).unwrap();

        let children = db.layer_children("base").unwrap();
        assert_eq!(children, vec!["child"]);
    }
}
