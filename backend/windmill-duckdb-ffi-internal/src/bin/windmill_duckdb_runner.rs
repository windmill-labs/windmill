//! Runs one DuckDB job out of the worker's process, so the worker can put the engine inside
//! the same jail as the other languages' jobs.
//!
//! The worker writes one JSON [`Request`] to stdin and reads the answer from
//! `Request::result_path`: a first line holding the column order (empty when there is none),
//! then exactly the string the in-process `run_duckdb_ffi` / `prepare_duckdb_ffi` would have
//! returned, `ERROR <json>` included. Going through those same `extern "C"` entry points keeps
//! the two paths one implementation: argument binding, result-size guards and error text
//! cannot drift apart.
//!
//! stdout/stderr are not protocol: the worker keeps only their tail, and reports it when the
//! runner dies without writing a result.

use std::{
    ffi::{c_char, CStr, CString},
    io::{Read, Write},
    ptr::null_mut,
    sync::atomic::Ordering,
};

use serde::Deserialize;
use windmill_duckdb_ffi_internal::{
    free_cstr, prepare_duckdb_ffi, run_duckdb_ffi, LOCK_TEMP_DIRECTORY,
};

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
    job_args: Option<Box<serde_json::value::RawValue>>,
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
    #[serde(default = "default_true")]
    lock_temp_directory: bool,
}

fn default_true() -> bool {
    true
}

fn cstring(s: &str) -> Result<CString, String> {
    CString::new(s).map_err(|e| format!("Invalid string in runner request: {e}"))
}

fn take_cstr(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    free_cstr(ptr);
    Some(s)
}

fn install(blocks: &[String]) -> String {
    let run = || -> Result<(), String> {
        let conn = duckdb::Connection::open_in_memory().map_err(|e| e.to_string())?;
        for b in blocks {
            conn.execute_batch(b).map_err(|e| e.to_string())?;
        }
        Ok(())
    };
    match run() {
        Ok(()) => "[]".to_string(),
        Err(e) => format!("ERROR {}", serde_json::to_string(&e).unwrap_or_default()),
    }
}

fn execute(req: &Request) -> Result<(String, Option<String>), String> {
    LOCK_TEMP_DIRECTORY.store(req.lock_temp_directory, Ordering::Relaxed);

    let blocks = req
        .blocks
        .iter()
        .map(|b| cstring(b))
        .collect::<Result<Vec<_>, _>>()?;
    let block_ptrs = blocks.iter().map(|b| b.as_ptr()).collect::<Vec<_>>();
    let token = cstring(&req.token)?;
    let base_internal_url = cstring(&req.base_internal_url)?;
    let w_id = cstring(&req.w_id)?;
    let memory_limit = cstring(&req.memory_limit)?;
    let temp_directory = cstring(&req.temp_directory)?;

    match req.mode {
        Mode::Install => Ok((install(&req.blocks), None)),
        Mode::Prepare => {
            let ptr = prepare_duckdb_ffi(
                block_ptrs.as_ptr(),
                block_ptrs.len(),
                token.as_ptr(),
                base_internal_url.as_ptr(),
                w_id.as_ptr(),
                memory_limit.as_ptr(),
                temp_directory.as_ptr(),
            );
            Ok((take_cstr(ptr).unwrap_or_default(), None))
        }
        Mode::Run => {
            let job_args = cstring(req.job_args.as_ref().map(|a| a.get()).unwrap_or("[]"))?;
            let mut column_order: *mut c_char = null_mut();
            let ptr = run_duckdb_ffi(
                block_ptrs.as_ptr(),
                block_ptrs.len(),
                job_args.as_ptr(),
                token.as_ptr(),
                base_internal_url.as_ptr(),
                w_id.as_ptr(),
                memory_limit.as_ptr(),
                temp_directory.as_ptr(),
                req.max_result_size,
                &mut column_order,
                req.collect_last_only,
                req.collect_first_row_only,
            );
            Ok((take_cstr(ptr).unwrap_or_default(), take_cstr(column_order)))
        }
    }
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

    let (result, column_order) = execute(&req).unwrap_or_else(|e| {
        (
            format!("ERROR {}", serde_json::to_string(&e).unwrap_or_default()),
            None,
        )
    });
    let write = || -> std::io::Result<()> {
        let mut f = std::io::BufWriter::new(std::fs::File::create(&req.result_path)?);
        f.write_all(column_order.as_deref().unwrap_or("").as_bytes())?;
        f.write_all(b"\n")?;
        f.write_all(result.as_bytes())?;
        f.flush()
    };
    if let Err(e) = write() {
        eprintln!(
            "windmill_duckdb_runner: could not write the result to {}: {e}",
            req.result_path
        );
        std::process::exit(2);
    }
}
