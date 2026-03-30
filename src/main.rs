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

// ============================================================
//  Top-level CLI
// ============================================================

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
    // ── Org-mirroring domains ────────────────────────────────

    /// Manage TODO state on individual items
    Todo {
        #[clap(subcommand)]
        sub: TodoCommands,
    },

    /// Manage SCHEDULED / DEADLINE dates on items
    Schedule {
        #[clap(subcommand)]
        sub: ScheduleCommands,
    },

    /// Time-based cross-file views (day, week, deadlines)
    Agenda {
        #[clap(subcommand)]
        sub: AgendaCommands,
    },

    /// Inspect the structure of a single org file
    Structure {
        #[clap(subcommand)]
        sub: StructureCommands,
    },

    /// Export org files to other formats
    Export {
        #[clap(subcommand)]
        sub: ExportCommands,
    },

    // ── Our custom site-tooling layer ────────────────────────

    /// Site build tooling (normalise, blog index, full build)
    Site {
        #[clap(subcommand)]
        sub: SiteCommands,
    },

    // ── Deprecated flat aliases (kept for back-compat) ───────

    /// [deprecated — use `todo list`] List active TODO items
    #[clap(hide = true)]
    List {
        #[clap(default_value = ".")]
        path: PathBuf,
    },

    /// [deprecated — use `todo add`] Add a new TODO item
    #[clap(hide = true)]
    Add {
        text: String,
        #[clap(long, short)]
        file: PathBuf,
        #[clap(long, short)]
        tag: Option<String>,
    },

    /// [deprecated — use `todo done`] Mark an item as DONE
    #[clap(hide = true)]
    Done {
        file: PathBuf,
        line: usize,
    },

    /// [deprecated — use `todo cancel`] Mark an item as CANCELLED
    #[clap(hide = true)]
    Cancel {
        file: PathBuf,
        line: usize,
    },

    /// [deprecated — use `todo wait`] Mark an item as WAITING
    #[clap(hide = true)]
    Wait {
        file: PathBuf,
        line: usize,
        #[clap(long, short)]
        date: Option<String>,
    },

    /// [deprecated — use `schedule set`] Reschedule an item
    #[clap(hide = true)]
    Reschedule {
        file: PathBuf,
        line: usize,
        #[clap(long, short)]
        date: String,
    },

    /// [deprecated — use `structure show`] Pretty-print a file
    #[clap(hide = true)]
    Show {
        file: PathBuf,
    },

    /// [deprecated — use `export html`] Export org files as HTML site
    #[clap(hide = true)]
    ExportFlat {
        path: PathBuf,
        #[clap(long, short)]
        output: PathBuf,
    },

    /// [deprecated — use `site normalise`] Normalise .org source files
    #[clap(hide = true)]
    Normalise {
        #[clap(default_value = ".")]
        dir: PathBuf,
        #[clap(long)]
        dry_run: bool,
    },

    /// [deprecated — use `site build`] Full site build pipeline
    #[clap(hide = true)]
    Build {
        #[clap(default_value = ".")]
        dir: PathBuf,
        #[clap(long)]
        config: Option<PathBuf>,
        #[clap(long)]
        force: bool,
    },
}

// ============================================================
//  todo subcommands
// ============================================================

#[derive(Subcommand)]
enum TodoCommands {
    /// List all active TODO/NEXT/WAITING/IN-PROGRESS items across files
    List {
        /// Directory or file to search (default: current directory)
        #[clap(default_value = ".")]
        path: PathBuf,
    },

    /// Add a new TODO item to a file
    Add {
        /// Heading text for the new TODO
        text: String,
        /// Target .org file
        #[clap(long, short)]
        file: PathBuf,
        /// Optional tag to attach
        #[clap(long, short)]
        tag: Option<String>,
    },

    /// Mark an item DONE (adds CLOSED timestamp)
    Done {
        /// Path to the .org file
        file: PathBuf,
        /// Line number of the heading
        line: usize,
    },

    /// Mark an item CANCELLED (adds CLOSED timestamp)
    Cancel {
        /// Path to the .org file
        file: PathBuf,
        /// Line number of the heading
        line: usize,
    },

    /// Mark an item WAITING (optionally set a follow-up SCHEDULED date)
    Wait {
        /// Path to the .org file
        file: PathBuf,
        /// Line number of the heading
        line: usize,
        /// Optional follow-up date (YYYY-MM-DD)
        #[clap(long, short)]
        date: Option<String>,
    },

