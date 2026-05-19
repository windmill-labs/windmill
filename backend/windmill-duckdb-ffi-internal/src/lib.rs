use std::{
    collections::HashMap,
    ffi::{c_char, c_uint, CStr, CString},
    ptr::null_mut,
    sync::LazyLock,
};

use duckdb::{core::LogicalTypeId, params_from_iter, types::TimeUnit, Row};
use regex::Regex;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Arg {
    pub name: String,
    pub arg_type: String,
    pub json_value: serde_json::Value,
}

// Freeing from the caller side crashes the runtime with jemalloc enabled (EXIT CODE 11 SEGFAULT)
#[unsafe(no_mangle)]
pub extern "C" fn free_cstr(string: *mut c_char) -> () {
    if string.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(string);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_version() -> c_uint {
    // Increment when making breaking changes to the FFI interface.
    // The windmill worker will check that the version matches or else refuse to call
    // the FFI functions to avoid undefined behavior.
    return 1;
}

#[unsafe(no_mangle)]
pub extern "C" fn run_duckdb_ffi(
    query_block_list: *const *const c_char,
    query_block_list_count: usize,
    job_args: *const c_char,
    token: *const c_char,
    base_internal_url: *const c_char,
    w_id: *const c_char,
    column_order_ptr: *mut *mut c_char,
    collect_last_only: bool,
    collect_first_row_only: bool,
) -> *mut c_char {
    let (r, column_order) = match convert_args(
        query_block_list,
        query_block_list_count,
        job_args,
        token,
        base_internal_url,
        w_id,
    )
    .and_then(
        |(query_block_list, job_args, token, base_internal_url, w_id)| {
            run_duckdb_internal(
                query_block_list,
                query_block_list_count,
                job_args,
                token,
                base_internal_url,
                w_id,
                collect_last_only,
                collect_first_row_only,
            )
        },
    ) {
        Ok(result) => result,
        Err(err) => {
            let err = serde_json::to_string(&err)
                .unwrap_or_else(|_| "Unknown error in duckdb ffi lib".to_string());
            (format!("ERROR {}", err), None)
        }
    };

    unsafe {
        if let Some(column_order) = column_order {
            let column_order =
                serde_json::to_string(&column_order).unwrap_or_else(|_| "[]".to_string());
            let c_column_order =
                CString::new(column_order).unwrap_or_else(|_| CString::new("[]").unwrap());
            *column_order_ptr = c_column_order.into_raw();
        } else {
            *column_order_ptr = null_mut();
        }
    }
    // CString::into_raw because it needs to outlive this function call.
    // The caller is responsible for freeing the memory through CString::from_raw.
    CString::new(r).map(|s| s.into_raw()).unwrap_or_else(|e| {
        println!("Failed to allocate error string in duckdb ffi lib: {:?}", e);
        null_mut()
    })
}

#[derive(Serialize, Debug)]
struct PrepareQueryColumnInfo {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
}

#[derive(Serialize, Debug)]
struct PrepareQueryResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    columns: Option<Vec<PrepareQueryColumnInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn is_setup_statement(query: &str) -> bool {
    let trimmed = query.trim_start();
    let upper = trimmed.to_uppercase();
    upper.starts_with("ATTACH")
        || upper.starts_with("USE")
        || upper.starts_with("INSTALL")
        || upper.starts_with("LOAD")
        || upper.starts_with("SET")
        || upper.starts_with("RESET")
        || upper.starts_with("CREATE OR REPLACE SECRET")
        || upper.starts_with("CREATE SECRET")
}

/// Returns true if the query is expected to return a result set and can be wrapped with DESCRIBE.
fn is_describable_query(query: &str) -> bool {
    let trimmed = query.trim_start();
    let upper = trimmed.to_uppercase();
    upper.starts_with("SELECT")
        || upper.starts_with("WITH")
        || upper.starts_with("VALUES")
        || upper.starts_with("TABLE")
        || upper.starts_with("FROM")
}

static PARAM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\d+").expect("invalid regex"));

fn replace_params_with_null(query: &str) -> String {
    PARAM_RE.replace_all(query, "NULL").to_string()
}

#[unsafe(no_mangle)]
pub extern "C" fn prepare_duckdb_ffi(
    query_block_list: *const *const c_char,
    query_block_list_count: usize,
    token: *const c_char,
    base_internal_url: *const c_char,
    w_id: *const c_char,
) -> *mut c_char {
    let r = match convert_prepare_args(
        query_block_list,
        query_block_list_count,
        token,
        base_internal_url,
        w_id,
    )
    .and_then(|(query_block_list, token, base_internal_url, w_id)| {
        prepare_duckdb_internal(query_block_list, token, base_internal_url, w_id)
    }) {
        Ok(result) => result,
        Err(err) => {
            let err = serde_json::to_string(&err)
                .unwrap_or_else(|_| "Unknown error in duckdb ffi lib".to_string());
            format!("ERROR {}", err)
        }
    };

    CString::new(r).map(|s| s.into_raw()).unwrap_or_else(|e| {
        println!("Failed to allocate error string in duckdb ffi lib: {:?}", e);
        null_mut()
    })
}

/// Above this, the cgroup memory limit is effectively unlimited: kernels report
/// page-aligned huge numbers (cgroup v1) or the literal "max" (cgroup v2) when
/// no limit is enforced. Same 1 PiB guard as
/// `windmill-common::worker::get_memory`.
///
/// Note: unlike `get_memory`, when no cgroup limit is enforced we deliberately
/// do NOT fall back to `/proc/meminfo`. DuckDB's own default already caps at a
/// fraction of detected physical RAM, which is the correct behavior on an
/// uncapped host — the bug this targets is specifically a cgroup cap smaller
/// than host RAM, which DuckDB is not aware of.
const CGROUP_UNLIMITED_THRESHOLD: i64 = 1024 * 1024 * 1024 * 1024 * 1024;

/// Fraction of the detected cgroup memory limit DuckDB may use. Set high on
/// purpose: a job should be free to use most of the container's memory. The
/// ~20% headroom is what makes the failure mode correct — DuckDB is in-process
/// in the worker, so the kernel OOM killer cannot kill "the job", only the
/// whole worker. The headroom (Rust runtime baseline + DuckDB's untracked
/// allocations) lets DuckDB hit its own `memory_limit` and abort *that query*
/// with a graceful error *before* the process total reaches the cgroup cap and
/// the kernel SIGKILLs the entire worker. Mirrors DuckDB's own default ratio
/// (80% of detected RAM), but cgroup-aware. Overridable via
/// `DUCKDB_MEMORY_LIMIT`.
const DEFAULT_CGROUP_MEMORY_FRACTION: f64 = 0.8;

// cgroup v2: `0::<path>` line in /proc/self/cgroup. Same regex as
// windmill-common::worker::CGROUP_V2_PATH_RE.
static CGROUP_V2_PATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?m)^0::(/.*)$"#).expect("invalid regex"));

