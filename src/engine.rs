use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::backend::MountBackend;
use crate::database::LayoutDatabase;
use crate::error::{LayerfsError, Result};
use crate::model::{Layer, LayerRole, LayerSource, Layout, MountStepDef};
use crate::operations::{MountTransaction, UnmountTransaction};

/// Central coordinator that ties together the database, mount backend, and
/// domain logic for layers and layouts.
pub struct LayoutEngine {
    db: LayoutDatabase,
    backend: Box<dyn MountBackend>,
}

impl LayoutEngine {
    pub fn new(db: LayoutDatabase, backend: Box<dyn MountBackend>) -> Self {
        Self { db, backend }
    }

    pub fn db(&self) -> &LayoutDatabase {
        &self.db
    }

    // --- Layer operations ---

    pub fn create_layer(
        &self,
        name: &str,
        source: LayerSource,
        mount_point: PathBuf,
    ) -> Result<Layer> {
        let data_dir = layers_data_dir();
        let upper_dir = data_dir.join(name).join("upper");
        let work_dir = data_dir.join(name).join("work");

        let layer = Layer {
            name: name.to_string(),
            source,
            mount_point,
            role: LayerRole::Writable,
            upper_dir,
            work_dir,
        };
        layer.validate()?;

        if let LayerSource::Layer(ref base_name) = layer.source {
            let base = self.db.load_layer(base_name)?;
            if base.role != LayerRole::Locked {
                return Err(LayerfsError::BaseLayerNotLocked(base_name.clone()));
            }
        }

        std::fs::create_dir_all(&layer.upper_dir)?;
        std::fs::create_dir_all(&layer.work_dir)?;

        self.db.create_layer(&layer)?;
        Ok(layer)
    }

    pub fn remove_layer(&self, name: &str) -> Result<()> {
        let children = self.db.layer_children(name)?;
        if !children.is_empty() {
            return Err(LayerfsError::LayerHasChildren {
                name: name.to_string(),
                children: children.join(", "),
            });
        }

        // Load layer to get paths before removing DB record
        let layer = self.db.load_layer(name)?;
        self.db.remove_layer(name)?;

        // Clean up on-disk data directory
        let data_dir = layers_data_dir().join(name);
        if data_dir.exists() {
            std::fs::remove_dir_all(&data_dir).map_err(|e| {
                LayerfsError::Other(format!(
                    "removed layer '{}' from database but failed to clean up {}: {}",
                    name,
                    data_dir.display(),
                    e
                ))
            })?;
        }
        let _ = layer; // used above to verify existence

        Ok(())
    }

    pub fn get_layer(&self, name: &str) -> Result<Layer> {
        self.db.load_layer(name)
    }

    pub fn list_layers(&self) -> Result<Vec<String>> {
        self.db.list_layers()
    }

    pub fn lock_layer(&self, name: &str) -> Result<()> {
        let mut layer = self.db.load_layer(name)?;
        if layer.role != LayerRole::Writable {
            return Err(LayerfsError::InvalidRoleTransition {
                name: name.to_string(),
                current_role: layer.role.to_string(),
            });
        }
        layer.role = LayerRole::Locked;
        self.db.save_layer(&layer)
    }

    pub fn unlock_layer(&self, name: &str) -> Result<()> {
        let mut layer = self.db.load_layer(name)?;
        if layer.role != LayerRole::Locked {
            return Err(LayerfsError::InvalidRoleTransition {
                name: name.to_string(),
                current_role: layer.role.to_string(),
            });
        }
        layer.role = LayerRole::Writable;
        self.db.save_layer(&layer)
    }

    /// Recursively resolve the lowerdir chain for a layer, flattening all
    /// base-layer references into a single Vec of absolute paths.
    pub fn resolve_lower_dirs(&self, layer_name: &str) -> Result<Vec<PathBuf>> {
        let mut visited = HashSet::new();
        self.resolve_lower_dirs_inner(layer_name, &mut visited)
    }

    fn resolve_lower_dirs_inner(
        &self,
        layer_name: &str,
        visited: &mut HashSet<String>,
    ) -> Result<Vec<PathBuf>> {
        let layer = self.db.load_layer(layer_name)?;
        match &layer.source {
            LayerSource::Directory(path) => Ok(vec![path.clone()]),
            LayerSource::Layer(base_name) => {
                if !visited.insert(base_name.clone()) {
                    return Err(LayerfsError::CircularReference(base_name.clone()));
                }
                let base = self.db.load_layer(base_name)?;
                if base.role != LayerRole::Locked {
                    return Err(LayerfsError::BaseLayerNotLocked(base_name.clone()));
                }
                let mut dirs = vec![base.upper_dir.clone()];
                dirs.extend(self.resolve_lower_dirs_inner(base_name, visited)?);
                Ok(dirs)
            }
        }
    }