    /// Mark an item NEXT
    Next {
        /// Path to the .org file
        file: PathBuf,
        /// Line number of the heading
        line: usize,
    },

    /// Set an arbitrary TODO keyword on an item
    Set {
        /// Path to the .org file
        file: PathBuf,
        /// Line number of the heading
        line: usize,
        /// Keyword to set (e.g. TODO, NEXT, IN-PROGRESS, WAITING, DONE, CANCELLED)
        keyword: String,
    },
}

// ============================================================
//  schedule subcommands
// ============================================================

#[derive(Subcommand)]
enum ScheduleCommands {
    /// Set or replace the SCHEDULED date on an item
    Set {
        /// Path to the .org file
        file: PathBuf,
        /// Line number of the heading
        line: usize,
        /// New scheduled date (YYYY-MM-DD)
        date: String,
    },

    /// Set or replace the DEADLINE date on an item
    Deadline {
        /// Path to the .org file
        file: PathBuf,
        /// Line number of the heading
        line: usize,
        /// Deadline date (YYYY-MM-DD)
        date: String,
    },

    /// Remove SCHEDULED and DEADLINE dates from an item
    Clear {
        /// Path to the .org file
        file: PathBuf,
        /// Line number of the heading
        line: usize,
    },
}

// ============================================================
//  agenda subcommands
// ============================================================

#[derive(Subcommand)]
enum AgendaCommands {
    /// Show items scheduled for a given day (default: today)
    Day {
        /// Date to show (YYYY-MM-DD, default: today)
        date: Option<String>,
        /// Directory or file to search (default: current directory)
        #[clap(long, default_value = ".")]
        path: PathBuf,
    },

    /// Show items scheduled within a week window (default: this week)
    Week {
        /// Start date (YYYY-MM-DD, default: today)
        date: Option<String>,
        /// Directory or file to search (default: current directory)
        #[clap(long, default_value = ".")]
        path: PathBuf,
    },

    /// Show items with approaching or overdue deadlines
    Deadlines {
        /// Directory or file to search (default: current directory)
        #[clap(default_value = ".")]
        path: PathBuf,
        /// Look-ahead window in days (default: 14)
        #[clap(long, default_value = "14")]
        days: i64,
    },
}

// ============================================================
//  structure subcommands
// ============================================================

#[derive(Subcommand)]
enum StructureCommands {
    /// Pretty-print headings and TODOs in a file
    Show {
        /// Path to the .org file
        file: PathBuf,
    },
}

// ============================================================
//  export subcommands
// ============================================================

#[derive(Subcommand)]
enum ExportCommands {
    /// Export org files as an HTML site with resolved org-id links
    Html {
        /// Directory or file to export
        path: PathBuf,
        /// Output directory for generated HTML
        #[clap(long, short)]
        output: PathBuf,
    },
}

// ============================================================
//  site subcommands
// ============================================================

#[derive(Subcommand)]
enum SiteCommands {
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

    /// Normalise .org source files in-place
    ///
    /// Flattens nested id: links, strips zero-width spaces, and consolidates
    /// :BACKLINKS: drawers. Safe to run repeatedly (idempotent).
    Normalise {
        /// Directory of .org files to normalise (default: current directory)
        #[clap(default_value = ".")]
        dir: PathBuf,
        /// Print what would change without writing any files
        #[clap(long)]
        dry_run: bool,
    },

    /// Regenerate the blog index and tag pages (without a full build)
    Blog {
        /// Directory containing blog posts (default: current directory)
        #[clap(default_value = ".")]
        dir: PathBuf,
    },
}

