use crate::backend::MountBackend;
use crate::database::LayoutDatabase;
use crate::engine::LayoutEngine;
use crate::error::{LayerfsError, Result};
use crate::model::{Layout, MountStepDef};

pub struct MountTransaction<'a> {
    layout: &'a Layout,
    db: &'a LayoutDatabase,
    engine: &'a LayoutEngine,
    backend: &'a dyn MountBackend,
    completed: Vec<usize>,
}

impl<'a> MountTransaction<'a> {
    pub fn new(
        layout: &'a Layout,
        db: &'a LayoutDatabase,
        engine: &'a LayoutEngine,
        backend: &'a dyn MountBackend,
    ) -> Self {
        Self {
            layout,
            db,
            engine,
            backend,
            completed: Vec::new(),
        }
    }

    pub fn execute(&mut self) -> Result<()> {
        for (i, step) in self.layout.steps.iter().enumerate() {
            match self.execute_step(step) {
                Ok(()) => self.completed.push(i),
                Err(e) => {
                    self.rollback();
                    return Err(LayerfsError::MountFailed {
                        position: i,
                        reason: e.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn execute_step(&self, step: &MountStepDef) -> Result<()> {
        match step {
            MountStepDef::Layer(layer_name) => {
                let layer = self.db.load_layer(layer_name)?;
                let lower_dirs = self.engine.resolve_lower_dirs(layer_name)?;

                std::fs::create_dir_all(&layer.upper_dir)?;
                std::fs::create_dir_all(&layer.work_dir)?;
                std::fs::create_dir_all(&layer.mount_point)?;

                self.backend.mount_overlay(
                    &lower_dirs,
                    &layer.upper_dir,
                    &layer.work_dir,
                    &layer.mount_point,
                )
            }
            MountStepDef::Bind { source, target } => self.backend.bind_mount(source, target),
        }
    }

    fn rollback(&self) {
        for &i in self.completed.iter().rev() {
            let _ = self.undo_step(&self.layout.steps[i]);
        }
    }

    fn undo_step(&self, step: &MountStepDef) -> Result<()> {
        match step {
            MountStepDef::Layer(layer_name) => {
                let layer = self.db.load_layer(layer_name)?;
                self.backend.unmount_overlay(&layer.mount_point)
            }
            MountStepDef::Bind { target, .. } => self.backend.unbind_mount(target),
        }
    }
}

pub struct UnmountTransaction<'a> {
    layout: &'a Layout,
    db: &'a LayoutDatabase,
    backend: &'a dyn MountBackend,
}

impl<'a> UnmountTransaction<'a> {
    pub fn new(layout: &'a Layout, db: &'a LayoutDatabase, backend: &'a dyn MountBackend) -> Self {
        Self {
            layout,
            db,
            backend,
        }
    }

    pub fn execute(&self) -> Result<()> {
        for (i, step) in self.layout.steps.iter().enumerate().rev() {
            let result = match step {
                MountStepDef::Bind { target, .. } => self.backend.unbind_mount(target),
                MountStepDef::Layer(layer_name) => {
                    let layer = self.db.load_layer(layer_name)?;
                    self.backend.unmount_overlay(&layer.mount_point)
                }
            };
            if let Err(e) = result {
                return Err(LayerfsError::MountFailed {
                    position: i,
                    reason: format!("unmount failed: {e}"),
                });
            }
        }
        Ok(())
    }
}