/// Parse a raw cgroup `memory.max` / `memory.limit_in_bytes` value into a real
/// byte limit, or `None` when there is effectively no enforced limit.
fn parse_cgroup_memory_value(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    if raw == "max" {
        return None;
    }
    let bytes = raw.parse::<i64>().ok()?;
    if bytes <= 0 || bytes >= CGROUP_UNLIMITED_THRESHOLD {
        None
    } else {
        Some(bytes)
    }
}

/// Value to pass to `SET memory_limit` given a real cgroup limit in bytes.
/// Floored at 64 MiB so a tiny cgroup never yields an unusable 0-byte limit.
fn duckdb_memory_limit_for_cgroup(cgroup_bytes: i64) -> String {
    let mib = ((cgroup_bytes as f64 * DEFAULT_CGROUP_MEMORY_FRACTION) as i64) / (1024 * 1024);
    format!("{}MiB", mib.max(64))
}

/// Best-effort detection of the container/cgroup memory limit in bytes.
/// `None` means no enforced limit (or detection failed) — we then leave
/// DuckDB's default behavior in place rather than guessing.
#[cfg(not(windows))]
fn detect_cgroup_memory_limit() -> Option<i64> {
    let raw = if std::path::Path::new("/sys/fs/cgroup/memory/memory.limit_in_bytes").exists() {
        // cgroup v1
        std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes").ok()?
    } else {
        // cgroup v2
        let cgroup = std::fs::read_to_string("/proc/self/cgroup").ok()?;
        let path = CGROUP_V2_PATH_RE
            .captures(&cgroup)
            .map(|c| format!("/sys/fs/cgroup{}/memory.max", c.get(1).unwrap().as_str()))?;
        std::fs::read_to_string(&path).ok()?
    };
    parse_cgroup_memory_value(&raw)
}

#[cfg(windows)]
fn detect_cgroup_memory_limit() -> Option<i64> {
    None
}

