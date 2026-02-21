use std::path::PathBuf;

use clap::Subcommand;

use fpj::engine::LayoutEngine;
use fpj::error::{LayerfsError, Result};
use fpj::model::MountStepDef;

#[derive(Subcommand)]
pub enum StepCommand {
    /// Add a layer mount step
    AddLayer {
        /// Layout name
        layout: String,

        /// Layer name to mount
        #[arg(long)]
        layer: String,
    },
    /// Add a bind mount step
    AddBind {
        /// Layout name
        layout: String,

        /// Absolute path to source directory
        #[arg(long)]
        source: PathBuf,

        /// Absolute path to target
        #[arg(long)]
        target: PathBuf,
    },
    /// Remove a step by position
    Remove {
        /// Layout name
        layout: String,

        /// Step position (0-based)
        #[arg(long)]
        position: usize,
    },
    /// List steps in a layout
    List {
        /// Layout name
        layout: String,
    },
}

pub fn handle(cmd: StepCommand, engine: &LayoutEngine) -> Result<()> {
    match cmd {
        StepCommand::AddLayer { layout, layer } => {
            engine.add_step(&layout, MountStepDef::Layer(layer.clone()))?;
            println!("Added layer step '@{layer}' to layout '{layout}'");
        }
        StepCommand::AddBind {
            layout,
            source,
            target,
        } => {
            if !source.is_absolute() {
                return Err(LayerfsError::RelativePath(source));
            }
            if !target.is_absolute() {
                return Err(LayerfsError::RelativePath(target));
            }

            engine.add_step(
                &layout,
                MountStepDef::Bind { source, target },
            )?;
            println!("Added bind step to layout '{layout}'");
        }
        StepCommand::Remove { layout, position } => {
            engine.remove_step(&layout, position)?;
            println!("Removed step {position} from layout '{layout}'");
        }
        StepCommand::List { layout } => {
            let l = engine.get_layout(&layout)?;
            if l.steps.is_empty() {
                println!("Layout '{layout}' has no steps");
            } else {
                for (i, step) in l.steps.iter().enumerate() {
                    println!("  [{i}] {}", step.description());
                }
            }
        }
    }
    Ok(())
}
