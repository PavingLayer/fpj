use std::path::PathBuf;

use clap::Subcommand;
use clap_complete::engine::ArgValueCompleter;

use fpj::engine::LayoutEngine;
use fpj::error::{LayerfsError, Result};
use fpj::model::LayerSource;

use super::complete;

#[derive(Subcommand)]
pub enum LayerCommand {
    /// Create a new layer
    Create {
        /// Layer name
        name: String,

        /// Source: absolute path for a directory, or @layer-name for a layer reference
        #[arg(long, add = ArgValueCompleter::new(complete::complete_layer_source))]
        source: String,

        /// Absolute path to the mount point
        #[arg(long)]
        mount_point: PathBuf,
    },
    /// Remove a layer
    Remove {
        /// Layer name
        #[arg(add = ArgValueCompleter::new(complete::complete_layer_names))]
        name: String,
    },
    /// List all layers
    List,
    /// Show layer details
    Show {
        /// Layer name
        #[arg(add = ArgValueCompleter::new(complete::complete_layer_names))]
        name: String,

        /// Show the resolved lower-dir chain (flattened ancestry)
        #[arg(long)]
        resolve: bool,
    },
    /// Lock a layer (writable -> locked)
    Lock {
        /// Layer name
        #[arg(add = ArgValueCompleter::new(complete::complete_layer_names))]
        name: String,
    },
    /// Unlock a layer (locked -> writable)
    Unlock {
        /// Layer name
        #[arg(add = ArgValueCompleter::new(complete::complete_layer_names))]
        name: String,
    },
}

fn parse_source(s: &str) -> Result<LayerSource> {
    if let Some(layer_name) = s.strip_prefix('@') {
        if layer_name.is_empty() {
            return Err(LayerfsError::Other("layer reference cannot be empty".into()));
        }
        Ok(LayerSource::Layer(layer_name.to_string()))
    } else {
        let path = PathBuf::from(s);
        if !path.is_absolute() {
            return Err(LayerfsError::RelativePath(path));
        }
        Ok(LayerSource::Directory(path))
    }
}

pub fn handle(cmd: LayerCommand, engine: &LayoutEngine) -> Result<()> {
    match cmd {
        LayerCommand::Create {
            name,
            source,
            mount_point,
        } => {
            if !mount_point.is_absolute() {
                return Err(LayerfsError::RelativePath(mount_point));
            }
            let source = parse_source(&source)?;
            engine.create_layer(&name, source, mount_point)?;
            println!("Created layer '{name}'");
        }
        LayerCommand::Remove { name } => {
            engine.remove_layer(&name)?;
            println!("Removed layer '{name}'");
        }
        LayerCommand::List => {
            let layers = engine.list_layers()?;
            if layers.is_empty() {
                println!("No layers defined");
            } else {
                for name in &layers {
                    let layer = engine.get_layer(name)?;
                    println!("  {}", layer.description());
                }
            }
        }
        LayerCommand::Show { name, resolve } => {
            let layer = engine.get_layer(&name)?;
            println!("Layer: {}", layer.name);
            println!("  Source:      {}", layer.source);
            println!("  Mount point: {}", layer.mount_point.display());
            println!("  Role:        {}", layer.role);
            println!("  Upper dir:   {}", layer.upper_dir.display());
            println!("  Work dir:    {}", layer.work_dir.display());
            if resolve {
                let lower_dirs = engine.resolve_lower_dirs(&name)?;
                println!("  Resolved lower dirs:");
                for (i, dir) in lower_dirs.iter().enumerate() {
                    println!("    [{i}] {}", dir.display());
                }
            }
        }
        LayerCommand::Lock { name } => {
            engine.lock_layer(&name)?;
            println!("Locked layer '{name}'");
        }
        LayerCommand::Unlock { name } => {
            engine.unlock_layer(&name)?;
            println!("Unlocked layer '{name}'");
        }
    }
    Ok(())
}
