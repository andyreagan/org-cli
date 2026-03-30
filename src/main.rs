use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod blog;
mod build;
mod commands;
mod config;
mod html;
mod normalise;
mod parser;
mod postprocess;
mod render;
mod site;
mod types;

use commands::*;

#[derive(Parser)]
#[clap(name = "org-cli")]
#[clap(about = "CLI for interacting with org-mode files")]
#[clap(version)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all TODO/NEXT/WAITING items across files, grouped by keyword
    List {
        #[clap(default_value = ".")]
        path: PathBuf,
    },

    /// Add a new TODO item
    Add {
        text: String,
        #[clap(long, short)]
        file: PathBuf,
        #[clap(long, short)]
        tag: Option<String>,
    },

    /// Mark an item as DONE
    Done {
        file: PathBuf,
        line: usize,
    },

    /// Mark an item as CANCELLED
    Cancel {
        file: PathBuf,
        line: usize,
    },

    /// Mark an item as WAITING
    Wait {
        file: PathBuf,
        line: usize,
        #[clap(long, short)]
        date: Option<String>,
    },

    /// Reschedule an item to a new date
    Reschedule {
        file: PathBuf,
        line: usize,
        #[clap(long, short)]
        date: String,
    },

    /// Pretty-print headings and TODOs in a file
    Show {
        file: PathBuf,
    },

    /// Export org files as HTML site with resolved org-id links
    Export {
        path: PathBuf,
        #[clap(long, short)]
        output: PathBuf,
    },

    /// Normalise .org source files in-place (flatten nested id: links,
    /// consolidate :BACKLINKS: drawers). Safe to run repeatedly.
    Normalise {
        /// Directory of .org files to normalise (default: current directory)
        #[clap(default_value = ".")]
        dir: PathBuf,

        /// Print what would change without writing any files
        #[clap(long)]
        dry_run: bool,
    },

    /// Full site build: normalise → blog index → export HTML → post-process
    Build {
        /// Project root containing org-cli.toml (default: current directory)
        #[clap(default_value = ".")]
        dir: PathBuf,

        /// Path to config file (default: <dir>/org-cli.toml)
        #[clap(long)]
        config: Option<PathBuf>,

        /// Force a full rebuild, ignoring the incremental cache
        #[clap(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { path }               => list_todos(&path)?,
        Commands::Add { text, file, tag }     => add_todo(&text, &file, tag.as_deref())?,
        Commands::Done { file, line }         => mark_done(&file, line)?,
        Commands::Cancel { file, line }       => mark_cancelled(&file, line)?,
        Commands::Wait { file, line, date }   => mark_waiting(&file, line, date.as_deref())?,
        Commands::Reschedule { file, line, date } => reschedule(&file, line, &date)?,
        Commands::Show { file }               => show_file(&file)?,

        Commands::Export { path, output } => {
            html::export_site(&path, &output)?;
            println!("Exported to {}", output.display());
        }

        Commands::Normalise { dir, dry_run } => {
            let modified = normalise::normalise_dir(&dir, dry_run)?;
            if modified.is_empty() {
                println!("Nothing to normalise.");
            } else {
                for p in &modified {
                    println!("{} {}", if dry_run { "would modify" } else { "modified" }, p.display());
                }
                println!("{} file(s) {}.", modified.len(),
                    if dry_run { "would be modified" } else { "modified" });
            }
        }

        Commands::Build { dir, config: config_path, force } => {
            build::run_build(&dir, config_path.as_deref(), force)?;
        }
    }

    Ok(())
}
