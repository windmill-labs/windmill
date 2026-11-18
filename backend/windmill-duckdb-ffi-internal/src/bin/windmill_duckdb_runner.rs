//! Runs one DuckDB job, so the engine lives in a process of its own under the worker's job
//! isolation rather than in the worker.
//!
//! The worker writes one JSON [`Request`] to stdin and reads the answer from
//! `Request::result_path`: a first line holding the column order as JSON (empty when there is
//! none), then the collected results, or `ERROR <json-encoded message>`.
//!
//! stdout/stderr are not protocol: the worker keeps only their tail, and reports it when the
//! runner dies without writing a result.

use std::io::{Read, Write};

use serde::Deserialize;
use windmill_duckdb_ffi_internal::{prepare, run, Arg, Connection};

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Run,
    Prepare,
    /// `INSTALL` only, into the extension cache the jail later mounts read-only. Never loads
    /// anything, so it runs no extension code.
    Install,
}

#[derive(Deserialize)]
struct Request {
    mode: Mode,
    blocks: Vec<String>,
    #[serde(default)]
    job_args: Vec<Arg>,
    #[serde(default)]
    token: String,
    #[serde(default)]
    base_internal_url: String,
    #[serde(default)]
    w_id: String,
    #[serde(default)]
    memory_limit: String,
    #[serde(default)]
    temp_directory: String,
    #[serde(default)]
    max_result_size: usize,
    #[serde(default)]
    collect_last_only: bool,
    #[serde(default)]
    collect_first_row_only: bool,
    result_path: String,
}

fn non_empty(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

fn install(blocks: &[String]) -> Result<String, String> {
    let conn = duckdb::Connection::open_in_memory().map_err(|e| e.to_string())?;
    for b in blocks {
        conn.execute_batch(b).map_err(|e| e.to_string())?;
    }
    Ok("[]".to_string())
}

/// The result string and the column order as JSON.
fn execute(req: Request) -> (String, Option<String>) {
    let conn = Connection {
        token: &req.token,
        base_internal_url: &req.base_internal_url,
        w_id: &req.w_id,
        memory_limit: non_empty(&req.memory_limit),
        temp_directory: non_empty(&req.temp_directory),
    };
    let outcome = match req.mode {
        Mode::Install => install(&req.blocks).map(|r| (r, None)),
        Mode::Prepare => prepare(&req.blocks, conn).map(|r| (r, None)),
        Mode::Run => run(
            &req.blocks,
            req.job_args,
            conn,
            req.max_result_size,
            req.collect_last_only,
            req.collect_first_row_only,
        )
        .map(|(r, order)| {
            let order = order.map(|o| serde_json::to_string(&o).unwrap_or_else(|_| "[]".into()));
            (r, order)
        }),
    };
    outcome.unwrap_or_else(|e| {
        let msg = serde_json::to_string(&e).unwrap_or_else(|_| "\"Unknown DuckDB error\"".into());
        (format!("ERROR {msg}"), None)
    })
}

fn main() {
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("windmill_duckdb_runner: could not read the request: {e}");
        std::process::exit(2);
    }
    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("windmill_duckdb_runner: invalid request: {e}");
            std::process::exit(2);
        }
    };
    drop(input);

    let result_path = req.result_path.clone();
    let (result, column_order) = execute(req);
    let write = || -> std::io::Result<()> {
        let mut f = std::io::BufWriter::new(std::fs::File::create(&result_path)?);
        f.write_all(column_order.as_deref().unwrap_or("").as_bytes())?;
        f.write_all(b"\n")?;
        f.write_all(result.as_bytes())?;
        f.flush()
    };
    if let Err(e) = write() {
        eprintln!("windmill_duckdb_runner: could not write the result to {result_path}: {e}");
        std::process::exit(2);
    }
}