/// Resolve the `SET memory_limit` value, or `None` to keep DuckDB's default.
///
/// 1. `DUCKDB_MEMORY_LIMIT` env var, used verbatim (e.g. `512MB`, `2GB`, `60%`)
/// 2. 80% of the detected cgroup limit
/// 3. `None` — no container limit detected, keep DuckDB's own default (which is
///    already 80% of detected physical RAM, the correct behavior off-container)
fn resolve_memory_limit() -> Option<String> {
    if let Ok(v) = std::env::var("DUCKDB_MEMORY_LIMIT") {
        let v = v.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    detect_cgroup_memory_limit().map(duckdb_memory_limit_for_cgroup)
}

/// Escape a value for inclusion inside a single-quoted DuckDB SQL string
/// literal (env-var-provided values are operator-controlled but still escaped).
fn sql_single_quote(s: &str) -> String {
    s.replace('\'', "''")
}

/// Configure DuckDB so a long-lived worker running DuckDB queries cannot get
/// OOM-killed. Must run before any other statement on the connection. Three
/// levers:
/// - `allocator_background_threads`: makes DuckDB's bundled jemalloc return
///   freed memory to the OS after each job (fixes the steady-state RSS ratchet
///   over the worker's uptime; the per-job connection is already dropped, but
///   jemalloc retains pages otherwise).
/// - `memory_limit`: bounds a single query's tracked working set. Because
///   DuckDB runs in-process, the kernel OOM killer can only kill the whole
///   worker, never the job — so this limit is what converts an oversized
///   query into a graceful "exceeds memory limit" error scoped to that one
///   job, leaving the worker and its other jobs alive.
/// - `temp_directory=''`: deliberately DISABLES disk spill. This DuckDB
///   version defaults an in-memory database's temp_directory to `.tmp` (spill
///   on), so we must explicitly clear it: an oversized query should fail fast
///   and loudly as a job-scoped error, not silently degrade the node by
///   streaming gigabytes to disk.
fn configure_duckdb_resource_limits(conn: &duckdb::Connection) -> Result<(), String> {
    // Always on: independent of any cgroup/env detection, this is what stops
    // RSS from climbing over the worker's uptime.
    let mut config_sql =
        String::from("SET allocator_background_threads=true;\nSET temp_directory='';\n");
    if let Some(mem) = resolve_memory_limit() {
        config_sql.push_str(&format!("SET memory_limit='{}';\n", sql_single_quote(&mem)));
    }
    conn.execute_batch(&config_sql).map_err(|e| {
        format!(
            "Error configuring DuckDB resource limits: {}",
            e.to_string()
        )
    })
}

fn setup_duckdb_connection(
    conn: &duckdb::Connection,
    token: &str,
    base_internal_url: &str,
    w_id: &str,
) -> Result<(), String> {
    // Bound RAM and enable allocator memory release before anything else runs
    // on this connection (otherwise in-memory DuckDB has no cgroup-derived
    // limit and its allocator ratchets the worker's RSS to the cgroup cap).
    configure_duckdb_resource_limits(conn)?;

    let (s3_access_key, s3_secret_key) = token.rsplit_once('.').unwrap_or(("", token));
    let (s3_endpoint_ssl, s3_endpoint) = base_internal_url
        .split_once("://")
        .unwrap_or(("http", &base_internal_url));
    let s3_endpoint_ssl = s3_endpoint_ssl == "https";

    conn.execute_batch(&format!(
        "INSTALL httpfs; LOAD httpfs;
        INSTALL azure; LOAD azure;
        CREATE OR REPLACE SECRET s3_secret (
            TYPE s3,
            PROVIDER config,
            KEY_ID '{s3_access_key}',
            SECRET '{s3_secret_key}',
            ENDPOINT '{s3_endpoint}/api/w/{w_id}/s3_proxy',
            URL_STYLE path,
            USE_SSL {s3_endpoint_ssl}
        );
        CREATE OR REPLACE SECRET gcs_secret (
            TYPE gcs,
            KEY_ID '{s3_access_key}',
            SECRET '{s3_secret_key}',
            ENDPOINT '{s3_endpoint}/api/w/{w_id}/s3_proxy',
            USE_SSL {s3_endpoint_ssl}
        );
        ",
    ))
    .map_err(|e| format!("Error setting up S3 secret: {}", e.to_string()))
}

fn convert_prepare_args<'a>(
    query_block_list: *const *const c_char,
    query_block_list_count: usize,
    token: *const c_char,
    base_internal_url: *const c_char,
    w_id: *const c_char,
) -> Result<(Vec<&'a str>, &'a str, &'a str, &'a str), String> {
    let query_block_list = unsafe {
        std::slice::from_raw_parts(query_block_list, query_block_list_count)
            .iter()
            .map(|q| {
                CStr::from_ptr(*q).to_str().unwrap_or_else(|e| {
                    println!(
                        "Invalid query_block string pointer in duckdb ffi: {}",
                        e.to_string()
                    );
                    "Invalid query_block string pointer in duckdb ffi"
                })
            })
            .collect::<Vec<_>>()
    };
    let token = unsafe { CStr::from_ptr(token) }
        .to_str()
        .map_err(|e| format!("Invalid token string: {}", e.to_string()))?;
    let base_internal_url = unsafe { CStr::from_ptr(base_internal_url) }
        .to_str()
        .map_err(|e| format!("Invalid base_internal_url string: {}", e.to_string()))?;
    let w_id = unsafe { CStr::from_ptr(w_id) }
        .to_str()
        .map_err(|e| format!("Invalid w_id string: {}", e.to_string()))?;
    Ok((query_block_list, token, base_internal_url, w_id))
}

