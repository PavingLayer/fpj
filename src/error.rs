use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LayerfsError {
    #[error("layout '{0}' not found")]
    LayoutNotFound(String),

    #[error("layout '{0}' already exists")]
    LayoutAlreadyExists(String),

    #[error("layer '{0}' not found")]
    LayerNotFound(String),

    #[error("layer '{0}' already exists")]
    LayerAlreadyExists(String),

    #[error("base layer '{0}' is not locked (current role: writable)")]
    BaseLayerNotLocked(String),

    #[error("circular layer reference detected: {0}")]
    CircularReference(String),

    #[error("layer '{name}' is referenced by child layer(s): {children}")]
    LayerHasChildren { name: String, children: String },

    #[error("step at position {0} not found in layout '{1}'")]
    StepNotFound(usize, String),

    #[error("step at position {position} in layout '{layout}' is not a layer")]
    NotALayer { position: usize, layout: String },

    #[error("layer '{name}' cannot transition: current role is {current_role}")]
    InvalidRoleTransition { name: String, current_role: String },

    #[error("mount failed at step {position}: {reason}")]
    MountFailed { position: usize, reason: String },

    #[error("unmount failed at {path}: {reason}")]
    UnmountFailed { path: PathBuf, reason: String },

    #[error("path is not absolute: {0}")]
    RelativePath(PathBuf),

    #[error("backend error: {0}")]
    Backend(String),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, LayerfsError>;
