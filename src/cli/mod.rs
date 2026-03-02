mod complete;
mod layer;
mod layout;
mod step;

use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::engine::ArgValueCompleter;
use clap_complete::env::CompleteEnv;

use fpj::backend::create_backend;
use fpj::database::LayoutDatabase;
use fpj::engine::{default_db_path, LayoutEngine};
use fpj::error::Result;

#[derive(Parser)]
#[command(
    name = "fpj",
    about = "File Projector — flexible filesystem view manager"
)]
struct Cli {
    /// Path to database file (default: ~/.local/share/fpj/fpj.db)
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage layers
    Layer {
        #[command(subcommand)]
        command: layer::LayerCommand,
    },
    /// Manage layouts
    Layout {
        #[command(subcommand)]
        command: layout::LayoutCommand,
    },
    /// Manage mount steps within a layout
    Step {
        #[command(subcommand)]
        command: step::StepCommand,
    },
    /// Mount all steps of a layout atomically
    Mount {
        /// Layout name
        #[arg(add = ArgValueCompleter::new(complete::complete_layout_names))]
        layout: String,
    },
    /// Unmount all steps of a layout in reverse order
    Unmount {
        /// Layout name
        #[arg(add = ArgValueCompleter::new(complete::complete_layout_names))]
        layout: String,
    },
    /// Restore layouts from persisted definitions
    Restore {
        /// Layout name (omit for all layouts)
        #[arg(add = ArgValueCompleter::new(complete::complete_layout_names))]
        layout: Option<String>,
    },
    /// Show mount status
    Status {
        /// Layout name
        #[arg(add = ArgValueCompleter::new(complete::complete_layout_names))]
        layout: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

pub fn run() -> Result<()> {
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db_path);
    let db = LayoutDatabase::open(&db_path)?;
    let backend = create_backend();
    let engine = LayoutEngine::new(db, backend);

    match cli.command {
        Commands::Layer { command } => layer::handle(command, &engine),
        Commands::Layout { command } => layout::handle(command, &engine),
        Commands::Step { command } => step::handle(command, &engine),
        Commands::Mount { layout } => {
            engine.mount(&layout)?;
            println!("Mounted layout '{layout}'");
            Ok(())
        }
        Commands::Unmount { layout } => {
            engine.unmount(&layout)?;
            println!("Unmounted layout '{layout}'");
            Ok(())
        }
        Commands::Restore { layout } => {
            engine.restore(layout.as_deref())?;
            match layout {
                Some(name) => println!("Restored layout '{name}'"),
                None => println!("Restored all layouts"),
            }
            Ok(())
        }
        Commands::Status { layout, json } => {
            if json {
                println!("{}", engine.status_json(&layout)?);
            } else {
                let status = engine.status(&layout)?;
                println!("Layout: {}", status.name);
                for s in &status.steps {
                    let icon = if s.mounted { "●" } else { "○" };
                    println!("  [{icon}] {}: {}", s.position, s.description);
                }
            }
            Ok(())
        }
    }
}