fn prepare_duckdb_internal(
    query_block_list: Vec<&str>,
    token: &str,
    base_internal_url: &str,
    w_id: &str,
) -> Result<String, String> {
    let conn = duckdb::Connection::open_in_memory().map_err(|e| e.to_string())?;

    setup_duckdb_connection(&conn, token, base_internal_url, w_id)?;

    let mut results: Vec<PrepareQueryResult> = vec![];

    // IMPORTANT: Setup statements (ATTACH, USE, INSTALL, etc.) are executed but intentionally
    // do not produce a PrepareQueryResult entry. The frontend prepends these as connection setup
    // before the actual user queries, and mapPrepareResults expects results.length to equal the
    // number of user queries (not setup statements). If a new setup-like statement is added to
    // the connection flow (e.g. in setup_duckdb_connection or transform_attach_ducklake) without
    // also being caught by is_setup_statement, the result count will mismatch and the frontend
    // will throw.
    for query_block in &query_block_list {
        if is_setup_statement(query_block) {
            conn.execute_batch(query_block)
                .map_err(|e| format!("Error executing setup statement: {}", e.to_string()))?;
            continue;
        }

        let modified_query = replace_params_with_null(query_block);
        // Validate the query parses correctly by preparing it
        if let Err(e) = conn.prepare(&modified_query) {
            results.push(PrepareQueryResult { columns: None, error: Some(e.to_string()) });
            continue;
        }

        // DESCRIBE only works on queries that return result sets (SELECT, WITH, VALUES, TABLE,
        // FROM). For non-returning statements (INSERT, UPDATE, DELETE, CREATE, DROP, ALTER, etc.)
        // we skip DESCRIBE and assume no columns.
        if !is_describable_query(&modified_query) {
            results.push(PrepareQueryResult { columns: Some(vec![]), error: None });
            continue;
        }

        // Note: We have to use a DESCRIBE statement and cannot simply use the
        // methods returned by .prepare() because they panic if the statement was
        // not executed at least once (which we specifically do not want to do).
        let describe_query = format!("DESCRIBE {}", modified_query);
        match conn.prepare(&describe_query).and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| {
                Ok(PrepareQueryColumnInfo {
                    name: row.get::<_, String>(0)?,
                    type_name: row.get::<_, String>(1)?,
                })
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
        }) {
            Ok(columns) => {
                results.push(PrepareQueryResult { columns: Some(columns), error: None });
            }
            Err(e) => {
                results.push(PrepareQueryResult { columns: None, error: Some(e.to_string()) });
            }
        }
    }

    serde_json::to_string(&results).map_err(|e| e.to_string())
}

fn convert_args<'a>(
    query_block_list: *const *const c_char,
    query_block_list_count: usize,
    job_args: *const c_char,
    token: *const c_char,
    base_internal_url: *const c_char,
    w_id: *const c_char,
) -> Result<
    (
        impl Iterator<Item = &'a str>,
        HashMap<String, duckdb::types::Value>,
        &'a str,
        &'a str,
        &'a str,
    ),
    String,
