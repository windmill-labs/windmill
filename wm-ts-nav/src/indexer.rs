use anyhow::Result;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::db::{self, Db};
use crate::parser::{self, Lang};

pub struct IndexStats {
    pub files_scanned: usize,
    pub files_updated: usize,
    pub files_removed: usize,
    pub files_unchanged: usize,
}

/// Find gitignored *_ee files (EE symlinks from windmill-ee-private).
fn find_ee_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_str().unwrap_or("");
        // path.is_dir() follows symlinks
        if path.is_dir() {
            if !matches!(name_str, "target" | "node_modules" | ".git") {
                find_ee_files(&path, out);
            }
        } else if Lang::from_path(&path).is_some()
            && (name_str.ends_with("_ee.rs")
                || name_str.ends_with("_ee.ts")
                || name_str.ends_with("_ee.svelte"))
            && path.exists() // follows symlink, checks target exists
        {
            out.push(path);
        }
    }
}

/// Incrementally update the index for the given root directory.
/// Only re-parses files whose mtime has changed since last index.
pub fn update_index(db: &Db, root: &Path) -> Result<IndexStats> {
    // Collect all supported files using `ignore` crate (respects .gitignore)
    let mut files: Vec<_> = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .follow_links(true)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|e| Lang::from_path(e.path()).is_some())
        .map(|e| e.into_path())
        .collect();

    // Also include gitignored *_ee files (EE symlinks from windmill-ee-private)
    let file_set: HashSet<_> = files.iter().cloned().collect();
    let mut ee_files = Vec::new();
    find_ee_files(root, &mut ee_files);
    for f in ee_files {
        if !file_set.contains(&f) {
            files.push(f);
        }
    }

    let disk_paths: HashSet<String> = files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    // Check which files need updating
    let existing = db.all_indexed_paths()?;
    // Remove files no longer on disk
    let mut files_removed = 0;
    db.begin()?;
    for (path, _) in &existing {
        if !disk_paths.contains(path) {
            db.remove_file(path)?;
            files_removed += 1;
        }
    }
    db.commit()?;

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
                Ok(result) => Some((path.to_string_lossy().to_string(), mtime, result)),
                Err(e) => {
                    eprintln!("warning: failed to parse {}: {e}", path.display());
                    None
                }
            }
        })
        .collect();

    let files_updated = results.len();

    // Write to db in a single transaction
    db.begin()?;
    for (path, mtime, result) in &results {
        db.upsert_file(path, *mtime, &result.symbols, &result.refs)?;
    }
    db.commit()?;

    Ok(IndexStats {
        files_scanned: files.len(),
        files_updated,
        files_removed,
        files_unchanged,
    })
}