    // --- Layout operations ---

    pub fn create_layout(&self, name: &str) -> Result<Layout> {
        self.db.create_layout(name)?;
        Ok(Layout::new(name.to_string()))
    }

    pub fn remove_layout(&self, name: &str) -> Result<()> {
        self.db.remove_layout(name)
    }

    pub fn list_layouts(&self) -> Result<Vec<String>> {
        self.db.list_layouts()
    }

    pub fn get_layout(&self, name: &str) -> Result<Layout> {
        self.db.load_layout(name)
    }

    pub fn add_step(&self, layout_name: &str, step: MountStepDef) -> Result<()> {
        step.validate_paths()?;
        if let MountStepDef::Layer(ref layer_name) = step {
            if !self.db.layer_exists(layer_name)? {
                return Err(LayerfsError::LayerNotFound(layer_name.clone()));
            }
        }
        let mut layout = self.db.load_layout(layout_name)?;
        layout.steps.push(step);
        self.db.save_layout(&layout)?;
        Ok(())
    }

    pub fn remove_step(&self, layout_name: &str, position: usize) -> Result<()> {
        let mut layout = self.db.load_layout(layout_name)?;
        if position >= layout.steps.len() {
            return Err(LayerfsError::StepNotFound(
                position,
                layout_name.to_string(),
            ));
        }
        layout.steps.remove(position);
        self.db.save_layout(&layout)?;
        Ok(())
    }

    pub fn mount(&self, layout_name: &str) -> Result<()> {
        let layout = self.db.load_layout(layout_name)?;
        let mut tx = MountTransaction::new(&layout, &self.db, self, self.backend.as_ref());
        tx.execute()
    }

    pub fn unmount(&self, layout_name: &str) -> Result<()> {
        let layout = self.db.load_layout(layout_name)?;
        let tx = UnmountTransaction::new(&layout, &self.db, self.backend.as_ref());
        tx.execute()
    }

    pub fn restore(&self, layout_name: Option<&str>) -> Result<()> {
        let names = match layout_name {
            Some(name) => vec![name.to_string()],
            None => self.db.list_layouts()?,
        };

        for name in &names {
            let layout = self.db.load_layout(name)?;
            let mut tx = MountTransaction::new(&layout, &self.db, self, self.backend.as_ref());
            tx.execute()?;
        }

        Ok(())
    }

    pub fn status(&self, layout_name: &str) -> Result<LayoutStatus> {
        let layout = self.db.load_layout(layout_name)?;
        let mut step_statuses = Vec::new();

        for (i, step) in layout.steps.iter().enumerate() {
            let mounted = match step {
                MountStepDef::Layer(layer_name) => {
                    let layer = self.db.load_layer(layer_name)?;
                    self.backend.is_mounted(&layer.mount_point)?
                }
                MountStepDef::Bind { target, .. } => self.backend.is_mounted(target)?,
            };
            step_statuses.push(StepStatus {
                position: i,
                description: step.description(),
                mounted,
            });
        }

        Ok(LayoutStatus {
            name: layout_name.to_string(),
            steps: step_statuses,
        })
    }

    pub fn status_json(&self, layout_name: &str) -> Result<String> {
        let status = self.status(layout_name)?;
        Ok(serde_json::to_string_pretty(&status)?)
    }
}

/// Default path for the fpj SQLite database under the OS data directory.
pub fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| Path::new("/tmp").to_path_buf())
        .join("fpj")
        .join("fpj.db")
}

/// Base directory for internally managed layer data (upper/work dirs).
pub fn layers_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| Path::new("/tmp").to_path_buf())
        .join("fpj")
        .join("layers")
}

/// Per-layout mount status returned by [`LayoutEngine::status`].
#[derive(Debug, serde::Serialize)]
pub struct LayoutStatus {
    pub name: String,
    pub steps: Vec<StepStatus>,
}

/// Mount status of an individual step within a layout.
#[derive(Debug, serde::Serialize)]
pub struct StepStatus {
    pub position: usize,
    pub description: String,
    pub mounted: bool,
}