> {
    let query_block_list = unsafe {
        std::slice::from_raw_parts(query_block_list, query_block_list_count)
            .iter()
            .map(|q| {
                CStr::from_ptr(*q).to_str().unwrap_or_else(|e| {
                    println!(
                        "Invalid query_block string pointer in duckdb ffi: {}",
                        e.to_string()
                    );
                    "Invalid query_block string pointer in duckdb ffi"
                })
            })
    };
    let job_args_str = unsafe { CStr::from_ptr(job_args) }
        .to_str()
        .map_err(|e| format!("Invalid job_args string: {}", e.to_string()))?;
    let job_args: Vec<Arg> = serde_json::from_str(job_args_str)
        .map_err(|e| format!("Invalid job_args JSON: {}", e.to_string()))?;
    let job_args: HashMap<String, duckdb::types::Value> = job_args
        .into_iter()
        .map(|arg| {
            let duckdb_value = json_value_to_duckdb_value(&arg.json_value, &arg.arg_type)
                .unwrap_or_else(|e| {
                    println!(
                        "Error converting job_arg {} to duckdb value: {}",
                        arg.name, e
                    );
                    duckdb::types::Value::Null
                });
            (arg.name, duckdb_value)
        })
        .collect();
    let token = unsafe { CStr::from_ptr(token) }
        .to_str()
        .map_err(|e| format!("Invalid token string: {}", e.to_string()))?;
    let base_internal_url = unsafe { CStr::from_ptr(base_internal_url) }
        .to_str()
        .map_err(|e| format!("Invalid base_internal_url string: {}", e.to_string()))?;
    let w_id = unsafe { CStr::from_ptr(w_id) }
        .to_str()
        .map_err(|e| format!("Invalid w_id string: {}", e.to_string()))?;
    Ok((query_block_list, job_args, token, base_internal_url, w_id))
}

// TODO : Better error return to leverage different error kinds on the worker side
fn run_duckdb_internal<'a>(
    query_block_list: impl Iterator<Item = &'a str>,
    query_block_list_count: usize,
    job_args: HashMap<String, duckdb::types::Value>,
    token: &str,
    base_internal_url: &str,
    w_id: &str,
    collect_last_only: bool,
    collect_first_row_only: bool,
) -> Result<(String, Option<Vec<String>>), String> {
    let conn = duckdb::Connection::open_in_memory().map_err(|e| e.to_string())?;

    setup_duckdb_connection(&conn, token, base_internal_url, w_id)?;

    let mut results: Vec<Vec<Box<RawValue>>> = vec![];
    let mut column_order = None;

    for (query_block_index, query_block) in query_block_list.enumerate() {
        let result = do_duckdb_inner(
            &conn,
            query_block,
            &job_args,
            collect_last_only && query_block_index != query_block_list_count - 1,
            collect_first_row_only,
            &mut column_order,
        )
        .map_err(|e| e.to_string())?;
        results.push(result);
    }
    let results = serde_json::value::to_raw_value(&results).map_err(|e| e.to_string())?;
    Ok((results.get().to_string(), column_order))
}

fn do_duckdb_inner(
    conn: &duckdb::Connection,
    query: &str,
    job_args: &HashMap<String, duckdb::types::Value>,
    skip_collect: bool,
    collect_first_row_only: bool,
    column_order: &mut Option<Vec<String>>,
) -> Result<Vec<Box<RawValue>>, String> {
    let mut rows_vec = vec![];

    let (query, job_args) = interpolate_named_args(query, &job_args);

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query(params_from_iter(job_args))
        .map_err(|e| e.to_string())?;

    if skip_collect {
        return Ok(vec![]);
    }
    // Statement needs to be stepped at least once or stmt.column_names() will panic
    let mut column_names = None;
    let mut type_aliases = None;
    loop {
        let row = rows.next();
        match row {
            Ok(Some(row)) => {
                // Set column names if not already set
                let stmt = row.as_ref();

                let column_names = match column_names.as_ref() {
                    Some(column_names) => column_names,
                    None => {
                        column_names = Some(stmt.column_names());
                        column_names.as_ref().unwrap()
                    }
                };
                let type_aliases = match type_aliases.as_ref() {
                    Some(type_aliases) => type_aliases,
                    None => {
                        type_aliases = Some(
                            (0..stmt.column_count())
                                .map(|i| {
                                    let logical_type = stmt.column_logical_type(i);
                                    let logical_type_id = logical_type.id();
                                    let invalid = logical_type_id == LogicalTypeId::Invalid
                                        || logical_type_id == LogicalTypeId::Unsupported;
                                    if invalid {
                                        None
                                    } else {
                                        logical_type.get_alias()
                                    }
                                })
                                .collect::<Vec<_>>(),
                        );
                        type_aliases.as_ref().unwrap()
                    }
                };

                // let type_aliases = (0..stmt.column_count()).map(|_| None).collect::<Vec<_>>();

                let row = row_to_value(row, &column_names.as_slice(), &type_aliases.as_slice())
                    .map_err(|e| e.to_string())?;
                rows_vec.push(row);
            }
            Ok(None) => break,
            Err(e) => {
                return Err(e.to_string());
            }
        }
        if collect_first_row_only {
            break;
        }
    }

    *column_order = column_names;

    Ok(rows_vec)
}