// ============================================================
//  Dispatch
// ============================================================

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {

        // ── todo ────────────────────────────────────────────

        Commands::Todo { sub } => match sub {
            TodoCommands::List { path }             => list_todos(&path)?,
            TodoCommands::Add { text, file, tag }   => add_todo(&text, &file, tag.as_deref())?,
            TodoCommands::Done { file, line }        => mark_done(&file, line)?,
            TodoCommands::Cancel { file, line }      => mark_cancelled(&file, line)?,
            TodoCommands::Wait { file, line, date }  => mark_waiting(&file, line, date.as_deref())?,
            TodoCommands::Next { file, line }        => mark_next(&file, line)?,
            TodoCommands::Set { file, line, keyword } => set_keyword(&file, line, &keyword)?,
        },

        // ── schedule ────────────────────────────────────────

        Commands::Schedule { sub } => match sub {
            ScheduleCommands::Set { file, line, date }      => reschedule(&file, line, &date)?,
            ScheduleCommands::Deadline { file, line, date } => set_deadline(&file, line, &date)?,
            ScheduleCommands::Clear { file, line }          => clear_schedule(&file, line)?,
        },

        // ── agenda ──────────────────────────────────────────

        Commands::Agenda { sub } => match sub {
            AgendaCommands::Day { date, path }          => agenda_day(date.as_deref(), &path)?,
            AgendaCommands::Week { date, path }         => agenda_week(date.as_deref(), &path)?,
            AgendaCommands::Deadlines { path, days }    => agenda_deadlines(&path, days)?,
        },

        // ── structure ───────────────────────────────────────

        Commands::Structure { sub } => match sub {
            StructureCommands::Show { file } => show_file(&file)?,
        },

        // ── export ──────────────────────────────────────────

        Commands::Export { sub } => match sub {
            ExportCommands::Html { path, output } => {
                html::export_site(&path, &output)?;
                println!("Exported to {}", output.display());
            }
        },

        // ── site ────────────────────────────────────────────

        Commands::Site { sub } => match sub {
            SiteCommands::Build { dir, config: config_path, force } => {
                build::run_build(&dir, config_path.as_deref(), force)?;
            }
            SiteCommands::Normalise { dir, dry_run } => {
                run_normalise(&dir, dry_run)?;
            }
            SiteCommands::Blog { dir } => {
                run_blog(&dir)?;
            }
        },

        // ── deprecated flat aliases ──────────────────────────

        Commands::List { path } => {
            eprintln!("note: `list` is deprecated — use `org-cli todo list`");
            list_todos(&path)?;
        }
        Commands::Add { text, file, tag } => {
            eprintln!("note: `add` is deprecated — use `org-cli todo add`");
            add_todo(&text, &file, tag.as_deref())?;
        }
        Commands::Done { file, line } => {
            eprintln!("note: `done` is deprecated — use `org-cli todo done`");
            mark_done(&file, line)?;
        }
        Commands::Cancel { file, line } => {
            eprintln!("note: `cancel` is deprecated — use `org-cli todo cancel`");
            mark_cancelled(&file, line)?;
        }
        Commands::Wait { file, line, date } => {
            eprintln!("note: `wait` is deprecated — use `org-cli todo wait`");
            mark_waiting(&file, line, date.as_deref())?;
        }
        Commands::Reschedule { file, line, date } => {
            eprintln!("note: `reschedule` is deprecated — use `org-cli schedule set`");
            reschedule(&file, line, &date)?;
        }
        Commands::Show { file } => {
            eprintln!("note: `show` is deprecated — use `org-cli structure show`");
            show_file(&file)?;
        }
        Commands::ExportFlat { path, output } => {
            eprintln!("note: `export` (flat) is deprecated — use `org-cli export html`");
            html::export_site(&path, &output)?;
            println!("Exported to {}", output.display());
        }
        Commands::Normalise { dir, dry_run } => {
            eprintln!("note: `normalise` is deprecated — use `org-cli site normalise`");
            run_normalise(&dir, dry_run)?;
        }
        Commands::Build { dir, config: config_path, force } => {
            eprintln!("note: `build` is deprecated — use `org-cli site build`");
            build::run_build(&dir, config_path.as_deref(), force)?;
        }
    }

    Ok(())
}

// ============================================================
//  Shared helpers for commands that need local output logic
// ============================================================

fn run_normalise(dir: &std::path::Path, dry_run: bool) -> Result<()> {
    let modified = normalise::normalise_dir(dir, dry_run)?;
    if modified.is_empty() {
        println!("Nothing to normalise.");
    } else {
        for p in &modified {
            println!(
                "{} {}",
                if dry_run { "would modify" } else { "modified" },
                p.display()
            );
        }
        println!(
            "{} file(s) {}.",
            modified.len(),
            if dry_run { "would be modified" } else { "modified" }
        );
    }
    Ok(())
}

fn run_blog(dir: &std::path::Path) -> Result<()> {
    let cfg = config::Config::load(dir)?;
    let posts = blog::build_blog(dir, &cfg.blog)?;
    println!("Blog: {} posts indexed.", posts.len());
    Ok(())
}
