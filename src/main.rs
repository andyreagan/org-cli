use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod html;
mod parser;
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
        /// Path to file or directory to scan
        #[clap(default_value = ".")]
        path: PathBuf,
    },
    
    /// Add a new TODO item
    Add {
        /// Text for the TODO item
        text: String,
        
        /// File to add the TODO to
        #[clap(long, short)]
        file: PathBuf,
        
        /// Tag to add
        #[clap(long, short)]
        tag: Option<String>,
    },
    
    /// Mark an item as DONE
    Done {
        /// File containing the item
        file: PathBuf,
        
        /// Line number of the item
        line: usize,
    },
    
    /// Mark an item as CANCELLED
    Cancel {
        /// File containing the item
        file: PathBuf,
        
        /// Line number of the item
        line: usize,
    },
    
    /// Mark an item as WAITING
    Wait {
        /// File containing the item
        file: PathBuf,
        
        /// Line number of the item
        line: usize,
        
        /// Optional scheduled date (YYYY-MM-DD)
        #[clap(long, short)]
        date: Option<String>,
    },
    
    /// Reschedule an item to a new date
    Reschedule {
        /// File containing the item
        file: PathBuf,
        
        /// Line number of the item
        line: usize,
        
        /// New scheduled date (YYYY-MM-DD)
        #[clap(long, short)]
        date: String,
    },
    
    /// Pretty-print headings and TODOs in a file
    Show {
        /// File to show
        file: PathBuf,
    },
    
    /// Export org files as HTML site with resolved org-id links
    Export {
        /// Path to directory of org files to export
        path: PathBuf,
        
        /// Output directory for HTML files
        #[clap(long, short)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::List { path } => {
            list_todos(&path)?;
        }
        Commands::Add { text, file, tag } => {
            add_todo(&text, &file, tag.as_deref())?;
        }
        Commands::Done { file, line } => {
            mark_done(&file, line)?;
        }
        Commands::Cancel { file, line } => {
            mark_cancelled(&file, line)?;
        }
        Commands::Wait { file, line, date } => {
            mark_waiting(&file, line, date.as_deref())?;
        }
        Commands::Reschedule { file, line, date } => {
            reschedule(&file, line, &date)?;
        }
        Commands::Show { file } => {
            show_file(&file)?;
        }
        Commands::Export { path, output } => {
            html::export_site(&path, &output)?;
            println!("Exported to {}", output.display());
        }
    }
    
    Ok(())
}
