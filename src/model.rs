use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{LayerfsError, Result};

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
