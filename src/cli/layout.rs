use clap::Subcommand;
use clap_complete::engine::ArgValueCompleter;

use fpj::engine::LayoutEngine;
use fpj::error::Result;

use super::complete;

#[derive(Subcommand)]
pub enum LayoutCommand {
    /// Create a new empty layout
    Create {
        /// Layout name
        name: String,
    },
    /// Remove a layout and its step definitions
    Remove {
        /// Layout name
        #[arg(add = ArgValueCompleter::new(complete::complete_layout_names))]
        name: String,
    },
    /// List all layouts
    List,
    /// Show layout details
    Show {
        /// Layout name
        #[arg(add = ArgValueCompleter::new(complete::complete_layout_names))]
        name: String,
    },
}

pub fn handle(cmd: LayoutCommand, engine: &LayoutEngine) -> Result<()> {
    match cmd {
        LayoutCommand::Create { name } => {
            engine.create_layout(&name)?;
            println!("Created layout '{name}'");
        }
        LayoutCommand::Remove { name } => {
            engine.remove_layout(&name)?;
            println!("Removed layout '{name}'");
        }
        LayoutCommand::List => {
            let layouts = engine.list_layouts()?;
            if layouts.is_empty() {
                println!("No layouts defined");
            } else {
                for name in &layouts {
                    println!("  {name}");
                }
            }
        }
        LayoutCommand::Show { name } => {
            let layout = engine.get_layout(&name)?;
            println!("Layout: {}", layout.name);
            println!("Steps ({}):", layout.steps.len());
            for (i, step) in layout.steps.iter().enumerate() {
                println!("  [{i}] {}", step.description());
            }
        }
    }
    Ok(())
}
