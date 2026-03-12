use anyhow::Result;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::Path;

use crate::db::{self, Db};
use crate::parser::{self, Lang};

pub struct IndexStats {
    pub files_scanned: usize,
    pub files_updated: usize,
    pub files_removed: usize,
    pub files_unchanged: usize,
}

/// Incrementally update the index for the given root directory.
/// Only re-parses files whose mtime has changed since last index.
pub fn update_index(db: &Db, root: &Path) -> Result<IndexStats> {
    // Collect all supported files using `ignore` crate (respects .gitignore)
    let files: Vec<_> = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|e| Lang::from_path(e.path()).is_some())
        .map(|e| e.into_path())
        .collect();

    let disk_paths: HashSet<String> = files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    // Check which files need updating
    let existing = db.all_indexed_paths()?;
    // Remove files no longer on disk
    let mut files_removed = 0;
    for (path, _) in &existing {
        if !disk_paths.contains(path) {
            db.remove_file(path)?;
            files_removed += 1;
        }
    }

    // Figure out which files need re-parsing
    let existing_map: std::collections::HashMap<&str, i64> = existing
        .iter()
        .map(|(p, m)| (p.as_str(), *m))
        .collect();

    let to_parse: Vec<_> = files
        .iter()
        .filter(|path| {
            let path_str = path.to_string_lossy();
            match existing_map.get(path_str.as_ref()) {
                Some(&old_mtime) => {
                    // Check if mtime changed
                    db::mtime_secs(path).unwrap_or(0) != old_mtime
                }
                None => true, // New file
            }
        })
        .collect();

    let files_unchanged = files.len() - to_parse.len();

    // Parse files in parallel
    let results: Vec<_> = to_parse
        .par_iter()
        .filter_map(|path| {
            let mtime = db::mtime_secs(path).ok()?;
            match parser::parse_file(path) {
                Ok(symbols) => Some((path.to_string_lossy().to_string(), mtime, symbols)),
                Err(e) => {
                    eprintln!("warning: failed to parse {}: {e}", path.display());
                    None
                }
            }
        })
        .collect();

    let files_updated = results.len();

    // Write to db (single-threaded, SQLite doesn't like concurrent writes)
    for (path, mtime, symbols) in &results {
        db.upsert_file(path, *mtime, symbols)?;
    }

    Ok(IndexStats {
        files_scanned: files.len(),
        files_updated,
        files_removed,
        files_unchanged,
    })
}