// duckdb-rs does not support named parameters,
// and it raises an error when passing unused arguments. We cannot prepare batch statements
// but only single SQL statements so it doesn't work when all arguments are not used by
// every single statement.
fn interpolate_named_args<'a>(
    query: &str,
    args: &'a HashMap<String, duckdb::types::Value>,
) -> (String, Vec<&'a duckdb::types::Value>) {
    let mut query = query.to_string();

    let mut values = vec![];
    for (arg_name, arg_value) in args {
        let pat = format!("${}", arg_name);
        if !query.contains(&pat) {
            continue;
        }
        values.push(arg_value);
        query = query.replace(&pat, &format!("${}", values.len()));
    }
    (query, values)
}

fn row_to_value(
    row: &Row<'_>,
    column_names: &[String],
    type_aliases: &[Option<String>],
) -> Result<Box<RawValue>, String> {
    let mut obj = serde_json::Map::new();
    for (i, key) in column_names.iter().enumerate() {
        let value: duckdb::types::Value = row.get(i).map_err(|e| e.to_string())?;
        let type_alias = &type_aliases[i];
        let json_value = duckdb_value_to_json_value(value, type_alias)?;
        obj.insert(key.clone(), json_value);
    }
    serde_json::value::to_raw_value(&obj).map_err(|e| e.to_string())
}

fn duckdb_value_to_json_value(
    value: duckdb::types::Value,
    type_alias: &Option<String>,
) -> Result<serde_json::Value, String> {
    let json_value = match value {
        duckdb::types::Value::Null => serde_json::Value::Null,
        duckdb::types::Value::Boolean(b) => serde_json::Value::Bool(b),
        duckdb::types::Value::TinyInt(i) => serde_json::Value::Number(i.into()),
        duckdb::types::Value::SmallInt(i) => serde_json::Value::Number(i.into()),
        duckdb::types::Value::Int(i) => serde_json::Value::Number(i.into()),
        duckdb::types::Value::BigInt(i) => serde_json::Value::Number(i.into()),
        duckdb::types::Value::HugeInt(i) => serde_json::Value::String(i.to_string()),
        duckdb::types::Value::UTinyInt(u) => serde_json::Value::Number(u.into()),
        duckdb::types::Value::USmallInt(u) => serde_json::Value::Number(u.into()),
        duckdb::types::Value::UInt(u) => serde_json::Value::Number(u.into()),
        duckdb::types::Value::UBigInt(u) => serde_json::Value::Number(u.into()),
        duckdb::types::Value::Float(f) => serde_json::Value::Number(
            serde_json::Number::from_f64(f as f64)
                .ok_or_else(|| "Could not convert to f64".to_string())?,
        ),
        duckdb::types::Value::Double(f) => serde_json::Value::Number(
            serde_json::Number::from_f64(f)
                .ok_or_else(|| "Could not convert to f64".to_string())?,
        ),
        duckdb::types::Value::Decimal(d) => serde_json::Value::String(d.to_string()),
        duckdb::types::Value::Timestamp(_, ts) => serde_json::Value::String(ts.to_string()),
        duckdb::types::Value::Text(s) if type_alias.as_deref().unwrap_or_default() == "JSON" => {
            serde_json::from_str(&s)
                .map_err(|e| format!("Error parsing JSON text: {}", e.to_string()))?
        }
        duckdb::types::Value::Text(s) => serde_json::Value::String(s),
        duckdb::types::Value::Blob(b) => serde_json::Value::Array(
            b.into_iter()
                .map(|byte| serde_json::Value::Number(byte.into()))
                .collect(),
        ),
        duckdb::types::Value::Date32(d) => serde_json::Value::Number(d.into()),
        duckdb::types::Value::Time64(_, t) => serde_json::Value::String(t.to_string()),
        duckdb::types::Value::Interval { months, days, nanos } => serde_json::json!({
            "months": months,
            "days": days,
            "nanos": nanos
        }),
        duckdb::types::Value::List(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(|v| duckdb_value_to_json_value(v, &None))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        duckdb::types::Value::Enum(e) => serde_json::Value::String(e),
        duckdb::types::Value::Struct(fields) => serde_json::Value::Object(
            fields
                .iter()
                .map(|(k, v)| duckdb_value_to_json_value(v.clone(), &None).map(|v| (k.clone(), v)))
                .collect::<Result<serde_json::Map<_, _>, _>>()?,
        ),
        duckdb::types::Value::Array(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(|v| duckdb_value_to_json_value(v, &None))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        duckdb::types::Value::Map(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let k = match k {
                        duckdb::types::Value::Text(s) | duckdb::types::Value::Enum(s) => s.clone(),
                        _ => format!("{:?}", k),
                    };
                    duckdb_value_to_json_value(v.clone(), &None).map(|v| (k, v))
                })
                .collect::<Result<serde_json::Map<_, _>, _>>()?,
        ),
        duckdb::types::Value::Union(value) => serde_json::Value::String(format!("{:?}", *value)),
    };
    Ok(json_value)
}

