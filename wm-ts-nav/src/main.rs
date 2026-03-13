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
        /// Filter by parent (e.g. --parent ServiceName to find methods on that type)
        #[arg(short, long)]
        parent: Option<String>,
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
    /// Find references to a symbol in code (skips comments and strings)
    Refs {
        /// Symbol name to find
        name: String,
        /// Max results
        #[arg(short, long, default_value = "50")]
        limit: usize,
        /// Filter to files matching this substring
        #[arg(short, long)]
        file: Option<String>,
        /// Show which function/symbol contains each reference
        #[arg(short, long)]
        caller: bool,
    },
    /// Extract and print a symbol's source code
    Body {
        /// Exact symbol name
        name: String,
        /// Filter by kind
        #[arg(short, long)]
        kind: Option<String>,
        /// Filter to files matching this substring
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Find what calls a symbol (who calls X?)
    Callers {
        /// Symbol name to find callers of
        name: String,
        /// Max results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Find what a symbol calls (what does X call?)
    Callees {
        /// Exact symbol name
        name: String,
        /// Filter by kind
        #[arg(short, long)]
        kind: Option<String>,
        /// Filter to files matching this substring
        #[arg(short, long)]
        file: Option<String>,
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
    let stats = indexer::update_index(&db, &root)?;

    match cli.command {
        Command::Index => {
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
                println!("L{}-{} {:12} {}{}{}", s.line, s.end_line, s.kind, s.name, parent, sig);
            }
        }
        Command::Search {
            pattern,
            kind,
            parent,
            limit,
        } => {
            let results = db.search_symbols(&pattern, kind.as_deref(), parent.as_deref(), limit)?;
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
                let parent_info = r
                    .parent
                    .as_deref()
                    .map(|p| format!(" [{p}]"))
                    .unwrap_or_default();
                println!("{}:{} {:12} {}{}{}", r.path, r.line, r.kind, r.name, parent_info, sig);
            }
        }
        Command::Def { name, kind } => {
            let results = db.search_symbols(&name, kind.as_deref(), None, 100)?;
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
        Command::Refs {
            name,
            limit,
            file,
            caller,
        } => {
            let results = db.find_refs(&name, limit, file.as_deref(), caller)?;
            if results.is_empty() {
                println!("No references found for '{name}'");
                return Ok(());
            }
            for r in &results {
                let origin = r
                    .import_path
                    .as_deref()
                    .map(|p| format!("  ({p})"))
                    .unwrap_or_default();
                let caller_info = r
                    .caller_name
                    .as_deref()
                    .map(|c| format!("  [{c}]"))
                    .unwrap_or_default();
                println!("{}:{}{}{}", r.path, r.line, caller_info, origin);
            }
        }
        Command::Body { name, kind, file } => {
            let results = db.search_symbols(&name, kind.as_deref(), None, 100)?;
            let mut exact: Vec<_> = results.into_iter().filter(|r| r.name == name).collect();
            if let Some(ref f) = file {
                exact.retain(|r| r.path.contains(f.as_str()));
            }
            if exact.is_empty() {
                println!("No definition found for '{name}'");
                return Ok(());
            }
            for (i, r) in exact.iter().enumerate() {
                if i > 0 {
                    println!("\n---\n");
                }
                println!("{}:L{}-{}", r.path, r.line, r.end_line);
                match std::fs::read_to_string(&r.path) {
                    Ok(contents) => {
                        let lines: Vec<&str> = contents.lines().collect();
                        let start = (r.line as usize).saturating_sub(1);
                        let end = (r.end_line as usize).min(lines.len());
                        for line in &lines[start..end] {
                            println!("{line}");
                        }
                    }
                    Err(e) => println!("  (error reading file: {e})"),
                }
            }
        }
        Command::Callers { name, limit } => {
            let results = db.find_callers(&name, limit)?;
            if results.is_empty() {
                println!("No callers found for '{name}'");
                return Ok(());
            }
            for r in &results {
                println!(
                    "{}:L{}-{} {} {} → L{}",
                    r.path, r.caller_line, r.caller_end_line, r.caller_kind, r.caller_name, r.ref_line
                );
            }
        }
        Command::Callees { name, kind, file } => {
            let results = db.find_callees(&name, kind.as_deref(), file.as_deref())?;
            if results.is_empty() {
                println!("No callees found for '{name}'");
                return Ok(());
            }
            for r in &results {
                let origin = r
                    .import_path
                    .as_deref()
                    .map(|p| format!("  ({p})"))
                    .unwrap_or_default();
                println!("{}{}", r.name, origin);
            }
        }
    }

    Ok(())
}
