mod db;
mod indexer;
mod parser;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "wm-ts-nav", about = "Tree-sitter code navigator for Windmill")]
struct Cli {
    /// Root directory to index (defaults to current directory)
    #[arg(short, long)]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Index/re-index the codebase
    Index,
    /// Show symbols in a file
    Outline {
        /// File path
        file: PathBuf,
    },
    /// Search symbols by name pattern
    Search {
        /// Name pattern (supports SQL LIKE % wildcards)
        pattern: String,
        /// Filter by kind (function, struct, enum, trait, impl, etc.)
        #[arg(short, long)]
        kind: Option<String>,
        /// Max results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Find symbol definition by exact name
    Def {
        /// Exact symbol name
        name: String,
        /// Filter by kind
        #[arg(short, long)]
        kind: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli
        .root
        .unwrap_or_else(|| std::env::current_dir().expect("no cwd"));
    let root = std::fs::canonicalize(&root)?;
    let cache_dir = db::cache_dir_for(&root);
    let db = db::Db::open(&cache_dir)?;

    // Always update index incrementally before any query
    indexer::update_index(&db, &root)?;

    match cli.command {
        Command::Index => {
            let stats = indexer::update_index(&db, &root)?;
            println!(
                "Indexed {} files: {} updated, {} unchanged, {} removed",
                stats.files_scanned, stats.files_updated, stats.files_unchanged, stats.files_removed
            );
        }
        Command::Outline { file } => {
            let file = std::fs::canonicalize(&file)?;
            let symbols = db.file_symbols(&file.to_string_lossy())?;
            if symbols.is_empty() {
                println!("No symbols found");
                return Ok(());
            }
            for s in &symbols {
                let parent = s
                    .parent
                    .as_deref()
                    .map(|p| format!(" [{p}]"))
                    .unwrap_or_default();
                let sig = s
                    .signature
                    .as_deref()
                    .map(|s| format!("  {s}"))
                    .unwrap_or_default();
                println!("L{:<5} {:12} {}{}{}", s.line, s.kind, s.name, parent, sig);
            }
        }
        Command::Search {
            pattern,
            kind,
            limit,
        } => {
            let results = db.search_symbols(&pattern, kind.as_deref(), limit)?;
            if results.is_empty() {
                println!("No symbols matching '{pattern}'");
                return Ok(());
            }
            for r in &results {
                let sig = r
                    .signature
                    .as_deref()
                    .map(|s| format!("  {s}"))
                    .unwrap_or_default();
                println!("{}:{} {:12} {}{}", r.path, r.line, r.kind, r.name, sig);
            }
        }
        Command::Def { name, kind } => {
            let results = db.search_symbols(&name, kind.as_deref(), 100)?;
            let exact: Vec<_> = results.iter().filter(|r| r.name == name).collect();
            if exact.is_empty() {
                println!("No definition found for '{name}'");
                return Ok(());
            }
            for r in &exact {
                let sig = r
                    .signature
                    .as_deref()
                    .map(|s| format!("\n  {s}"))
                    .unwrap_or_default();
                let parent = r
                    .parent
                    .as_deref()
                    .map(|p| format!(" [{p}]"))
                    .unwrap_or_default();
                println!(
                    "{}:L{}-{} {} {}{}{}",
                    r.path, r.line, r.end_line, r.kind, r.name, parent, sig
                );
            }
        }
    }

    Ok(())
}