fn json_value_to_duckdb_value(
    json_value: &serde_json::Value,
    arg_type: &str,
) -> Result<duckdb::types::Value, String> {
    let arg_type = arg_type.to_lowercase();
    let duckdb_value = match json_value {
        serde_json::Value::Null => duckdb::types::Value::Null,
        serde_json::Value::Bool(b) => duckdb::types::Value::Boolean(*b),

        serde_json::Value::String(s)
            if matches!(
                arg_type.as_str(),
                "timestamp" | "timestamptz" | "timestamp with time zone" | "datetime"
            ) =>
        {
            string_to_duckdb_timestamp(&s)?
        }
        serde_json::Value::String(s) if arg_type.as_str() == "date" => string_to_duckdb_date(&s)?,
        serde_json::Value::String(s) if arg_type.as_str() == "time" => string_to_duckdb_time(&s)?,
        serde_json::Value::String(s) => duckdb::types::Value::Text(s.clone()),

        serde_json::Value::Number(n) if n.is_i64() => {
            let v = n.as_i64().unwrap();
            match arg_type.as_str() {
                "tinyint" | "int1" => duckdb::types::Value::TinyInt(v as i8),
                "smallint" | "int2" | "short" => duckdb::types::Value::SmallInt(v as i16),
                "integer" | "int4" | "int" | "signed" => duckdb::types::Value::Int(v as i32),
                "bigint" | "int8" | "long" => duckdb::types::Value::BigInt(v),
                "hugeint" => duckdb::types::Value::HugeInt(v as i128),
                "float" | "float4" | "real" => duckdb::types::Value::Float(v as f32),
                "double" | "float8" => duckdb::types::Value::Double(v as f64),
                _ => duckdb::types::Value::BigInt(v), // default fallback
            }
        }

        serde_json::Value::Number(n) if n.is_u64() => {
            let v = n.as_u64().unwrap();
            match arg_type.as_str() {
                "utinyint" => duckdb::types::Value::UTinyInt(v as u8),
                "usmallint" => duckdb::types::Value::USmallInt(v as u16),
                "uinteger" => duckdb::types::Value::UInt(v as u32),
                "ubigint" | "uhugeint" => duckdb::types::Value::UBigInt(v),
                _ => duckdb::types::Value::UBigInt(v), // default fallback
            }
        }

        serde_json::Value::Number(n) if n.is_f64() => {
            let v = n.as_f64().unwrap();
            match arg_type.as_str() {
                "float" | "float4" | "real" => duckdb::types::Value::Float(v as f32),
                "double" | "float8" => duckdb::types::Value::Double(v),
                "decimal" | "numeric" => duckdb::types::Value::Decimal(
                    Decimal::from_f64(v)
                        .ok_or_else(|| "Could not convert f64 to Decimal".to_string())?,
                ),
                _ => duckdb::types::Value::Double(v), // default fallback
            }
        }

        serde_json::Value::Array(arr) => {
            duckdb::types::Value::Text(serde_json::to_string(arr).map_err(|e| e.to_string())?)
        }
        serde_json::Value::Object(map) => {
            duckdb::types::Value::Text(serde_json::to_string(map).map_err(|e| e.to_string())?)
        }

        value @ _ => {
            return Err(format!(
                "Unsupported type in query: {:?} and signature {arg_type:?}",
                value
            ));
        }
    };
    Ok(duckdb_value)
}

fn string_to_duckdb_timestamp(s: &str) -> Result<duckdb::types::Value, String> {
    let ts =
        chrono::DateTime::parse_from_rfc3339(s).map_err(|e: chrono::ParseError| e.to_string())?;
    Ok(duckdb::types::Value::Timestamp(
        TimeUnit::Millisecond,
        ts.timestamp_millis(),
    ))
}

fn string_to_duckdb_date(s: &str) -> Result<duckdb::types::Value, String> {
    use chrono::Datelike;
    let date = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;
    Ok(duckdb::types::Value::Date32(date.num_days_from_ce()))
}

