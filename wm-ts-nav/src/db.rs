use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::parser::{IdentRef, Symbol};

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir)
            .with_context(|| format!("creating cache dir: {}", cache_dir.display()))?;
        let db_path = cache_dir.join("index.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("opening db: {}", db_path.display()))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS files (
                 id INTEGER PRIMARY KEY,
                 path TEXT NOT NULL UNIQUE,
                 mtime_secs INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS symbols (
                 id INTEGER PRIMARY KEY,
                 file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                 name TEXT NOT NULL,
                 kind TEXT NOT NULL,
                 line INTEGER NOT NULL,
                 end_line INTEGER NOT NULL,
                 signature TEXT,
                 parent TEXT
             );
             CREATE TABLE IF NOT EXISTS refs (
                 file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                 name TEXT NOT NULL,
                 line INTEGER NOT NULL,
                 import_path TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
             CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_id);
             CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind);
             CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
             CREATE INDEX IF NOT EXISTS idx_refs_name ON refs(name);
             CREATE INDEX IF NOT EXISTS idx_refs_file ON refs(file_id);",
        )?;

        Ok(Self { conn })
    }

    pub fn begin(&self) -> Result<()> {
        self.conn.execute_batch("BEGIN")?;
        Ok(())
    }

    pub fn commit(&self) -> Result<()> {
        self.conn.execute_batch("COMMIT")?;
        Ok(())
    }

    pub fn upsert_file(
        &self,
        path: &str,
        mtime_secs: i64,
        symbols: &[Symbol],
        refs: &[IdentRef],
    ) -> Result<()> {
        // Delete old entry if exists
        self.conn.execute(
            "DELETE FROM refs WHERE file_id IN (SELECT id FROM files WHERE path = ?1)",
            params![path],
        )?;
        self.conn.execute(
            "DELETE FROM symbols WHERE file_id IN (SELECT id FROM files WHERE path = ?1)",
            params![path],
        )?;
        self.conn
            .execute("DELETE FROM files WHERE path = ?1", params![path])?;

        // Insert new file
        self.conn.execute(
            "INSERT INTO files (path, mtime_secs) VALUES (?1, ?2)",
            params![path, mtime_secs],
        )?;
        let file_id = self.conn.last_insert_rowid();

        // Insert symbols
        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO symbols (file_id, name, kind, line, end_line, signature, parent) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for sym in symbols {
            stmt.execute(params![
                file_id,
                sym.name,
                sym.kind,
                sym.line,
                sym.end_line,
                sym.signature,
                sym.parent,
            ])?;
        }

        // Insert refs
        let mut ref_stmt = self.conn.prepare_cached(
            "INSERT INTO refs (file_id, name, line, import_path) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for r in refs {
            ref_stmt.execute(params![file_id, r.name, r.line, r.import_path])?;
        }

        Ok(())
    }

    pub fn remove_file(&self, path: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM refs WHERE file_id IN (SELECT id FROM files WHERE path = ?1)",
            params![path],
        )?;
        self.conn.execute(
            "DELETE FROM symbols WHERE file_id IN (SELECT id FROM files WHERE path = ?1)",
            params![path],
        )?;
        self.conn
            .execute("DELETE FROM files WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn all_indexed_paths(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, mtime_secs FROM files")?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn search_symbols(
        &self,
        pattern: &str,
        kind_filter: Option<&str>,
        parent_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut conditions = vec!["s.name LIKE ?1".to_string()];
        if let Some(kind) = kind_filter {
            conditions.push(format!("s.kind = '{kind}'"));
        }
        if let Some(parent) = parent_filter {
            conditions.push(format!("s.parent LIKE '%{parent}%'"));
        }
        let where_clause = conditions.join(" AND ");
        let query = format!(
            "SELECT s.name, s.kind, s.line, s.end_line, s.signature, s.parent, f.path
             FROM symbols s JOIN files f ON s.file_id = f.id
             WHERE {where_clause}
             ORDER BY s.name LIMIT ?2"
        );

        let like_pattern = if pattern.contains('%') || pattern.contains('_') {
            pattern.to_string()
        } else {
            format!("%{pattern}%")
        };

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt
            .query_map(params![like_pattern, limit as i64], |row| {
                Ok(SearchResult {
                    name: row.get(0)?,
                    kind: row.get(1)?,
                    line: row.get(2)?,
                    end_line: row.get(3)?,
                    signature: row.get(4)?,
                    parent: row.get(5)?,
                    path: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn file_symbols(&self, path: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.name, s.kind, s.line, s.end_line, s.signature, s.parent, f.path
             FROM symbols s JOIN files f ON s.file_id = f.id
             WHERE f.path = ?1
             ORDER BY s.line",
        )?;
        let rows = stmt
            .query_map(params![path], |row| {
                Ok(SearchResult {
                    name: row.get(0)?,
                    kind: row.get(1)?,
                    line: row.get(2)?,
                    end_line: row.get(3)?,
                    signature: row.get(4)?,
                    parent: row.get(5)?,
                    path: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn find_refs(&self, name: &str, limit: usize) -> Result<Vec<RefResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.path, r.line, r.import_path
             FROM refs r JOIN files f ON r.file_id = f.id
             WHERE r.name = ?1
             ORDER BY f.path, r.line
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![name, limit as i64], |row| {
                Ok(RefResult {
                    path: row.get(0)?,
                    line: row.get(1)?,
                    import_path: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct RefResult {
    pub path: String,
    pub line: i64,
    pub import_path: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SearchResult {
    pub name: String,
    pub kind: String,
    pub line: i64,
    pub end_line: i64,
    pub signature: Option<String>,
    pub parent: Option<String>,
    pub path: String,
}

pub fn mtime_secs(path: &Path) -> Result<i64> {
    let meta = std::fs::metadata(path)?;
    let mtime = meta
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    Ok(mtime.as_secs() as i64)
}

pub fn cache_dir_for(root: &Path) -> PathBuf {
    let hash = {
        let s = root.to_string_lossy();
        let mut h: u64 = 5381;
        for b in s.bytes() {
            h = h.wrapping_mul(33).wrapping_add(b as u64);
        }
        h
    };
    dirs_cache().join(format!("{hash:x}"))
}

fn dirs_cache() -> PathBuf {
    if let Ok(d) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(d).join("wm-ts-nav")
    } else if let Ok(d) = std::env::var("HOME") {
        PathBuf::from(d).join(".cache").join("wm-ts-nav")
    } else {
        PathBuf::from("/tmp/wm-ts-nav")
    }
}
