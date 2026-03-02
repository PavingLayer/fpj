use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{LayerfsError, Result};

/// Whether a layer is writable (accepting changes) or locked (read-only base).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerRole {
    Writable,
    Locked,
}

impl std::fmt::Display for LayerRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayerRole::Writable => write!(f, "writable"),
            LayerRole::Locked => write!(f, "locked"),
        }
    }
}

/// Where a layer's lower directory comes from: either an on-disk path or
/// another (locked) layer referenced by name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerSource {
    Directory(PathBuf),
    Layer(String),
}

impl std::fmt::Display for LayerSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayerSource::Directory(p) => write!(f, "{}", p.display()),
            LayerSource::Layer(name) => write!(f, "@{name}"),
        }
    }
}

/// A named overlay layer with its source, mount point, role, and internal
/// upper/work directories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub source: LayerSource,
    pub mount_point: PathBuf,
    pub role: LayerRole,
    pub upper_dir: PathBuf,
    pub work_dir: PathBuf,
}

impl Layer {
    pub fn validate(&self) -> Result<()> {
        if !self.mount_point.is_absolute() {
            return Err(LayerfsError::RelativePath(self.mount_point.clone()));
        }
        if let LayerSource::Directory(p) = &self.source {
            if !p.is_absolute() {
                return Err(LayerfsError::RelativePath(p.clone()));
            }
        }
        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "layer '{}': {} -> {} [{}]",
            self.name,
            self.source,
            self.mount_point.display(),
            self.role,
        )
    }
}

/// A single step in a layout's mount sequence: either an overlay layer
/// reference or a bind mount with explicit source/target paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MountStepDef {
    Layer(String),
    Bind { source: PathBuf, target: PathBuf },
}

impl MountStepDef {
    pub fn validate_paths(&self) -> Result<()> {
        match self {
            MountStepDef::Layer(_) => Ok(()),
            MountStepDef::Bind { source, target } => {
                if !source.is_absolute() {
                    return Err(LayerfsError::RelativePath(source.clone()));
                }
                if !target.is_absolute() {
                    return Err(LayerfsError::RelativePath(target.clone()));
                }
                Ok(())
            }
        }
    }

    pub fn description(&self) -> String {
        match self {
            MountStepDef::Layer(name) => format!("layer @{name}"),
            MountStepDef::Bind { source, target } => {
                format!("bind {} -> {}", source.display(), target.display())
            }
        }
    }
}

/// An ordered list of mount steps that together form a complete filesystem view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub steps: Vec<MountStepDef>,
}

impl Layout {
    pub fn new(name: String) -> Self {
        Self {
            name,
            steps: Vec::new(),
        }
    }

    pub fn add_step(&mut self, step: MountStepDef) -> Result<()> {
        step.validate_paths()?;
        self.steps.push(step);
        Ok(())
    }

    pub fn remove_step(&mut self, position: usize) -> Result<MountStepDef> {
        if position >= self.steps.len() {
            return Err(LayerfsError::StepNotFound(position, self.name.clone()));
        }
        Ok(self.steps.remove(position))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_layer(source: LayerSource, mount_point: &str) -> Layer {
        Layer {
            name: "test".into(),
            source,
            mount_point: PathBuf::from(mount_point),
            role: LayerRole::Writable,
            upper_dir: PathBuf::from("/tmp/upper"),
            work_dir: PathBuf::from("/tmp/work"),
        }
    }

    #[test]
    fn validate_accepts_absolute_paths() {
        let layer = make_layer(
            LayerSource::Directory(PathBuf::from("/opt/src")),
            "/mnt/target",
        );
        assert!(layer.validate().is_ok());
    }

    #[test]
    fn validate_rejects_relative_mount_point() {
        let layer = make_layer(
            LayerSource::Directory(PathBuf::from("/opt/src")),
            "relative/path",
        );
        assert!(matches!(
            layer.validate(),
            Err(LayerfsError::RelativePath(_))
        ));
    }

    #[test]
    fn validate_rejects_relative_source_dir() {
        let layer = make_layer(
            LayerSource::Directory(PathBuf::from("relative/src")),
            "/mnt/target",
        );
        assert!(matches!(
            layer.validate(),
            Err(LayerfsError::RelativePath(_))
        ));
    }

    #[test]
    fn validate_skips_source_check_for_layer_ref() {
        let layer = make_layer(LayerSource::Layer("base".into()), "/mnt/target");
        assert!(layer.validate().is_ok());
    }

    #[test]
    fn step_validate_accepts_absolute_bind() {
        let step = MountStepDef::Bind {
            source: PathBuf::from("/src"),
            target: PathBuf::from("/dst"),
        };
        assert!(step.validate_paths().is_ok());
    }

    #[test]
    fn step_validate_rejects_relative_bind_source() {
        let step = MountStepDef::Bind {
            source: PathBuf::from("relative"),
            target: PathBuf::from("/dst"),
        };
        assert!(matches!(
            step.validate_paths(),
            Err(LayerfsError::RelativePath(_))
        ));
    }

    #[test]
    fn step_validate_rejects_relative_bind_target() {
        let step = MountStepDef::Bind {
            source: PathBuf::from("/src"),
            target: PathBuf::from("relative"),
        };
        assert!(matches!(
            step.validate_paths(),
            Err(LayerfsError::RelativePath(_))
        ));
    }

    #[test]
    fn step_validate_always_accepts_layer_ref() {
        let step = MountStepDef::Layer("anything".into());
        assert!(step.validate_paths().is_ok());
    }

    #[test]
    fn layout_add_step_validates() {
        let mut layout = Layout::new("test".into());
        let bad_step = MountStepDef::Bind {
            source: PathBuf::from("rel"),
            target: PathBuf::from("/abs"),
        };
        assert!(layout.add_step(bad_step).is_err());
        assert!(layout.steps.is_empty());
    }

    #[test]
    fn layout_remove_step_out_of_bounds() {
        let mut layout = Layout::new("test".into());
        assert!(matches!(
            layout.remove_step(0),
            Err(LayerfsError::StepNotFound(0, _))
        ));
    }

    #[test]
    fn layout_remove_step_returns_removed() {
        let mut layout = Layout::new("test".into());
        layout
            .add_step(MountStepDef::Layer("a".into()))
            .unwrap();
        layout
            .add_step(MountStepDef::Layer("b".into()))
            .unwrap();
        let removed = layout.remove_step(0).unwrap();
        assert!(matches!(removed, MountStepDef::Layer(name) if name == "a"));
        assert_eq!(layout.steps.len(), 1);
    }
}