fn string_to_duckdb_time(s: &str) -> Result<duckdb::types::Value, String> {
    use chrono::Timelike;
    let time = chrono::NaiveTime::parse_from_str(s, "%H:%M:%S").unwrap();
    Ok(duckdb::types::Value::Time64(
        TimeUnit::Microsecond,
        time.num_seconds_from_midnight() as i64,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cgroup_memory_value_handles_unlimited() {
        // cgroup v2 unlimited sentinel
        assert_eq!(parse_cgroup_memory_value("max"), None);
        assert_eq!(parse_cgroup_memory_value("  max\n"), None);
        // cgroup v1 unlimited: page-aligned ~i64::MAX, well above 1 PiB
        assert_eq!(parse_cgroup_memory_value("9223372036854771712"), None);
        // non-positive / unparseable
        assert_eq!(parse_cgroup_memory_value("0"), None);
        assert_eq!(parse_cgroup_memory_value("-1"), None);
        assert_eq!(parse_cgroup_memory_value("notanumber"), None);
    }

    #[test]
    fn parse_cgroup_memory_value_accepts_real_limits() {
        assert_eq!(parse_cgroup_memory_value("1073741824"), Some(1073741824));
        // trailing newline as written by the kernel
        assert_eq!(parse_cgroup_memory_value("536870912\n"), Some(536870912));
        // just below the unlimited threshold is still a real limit
        assert_eq!(
            parse_cgroup_memory_value(&(CGROUP_UNLIMITED_THRESHOLD - 1).to_string()),
            Some(CGROUP_UNLIMITED_THRESHOLD - 1)
        );
    }

    #[test]
    fn duckdb_memory_limit_is_fraction_of_cgroup() {
        // 1 GiB cgroup -> 80% -> 819 MiB
        assert_eq!(duckdb_memory_limit_for_cgroup(1024 * 1024 * 1024), "819MiB");
        // 4 GiB cgroup -> 3276 MiB
        assert_eq!(
            duckdb_memory_limit_for_cgroup(4 * 1024 * 1024 * 1024),
            "3276MiB"
        );
    }

    #[test]
    fn duckdb_memory_limit_is_floored() {
        // A tiny cgroup must not produce a 0 / unusable limit.
        assert_eq!(duckdb_memory_limit_for_cgroup(1024 * 1024), "64MiB");
        assert_eq!(duckdb_memory_limit_for_cgroup(1), "64MiB");
    }

    #[test]
    fn sql_single_quote_escapes() {
        assert_eq!(sql_single_quote("512MB"), "512MB");
        assert_eq!(sql_single_quote("a'b"), "a''b");
        assert_eq!(sql_single_quote("'; DROP"), "''; DROP");
    }

    // End-to-end: the generated SQL must be accepted by the bundled DuckDB
    // version, and the no-spill behavior must actually hold — an oversized
    // query must fail as a job-scoped error, not silently spill to disk.
    // This test owns DUCKDB_MEMORY_LIMIT (no other test reads it).
    #[test]
    fn configure_resource_limits_applies_to_real_connection() {
        unsafe {
            std::env::set_var("DUCKDB_MEMORY_LIMIT", "64MiB");
        }

        let conn = duckdb::Connection::open_in_memory().unwrap();
        configure_duckdb_resource_limits(&conn)
            .expect("DuckDB rejected the generated resource-limit SQL");

        // The allocator-release lever must be accepted by this DuckDB version
        // and actually enabled (cast to VARCHAR: it is a BOOLEAN setting).
        let alloc: String = conn
            .query_row(
                "SELECT current_setting('allocator_background_threads')::VARCHAR",
                [],
                |r| r.get(0),
            )
            .expect("allocator_background_threads not supported by bundled DuckDB");
        assert_eq!(alloc, "true", "allocator_background_threads not enabled");

        let mem: String = conn
            .query_row("SELECT current_setting('memory_limit')", [], |r| r.get(0))
            .unwrap();
        assert!(
            mem.contains("64.0 MiB") || mem.contains("64MiB") || mem.contains("67.1 MB"),
            "unexpected memory_limit: {mem}"
        );

        // Spill must be disabled: temp_directory cleared to empty (this DuckDB
        // version otherwise defaults an in-memory DB to `.tmp`).
        let temp_dir: String = conn
            .query_row("SELECT current_setting('temp_directory')", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            temp_dir, "",
            "temp_directory must be empty (spill disabled)"
        );

        // Behavioral proof: a query whose hash table far exceeds 64 MiB must
        // ERROR (job-scoped) rather than succeed by spilling to disk.
        let oversized = conn.query_row(
            "SELECT count(*) FROM range(20000000) t1(a) \
             JOIN range(20000000) t2(b) ON t1.a = t2.b",
            [],
            |r| r.get::<_, i64>(0),
        );
        let err = oversized
            .expect_err("oversized query unexpectedly succeeded — it spilled to disk?")
            .to_string()
            .to_lowercase();
        assert!(
            err.contains("memory"),
            "expected an out-of-memory error, got: {err}"
        );

        unsafe {
            std::env::remove_var("DUCKDB_MEMORY_LIMIT");
        }
    }
}
