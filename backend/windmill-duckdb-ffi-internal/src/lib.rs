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

// Worker passes "" for "no override" — saves an extra C string nullability dance.
// Returns an owned String so the value outlives the raw pointer's lifetime.
fn ptr_to_opt_str(ptr: *const c_char) -> Result<Option<String>, String> {
    if ptr.is_null() {
        return Ok(None);
    }
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|e| format!("Invalid string in duckdb ffi: {}", e))?;
    Ok(if s.is_empty() {
        None
    } else {
        Some(s.to_owned())
    })
}

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
    return 3;
}

#[unsafe(no_mangle)]
pub extern "C" fn run_duckdb_ffi(
    query_block_list: *const *const c_char,
    query_block_list_count: usize,
    job_args: *const c_char,
    token: *const c_char,
    base_internal_url: *const c_char,
    w_id: *const c_char,
    memory_limit: *const c_char,
    temp_directory: *const c_char,
    // 0 means unbounded; the worker sends its own budget-derived cap.
    max_result_size: usize,
    column_order_ptr: *mut *mut c_char,
    collect_last_only: bool,
    collect_first_row_only: bool,
) -> *mut c_char {
    let resource_limits = match (ptr_to_opt_str(memory_limit), ptr_to_opt_str(temp_directory)) {
        (Ok(m), Ok(t)) => Ok(ResourceLimits::from_ffi(m, t)),
        (Err(e), _) | (_, Err(e)) => Err(e),
    };
    let (r, column_order) = match convert_args(
        query_block_list,
        query_block_list_count,
        job_args,
        token,
        base_internal_url,
        w_id,
    )
    .and_then(|args| resource_limits.map(|r| (args, r)))
    .and_then(
        |((query_block_list, job_args, token, base_internal_url, w_id), limits)| {
            run_duckdb_internal(
                query_block_list,
                query_block_list_count,
                job_args,
                token,
                base_internal_url,
                w_id,
                limits,
                max_result_size,
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
        || is_create_temp_macro(&upper)
}

/// `CREATE [OR REPLACE] TEMP|TEMPORARY MACRO` — a connection-scoped definition
/// that returns no result set and must be *executed* (not just prepared) during
/// the prepare/diagnostics pass, so later blocks that call the macro bind
/// against it instead of failing "function does not exist". Both the materialize
/// runtime (`wm_partition`) and the workspace-macro splicer inject these ahead
/// of the query that uses them, so — like ATTACH — they are connection setup,
/// not user queries (they must not add a `PrepareQueryResult` entry either).
/// Persistent `CREATE MACRO` (no TEMP) is a real user statement and is excluded.
fn is_create_temp_macro(upper: &str) -> bool {
    let norm = upper.split_whitespace().collect::<Vec<_>>().join(" ");
    norm.starts_with("CREATE TEMP MACRO")
        || norm.starts_with("CREATE TEMPORARY MACRO")
        || norm.starts_with("CREATE OR REPLACE TEMP MACRO")
        || norm.starts_with("CREATE OR REPLACE TEMPORARY MACRO")
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
    memory_limit: *const c_char,
    temp_directory: *const c_char,
) -> *mut c_char {
    let resource_limits = match (ptr_to_opt_str(memory_limit), ptr_to_opt_str(temp_directory)) {
        (Ok(m), Ok(t)) => Ok(ResourceLimits::from_ffi(m, t)),
        (Err(e), _) | (_, Err(e)) => Err(e),
    };
    let r = match convert_prepare_args(
        query_block_list,
        query_block_list_count,
        token,
        base_internal_url,
        w_id,
    )
    .and_then(|args| resource_limits.map(|r| (args, r)))
    .and_then(
        |((query_block_list, token, base_internal_url, w_id), limits)| {
            prepare_duckdb_internal(query_block_list, token, base_internal_url, w_id, limits)
        },
    ) {
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

#[derive(Clone, Default)]
struct ResourceLimits {
    memory_limit: Option<String>,
    temp_directory: Option<String>,
    max_temp_directory_size: Option<String>,
}

impl ResourceLimits {
    // The spill cap comes from the environment rather than from an FFI parameter: this
    // cdylib is versioned separately from the worker and agent workers ship it out of band,
    // so a signature change costs an ABI bump and a coordinated redeploy for what is a
    // config value. Unset keeps DuckDB's default of 90% of the temp volume's free space.
    fn from_ffi(memory_limit: Option<String>, temp_directory: Option<String>) -> Self {
        let max_temp_directory_size = std::env::var("DUCKDB_MAX_TEMP_DIRECTORY_SIZE")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        ResourceLimits { memory_limit, temp_directory, max_temp_directory_size }
    }
}

fn sql_single_quote(s: &str) -> String {
    s.replace('\'', "''")
}

// Bounds memory so DuckDB spills before blowing the cgroup cap and getting the worker
// SIGKILLed, and bounds the spill so one query cannot fill its disk. Spill goes to the job
// dir when set, so it is cleaned up with the job; either way the directory and its cap are
// then locked for the rest of the connection.
fn configure_duckdb_resource_limits(
    conn: &duckdb::Connection,
    limits: &ResourceLimits,
) -> Result<(), String> {
    let mut config_sql = String::new();
    // jemalloc-specific setting bundled with the Linux DuckDB build. macOS and
    // Windows builds may not accept it; gated to avoid breaking those workers.
    if cfg!(target_os = "linux") {
        config_sql.push_str("SET allocator_background_threads=true;\n");
    }
    if let Some(mem) = limits.memory_limit.as_deref() {
        config_sql.push_str(&format!("SET memory_limit='{}';\n", sql_single_quote(mem)));
    }
    if let Some(tmp) = limits.temp_directory.as_deref() {
        config_sql.push_str(&format!(
            "SET temp_directory='{}';\n",
            sql_single_quote(tmp)
        ));
    }
    let run = |sql: &str| {
        conn.execute_batch(sql).map_err(|e| {
            format!(
                "Error configuring DuckDB resource limits: {}",
                e.to_string()
            )
        })
    };
    run(&config_sql)?;
    // Applied on its own rather than batched with the rest so an unparseable value names the
    // knob to fix: it is spelled in the same units as DUCKDB_MEMORY_LIMIT and DuckDB's parse
    // error says only "memory", so together the two are indistinguishable to whoever set them.
    if let Some(max) = limits.max_temp_directory_size.as_deref() {
        conn.execute_batch(&format!(
            "SET max_temp_directory_size='{}';\n",
            sql_single_quote(max)
        ))
        .map_err(|e| {
            format!(
                "Error applying DUCKDB_MAX_TEMP_DIRECTORY_SIZE='{}': {}",
                max,
                e.to_string()
            )
        })?;
    }
    // Freeze both before any script statement can touch them. A remote temp directory makes
    // `~TemporaryDirectoryHandle` throw out of a `noexcept` destructor — `std::terminate`,
    // taking this in-process worker and every job on it down, uncatchable from Rust — and a
    // cap a script can raise bounds nothing.
    run("SET lock_temp_directory=true;\n")
}

fn setup_duckdb_connection(
    conn: &duckdb::Connection,
    token: &str,
    base_internal_url: &str,
    w_id: &str,
    limits: &ResourceLimits,
) -> Result<(), String> {
    configure_duckdb_resource_limits(conn, limits)?;

    let (s3_access_key, s3_secret_key) = token.rsplit_once('.').unwrap_or(("", token));
    let (s3_endpoint_ssl, s3_endpoint) = base_internal_url
        .split_once("://")
        .unwrap_or(("http", &base_internal_url));
    let s3_endpoint_ssl = s3_endpoint_ssl == "https";

    // Escape values interpolated into single-quoted SQL literals, consistent with
    // configure_duckdb_resource_limits above (a stray quote would break the statement).
    let s3_access_key = sql_single_quote(s3_access_key);
    let s3_secret_key = sql_single_quote(s3_secret_key);
    let endpoint = sql_single_quote(&format!("{s3_endpoint}/api/w/{w_id}/s3_proxy"));

    conn.execute_batch(&format!(
        "INSTALL httpfs; LOAD httpfs;
        INSTALL azure; LOAD azure;
        CREATE OR REPLACE SECRET s3_secret (
            TYPE s3,
            PROVIDER config,
            KEY_ID '{s3_access_key}',
            SECRET '{s3_secret_key}',
            ENDPOINT '{endpoint}',
            URL_STYLE path,
            USE_SSL {s3_endpoint_ssl}
        );
        CREATE OR REPLACE SECRET gcs_secret (
            TYPE gcs,
            KEY_ID '{s3_access_key}',
            SECRET '{s3_secret_key}',
            ENDPOINT '{endpoint}',
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
    limits: ResourceLimits,
) -> Result<String, String> {
    let conn = duckdb::Connection::open_in_memory().map_err(|e| e.to_string())?;

    setup_duckdb_connection(&conn, token, base_internal_url, w_id, &limits)?;

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
    limits: ResourceLimits,
    max_result_size: usize,
    collect_last_only: bool,
    collect_first_row_only: bool,
) -> Result<(String, Option<Vec<String>>), String> {
    let conn = duckdb::Connection::open_in_memory().map_err(|e| e.to_string())?;

    setup_duckdb_connection(&conn, token, base_internal_url, w_id, &limits)?;

    let mut results: Vec<Vec<Box<RawValue>>> = vec![];
    let mut column_order = None;
    // Carried across blocks: what the worker cannot survive is the total it ends
    // up holding, not any one statement's share of it.
    let mut collected = 0usize;

    for (query_block_index, query_block) in query_block_list.enumerate() {
        let result = do_duckdb_inner(
            &conn,
            query_block,
            &job_args,
            collect_last_only && query_block_index != query_block_list_count - 1,
            collect_first_row_only,
            &mut column_order,
            max_result_size,
            &mut collected,
        )
        .map_err(|e| e.to_string())?;
        results.push(result);
    }
    // Serializing straight to the String that crosses the FFI boundary; going
    // through a RawValue first would hold a second copy of the whole result.
    serde_json::to_string(&results)
        .map(|results| (results, column_order))
        .map_err(|e| e.to_string())
}

/// Beyond its JSON, every collected row costs a fat pointer parked in `rows_vec`
/// plus its own heap allocation, which the allocator rounds up and prefixes with
/// a header. Measured at 64 bytes resident for a 14-byte row, falling to a flat
/// ~34 bytes of overhead once rows are wide; 56 sits above that whole range, so
/// the charge is never less than what a row actually occupies.
///
/// Charging the payload alone is what makes small rows dangerous: `{"a":0}` is
/// seven bytes but occupies 64, so a cap counting only JSON would admit nine
/// times the rows the worker can hold.
const ROW_STORAGE_OVERHEAD: usize = 56;

fn row_storage_cost(json_len: usize) -> usize {
    json_len + ROW_STORAGE_OVERHEAD
}

// Duplicated from windmill-worker rather than shared: this crate is deliberately
// outside the workspace so the bundled engine compiles on its own.
fn format_byte_size(bytes: usize) -> String {
    match bytes {
        b if b >= 1 << 30 => format!("{:.1}GB", b as f64 / (1u64 << 30) as f64),
        b if b >= 1 << 20 => format!("{}MB", b >> 20),
        b if b >= 1 << 10 => format!("{}KB", b >> 10),
        b => format!("{b}B"),
    }
}

/// The limit is on what the collection *retains* — each row's JSON plus the
/// storage holding it (`row_storage_cost`) — not on how large the result reads.
/// For narrow rows that storage dominates, so the guard can fire while the JSON
/// alone is well under the quoted figure. That is the number worth defending:
/// the worker dies on what it holds, not on what it would have returned.
///
/// Rows whose destination is storage should never be collected at all, so lead
/// with the way to keep them out of it.
///
/// Only the limit is quoted. Neither caller knows a number worth naming: the row
/// loop stops on the row that crosses the threshold, so its total is the threshold
/// plus one row, and the expansion guard refuses on a projection while the total
/// can still be zero.
fn result_too_large_message(max_result_size: usize) -> String {
    format!(
        "Query result too large: collecting it passed the {} limit.\n\
         Write the rows out from the query instead of returning them:\n  \
         COPY (<your query>) TO 's3://<bucket>/<file>.parquet' (FORMAT PARQUET);\n\
         Or return fewer rows — aggregate, or add a LIMIT. On a self-hosted \
         worker, MAX_SQL_RESULT_SIZE raises the limit.",
        format_byte_size(max_result_size),
    )
}

#[cfg(test)]
mod result_size_guard_tests {
    use super::*;

    const ROWS: usize = 100_000;

    fn collect_into(max_result_size: usize, collected: &mut usize) -> Result<usize, String> {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        do_duckdb_inner(
            &conn,
            &format!("SELECT i FROM range({ROWS}) t(i)"),
            &HashMap::new(),
            false,
            false,
            &mut None,
            max_result_size,
            collected,
        )
        .map(|rows| rows.len())
    }

    #[test]
    fn oversized_result_is_refused_mid_collection() {
        let err = collect_into(1024, &mut 0).expect_err("1KB cannot hold 100k rows");
        assert!(err.contains("Query result too large"), "{err}");
        // The limit alone leaves the user stuck; the way out has to travel with it.
        assert!(err.contains("COPY ("), "no remedy in the error: {err}");
    }

    #[test]
    fn zero_is_unbounded() {
        assert_eq!(collect_into(0, &mut 0).unwrap(), ROWS);
    }

    /// A blob expands to one `serde_json::Value` per byte on the way in, so it is
    /// the one value that can exhaust the worker between two checks of the running
    /// total. It has to be refused on its length, before that expansion is built.
    #[test]
    fn a_wide_blob_is_refused_before_it_is_expanded() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        let mut collected = 0;
        // 1MB of blob expands to ~72MB of `Value`, well past this budget, while the
        // running total is still zero — nothing else in the loop would catch it.
        let err = do_duckdb_inner(
            &conn,
            "SELECT repeat('a', 1000000)::BLOB AS b",
            &HashMap::new(),
            false,
            false,
            &mut None,
            1024 * 1024,
            &mut collected,
        )
        .expect_err("a blob expanding past the budget must be refused");
        assert!(err.contains("Query result too large"), "{err}");
        assert_eq!(collected, 0, "it was refused before being counted");
    }

    /// The text length is the only signal available while refusing is still
    /// possible — once the tree exists the memory is committed.
    #[test]
    fn a_dense_json_column_is_refused_before_it_is_parsed() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        let attempt = |budget: usize| {
            do_duckdb_inner(
                &conn,
                // ~200KB of text, ~7MB once it is a tree of `Value`s.
                "SELECT ('[' || repeat('1,', 99999) || '1]')::JSON AS j",
                &HashMap::new(),
                false,
                false,
                &mut None,
                budget,
                &mut 0,
            )
        };

        let err = attempt(1024 * 1024).expect_err("the parsed tree does not fit 1MB");
        assert!(err.contains("Query result too large"), "{err}");
        assert_eq!(attempt(64 * 1024 * 1024).unwrap().len(), 1);
    }

    /// `Budget` charges expansions visible in the value tree, and escaping is not
    /// one of them — it happens while the finished row is written, which is before
    /// anything weighs it. Only bounding the writer catches that.
    #[test]
    fn escaping_cannot_outgrow_the_row_budget() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        // 100k control characters: ~100KB of `String`, ~600KB once escaped.
        let attempt = |budget: usize| {
            do_duckdb_inner(
                &conn,
                "SELECT repeat(chr(1), 100000) AS a",
                &HashMap::new(),
                false,
                false,
                &mut None,
                budget,
                &mut 0,
            )
        };

        let err = attempt(300_000).expect_err("the escaped form does not fit 300KB");
        assert!(err.contains("Query result too large"), "{err}");
        // Room for the escaped form, so the same row goes through.
        assert_eq!(attempt(2_000_000).unwrap().len(), 1);
    }

    /// The budget is per row, not per value: a blob nested in a container reaches
    /// the same expansion by a path the top-level columns never touch, and several
    /// modest sibling blobs reach it by summing. Both are invisible to a check
    /// that only looks at one value at a time.
    #[test]
    fn nested_and_sibling_blobs_share_one_budget() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        let attempt = |query: &str| {
            do_duckdb_inner(
                &conn,
                query,
                &HashMap::new(),
                false,
                false,
                &mut None,
                8 * 1024 * 1024,
                &mut 0,
            )
        };

        // One 1MB blob expands to ~72MB, over the 8MB cap, but it is reached only
        // by descending into the LIST.
        let nested = attempt("SELECT [repeat('a', 1000000)::BLOB] AS b")
            .expect_err("a blob inside a LIST must be charged too");
        assert!(nested.contains("Query result too large"), "{nested}");

        // Four 30KB blobs are ~2.2MB each — every one fits on its own, the row does not.
        let siblings = attempt(
            "SELECT repeat('a', 30000)::BLOB AS a, repeat('b', 30000)::BLOB AS b,
                    repeat('c', 30000)::BLOB AS c, repeat('d', 30000)::BLOB AS d",
        )
        .expect_err("sibling blobs must be charged against one shared budget");
        assert!(siblings.contains("Query result too large"), "{siblings}");
    }

    /// A row costs far more than the JSON it serializes to, and small rows are
    /// almost entirely that overhead. Billing only the payload would let a cap
    /// admit several times the rows the worker can actually hold.
    #[test]
    fn a_row_costs_more_than_its_payload() {
        assert_eq!(row_storage_cost(7), 7 + ROW_STORAGE_OVERHEAD);

        let mut collected = 0;
        collect_into(usize::MAX, &mut collected).unwrap();
        assert!(
            collected >= ROWS * ROW_STORAGE_OVERHEAD,
            "charged {collected} for {ROWS} rows, so containers went unbilled"
        );
    }

    /// The worker dies on the total it holds, not on any one statement, so the
    /// budget has to carry across blocks — a per-statement reset would let N
    /// statements each pass the check and still OOM the worker together.
    #[test]
    fn the_budget_is_shared_across_statements() {
        let mut collected = 0;
        let first = collect_into(usize::MAX, &mut collected).unwrap();
        assert_eq!(first, ROWS);
        assert!(collected > 0);

        let err = collect_into(collected, &mut collected)
            .expect_err("the first statement already spent the whole budget");
        assert!(err.contains("Query result too large"), "{err}");
    }
}

fn do_duckdb_inner(
    conn: &duckdb::Connection,
    query: &str,
    job_args: &HashMap<String, duckdb::types::Value>,
    skip_collect: bool,
    collect_first_row_only: bool,
    column_order: &mut Option<Vec<String>>,
    max_result_size: usize,
    collected: &mut usize,
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

                let row = row_to_value(
                    row,
                    &column_names.as_slice(),
                    &type_aliases.as_slice(),
                    max_result_size.saturating_sub(*collected),
                    max_result_size,
                )
                .map_err(|e| e.to_string())?;
                *collected += row_storage_cost(row.get().len());
                if max_result_size > 0 && *collected > max_result_size {
                    return Err(result_too_large_message(max_result_size));
                }
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

/// Serializes to JSON without letting the output outgrow `budget`.
///
/// The expansions the `Budget` charges are the ones visible in the value tree;
/// escaping is not one of them. A control character becomes the six-byte
/// `\u0001`, so a row that fit the budget as `Value`s can still allocate several
/// times it while being written — and the write happens before the finished row
/// is ever weighed. Bounding the writer keeps that inside the budget.
///
/// `None` is reported to the caller as "too large", which is only honest because
/// the callers pass a `serde_json::Map` of `Value`s: serializing one cannot fail
/// for any reason except the budget. A caller passing a type with a fallible
/// `Serialize` would have its error silently retold as a size limit.
fn to_raw_value_within<T: serde::Serialize>(value: &T, budget: usize) -> Option<Box<RawValue>> {
    struct Budgeted {
        buf: Vec<u8>,
        left: usize,
    }
    impl std::io::Write for Budgeted {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.write_all(bytes)?;
            Ok(bytes.len())
        }
        // `Vec<u8>` overrides this too: the default implementation loops over
        // `write`, and serde_json emits a great many small pieces per row.
        fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
            if bytes.len() > self.left {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "row over budget",
                ));
            }
            self.left -= bytes.len();
            self.buf.extend_from_slice(bytes);
            Ok(())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut writer = Budgeted { buf: Vec::new(), left: budget };
    serde_json::to_writer(&mut writer, value).ok()?;
    let json = String::from_utf8(writer.buf).ok()?;
    // SAFETY: `to_writer` returned `Ok`, so `json` holds one complete, well-formed
    // JSON value with no surrounding whitespace. Running out of budget is the only
    // way a partial write happens, and it takes the `?` above instead of reaching
    // here. The safe constructor re-parses every row to learn the same thing, which
    // measured ~1.8x the cost of serializing it in the first place; serde_json
    // itself builds a `RawValue` this way in `to_raw_value`, and `debug_assert!`s
    // the invariant by re-parsing in debug builds.
    Some(unsafe { RawValue::from_string_unchecked(json) })
}

/// How much a row may still expand to, carried through every value it contains.
///
/// Only weighed rows reach the running total, so anything that balloons *during*
/// conversion is invisible to it — a blob becomes one `serde_json::Value` per
/// byte, and both several sibling columns and one nested deep in a `LIST`/`STRUCT`
/// can each be individually modest while summing past what the worker can hold.
/// One shared, decrementing budget is what makes those cases add up.
/// Not covered: `row.get` materializes a `LIST`/`STRUCT` column before any budget
/// exists, and `Debug`-rendered `MAP` keys and `UNION`s expand ~5x. All are
/// upstream of or far cheaper than the 36x/72x paths above, and the first needs a
/// streaming accessor from `duckdb-rs` to fix at all.
struct Budget {
    remaining: usize,
    /// The configured cap, kept only so the error can quote it rather than the
    /// remainder, which would be a different number every time.
    limit: usize,
}

fn row_to_value(
    row: &Row<'_>,
    column_names: &[String],
    type_aliases: &[Option<String>],
    budget: usize,
    max_result_size: usize,
) -> Result<Box<RawValue>, String> {
    // Shared by every value in the row, and decremented as each is built, so
    // columns that are individually under the cap cannot add up past it.
    let mut budget = Budget {
        remaining: if max_result_size == 0 {
            usize::MAX
        } else {
            budget
        },
        limit: max_result_size,
    };
    let mut obj = serde_json::Map::new();
    for (i, key) in column_names.iter().enumerate() {
        let value: duckdb::types::Value = row.get(i).map_err(|e| e.to_string())?;
        let type_alias = &type_aliases[i];
        let json_value = duckdb_value_to_json_value(value, type_alias, &mut budget)
            .map_err(|e| format!("column \"{key}\": {e}"))?;
        obj.insert(key.clone(), json_value);
    }
    to_raw_value_within(&obj, budget.remaining)
        .ok_or_else(|| result_too_large_message(budget.limit))
}

fn duckdb_value_to_json_value(
    value: duckdb::types::Value,
    type_alias: &Option<String>,
    budget: &mut Budget,
) -> Result<serde_json::Value, String> {
    let json_value = match value {
        duckdb::types::Value::Null => serde_json::Value::Null,
        duckdb::types::Value::Boolean(b) => serde_json::Value::Bool(b),
        duckdb::types::Value::TinyInt(i) => serde_json::Value::Number(i.into()),
        duckdb::types::Value::SmallInt(i) => serde_json::Value::Number(i.into()),
        duckdb::types::Value::Int(i) => serde_json::Value::Number(i.into()),
        duckdb::types::Value::BigInt(i) => serde_json::Value::Number(i.into()),
        duckdb::types::Value::HugeInt(i) => serde_json::Value::String(i.to_string()),
        duckdb::types::Value::UHugeInt(u) => serde_json::Value::String(u.to_string()),
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
        duckdb::types::Value::Timestamp(unit, ts) => {
            serde_json::Value::String(duckdb_timestamp_to_iso(unit, ts))
        }
        duckdb::types::Value::Text(s) if type_alias.as_deref().unwrap_or_default() == "JSON" => {
            // An element can be two bytes of text (`1,`) and becomes a whole
            // `Value`, so the retained tree reaches half `size_of::<Value>()`
            // times the text, and the parser's own growth can hold ~2x that
            // again. Charged before parsing; afterwards the memory is committed.
            let expanded = s
                .len()
                .saturating_mul(std::mem::size_of::<serde_json::Value>() / 2);
            if expanded > budget.remaining {
                return Err(result_too_large_message(budget.limit));
            }
            budget.remaining -= expanded;
            serde_json::from_str(&s)
                .map_err(|e| format!("Error parsing JSON text: {}", e.to_string()))?
        }
        duckdb::types::Value::Text(s) => serde_json::Value::String(s),
        // GEOMETRY surfaces as WKB bytes; render it like any other byte string.
        duckdb::types::Value::Blob(b) | duckdb::types::Value::Geometry(b) => {
            // Refused on the length, which is known now — once the array below
            // exists the memory is already spent, and the row is not weighed
            // until every one of its columns has been built.
            //
            // The multiplier is not the modest number it looks like: `Value` is
            // 72 bytes here, because `preserve_order` backs objects with an
            // IndexMap. A 50MB blob becomes ~3.6GB of `Value` before anything
            // has had a chance to object.
            let expanded = b
                .len()
                .saturating_mul(std::mem::size_of::<serde_json::Value>());
            if expanded > budget.remaining {
                return Err(result_too_large_message(budget.limit));
            }
            budget.remaining -= expanded;
            serde_json::Value::Array(
                b.into_iter()
                    .map(|byte| serde_json::Value::Number(byte.into()))
                    .collect(),
            )
        }
        duckdb::types::Value::Date32(d) => {
            match chrono::DateTime::from_timestamp(i64::from(d) * 86_400, 0) {
                Some(dt) => serde_json::Value::String(dt.date_naive().to_string()),
                None => serde_json::Value::Number(d.into()),
            }
        }
        duckdb::types::Value::Time64(unit, t) => {
            serde_json::Value::String(duckdb_time_to_iso(unit, t).unwrap_or_else(|| t.to_string()))
        }
        duckdb::types::Value::Interval { months, days, nanos } => serde_json::json!({
            "months": months,
            "days": days,
            "nanos": nanos
        }),
        duckdb::types::Value::List(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(|v| duckdb_value_to_json_value(v, &None, budget))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        duckdb::types::Value::Enum(e) => serde_json::Value::String(e),
        duckdb::types::Value::Struct(fields) => serde_json::Value::Object(
            fields
                .iter()
                .map(|(k, v)| {
                    duckdb_value_to_json_value(v.clone(), &None, budget).map(|v| (k.clone(), v))
                })
                .collect::<Result<serde_json::Map<_, _>, _>>()?,
        ),
        duckdb::types::Value::Array(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(|v| duckdb_value_to_json_value(v, &None, budget))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        duckdb::types::Value::Map(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let k = match k {
                        duckdb::types::Value::Text(s) | duckdb::types::Value::Enum(s) => s.clone(),
                        _ => format!("{:?}", k),
                    };
                    duckdb_value_to_json_value(v.clone(), &None, budget).map(|v| (k, v))
                })
                .collect::<Result<serde_json::Map<_, _>, _>>()?,
        ),
        duckdb::types::Value::Union(value) => serde_json::Value::String(format!("{:?}", *value)),
        // `Value` is `#[non_exhaustive]`: a newer engine can hand back a variant this
        // build has never seen. Name its type instead of emitting a plausible-looking
        // rendering that silently misrepresents the column. The type, never the value:
        // a row that reaches here is already unprintable, and it may be arbitrarily
        // large or hold data that does not belong in a job's error message.
        other => {
            return Err(format!(
                "DuckDB type {:?} is not supported by this worker's engine bindings; \
                 cast the column to a supported type (e.g. VARCHAR) to return it",
                other.data_type()
            ))
        }
    };
    Ok(json_value)
}

// DuckDB surfaces TIMESTAMP[_S/_MS/_NS] values as a raw count since the epoch;
// stringifying that count leaks values like "1782974022218435" into job
// results. Render ISO-8601 instead (the Postgres executor's
// "2024-01-15T10:30:00" shape); a count outside chrono's representable range
// falls back to the raw number.
fn duckdb_timestamp_to_iso(unit: duckdb::types::TimeUnit, ts: i64) -> String {
    let dt = match unit {
        duckdb::types::TimeUnit::Second => chrono::DateTime::from_timestamp(ts, 0),
        duckdb::types::TimeUnit::Millisecond => chrono::DateTime::from_timestamp_millis(ts),
        duckdb::types::TimeUnit::Microsecond => chrono::DateTime::from_timestamp_micros(ts),
        duckdb::types::TimeUnit::Nanosecond => Some(chrono::DateTime::from_timestamp_nanos(ts)),
    };
    dt.map(|dt| dt.naive_utc().format("%Y-%m-%dT%H:%M:%S%.f").to_string())
        .unwrap_or_else(|| ts.to_string())
}

// Same story for TIME: a raw count since midnight. None when out of range
// (caller falls back to the raw number).
fn duckdb_time_to_iso(unit: duckdb::types::TimeUnit, t: i64) -> Option<String> {
    let (secs, nanos) = match unit {
        duckdb::types::TimeUnit::Second => (t, 0),
        duckdb::types::TimeUnit::Millisecond => (t / 1_000, (t % 1_000) * 1_000_000),
        duckdb::types::TimeUnit::Microsecond => (t / 1_000_000, (t % 1_000_000) * 1_000),
        duckdb::types::TimeUnit::Nanosecond => (t / 1_000_000_000, t % 1_000_000_000),
    };
    chrono::NaiveTime::from_num_seconds_from_midnight_opt(
        u32::try_from(secs).ok()?,
        u32::try_from(nanos).ok()?,
    )
    .map(|t| t.format("%H:%M:%S%.f").to_string())
}

#[cfg(test)]
mod temporal_json_tests {
    use super::*;
    use duckdb::types::TimeUnit;

    #[test]
    fn timestamp_micros_renders_iso() {
        // 2026-07-01 23:13:42.218435 UTC
        assert_eq!(
            duckdb_timestamp_to_iso(TimeUnit::Microsecond, 1_782_947_622_218_435),
            "2026-07-01T23:13:42.218435"
        );
    }

    #[test]
    fn timestamp_seconds_renders_iso_without_subseconds() {
        assert_eq!(
            duckdb_timestamp_to_iso(TimeUnit::Second, 1_735_689_600),
            "2025-01-01T00:00:00"
        );
    }

    #[test]
    fn out_of_range_timestamp_falls_back_to_raw() {
        assert_eq!(
            duckdb_timestamp_to_iso(TimeUnit::Second, i64::MAX),
            i64::MAX.to_string()
        );
    }

    #[test]
    fn time_micros_renders_iso() {
        assert_eq!(
            duckdb_time_to_iso(TimeUnit::Microsecond, 37_800_500_000).as_deref(),
            Some("10:30:00.500")
        );
    }

    #[test]
    fn temporal_values_render_iso_through_real_query() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT TIMESTAMP '2026-07-01 23:13:42.218435' AS ts,
                        TIMESTAMPTZ '2026-07-01 23:13:42+00' AS tstz,
                        TIMESTAMP_NS '2026-07-01 23:13:42.218435678' AS ts_ns,
                        DATE '2026-07-01' AS d,
                        TIME '10:30:00' AS t",
            )
            .unwrap();
        let mut rows = stmt.query([]).unwrap();
        let row = rows.next().unwrap().unwrap();
        let json_of = |i: usize| {
            let v: duckdb::types::Value = row.get(i).unwrap();
            let mut budget = Budget { remaining: usize::MAX, limit: 0 };
            duckdb_value_to_json_value(v, &None, &mut budget).unwrap()
        };
        assert_eq!(json_of(0), serde_json::json!("2026-07-01T23:13:42.218435"));
        assert_eq!(json_of(1), serde_json::json!("2026-07-01T23:13:42"));
        assert_eq!(
            json_of(2),
            serde_json::json!("2026-07-01T23:13:42.218435678")
        );
        assert_eq!(json_of(3), serde_json::json!("2026-07-01"));
        assert_eq!(json_of(4), serde_json::json!("10:30:00"));
    }

    // Numbers too wide for a JSON number are rendered as strings. DECIMAL runs to
    // 38 digits and UHUGEINT to 2^128-1; rendering either through a type that
    // cannot hold it aborts the whole worker rather than erroring, because the
    // panic escapes an `extern "C"` frame and those cannot unwind.
    #[test]
    fn wide_numbers_render_as_strings_without_losing_precision() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT (-1.05)::DECIMAL(4, 2) AS neg,
                        (1.50)::DECIMAL(4, 2) AS trailing_zero,
                        '1234567890123456789012345678.9012345678'::DECIMAL(38, 10) AS wide,
                        '-9999999999999999999999999999.9999999999'::DECIMAL(38, 10) AS wide_neg,
                        '340282366920938463463374607431768211455'::UHUGEINT AS uhuge",
            )
            .unwrap();
        let mut rows = stmt.query([]).unwrap();
        let row = rows.next().unwrap().unwrap();
        let json_of = |i: usize| {
            let v: duckdb::types::Value = row.get(i).unwrap();
            let mut budget = Budget { remaining: usize::MAX, limit: 0 };
            duckdb_value_to_json_value(v, &None, &mut budget).unwrap()
        };
        assert_eq!(json_of(0), serde_json::json!("-1.05"));
        assert_eq!(json_of(1), serde_json::json!("1.50"));
        assert_eq!(
            json_of(2),
            serde_json::json!("1234567890123456789012345678.9012345678")
        );
        assert_eq!(
            json_of(3),
            serde_json::json!("-9999999999999999999999999999.9999999999")
        );
        assert_eq!(
            json_of(4),
            serde_json::json!("340282366920938463463374607431768211455")
        );
    }

    // The data-test sample probe shape emitted by
    // `windmill-parser::sql_materialize::build_data_test_checks`: one scan
    // yielding the violating-row count plus a bounded `to_json` sample of the
    // rows as a VARCHAR. These tests gate that design against the *bundled*
    // engine (json extension availability, row-as-struct alias reference,
    // NULL degrade on zero rows / oversized samples, exotic column types).
    fn sample_probe_sql(rows_query: &str, max_len: usize) -> String {
        format!(
            "SELECT v, CASE WHEN strlen(s_raw) <= {max_len} THEN s_raw END AS s \
             FROM (SELECT count(*) AS v, \
                   to_json(list(_wm_v ORDER BY _wm_rn) FILTER (WHERE _wm_rn <= 20))::VARCHAR AS s_raw \
                   FROM (SELECT _wm_v, row_number() OVER () AS _wm_rn FROM ({rows_query}) _wm_v))"
        )
    }

    fn run_sample_probe(
        conn: &duckdb::Connection,
        rows_query: &str,
        max_len: usize,
    ) -> (i64, Option<String>) {
        let mut stmt = conn
            .prepare(&sample_probe_sql(rows_query, max_len))
            .unwrap();
        let mut rows = stmt.query([]).unwrap();
        let row = rows.next().unwrap().unwrap();
        (row.get(0).unwrap(), row.get(1).unwrap())
    }

    #[test]
    fn data_test_sample_probe_counts_and_samples() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE t (id INT, name VARCHAR); \
             INSERT INTO t VALUES (1, 'a'), (2, NULL), (3, NULL);",
        )
        .unwrap();
        let (v, s) = run_sample_probe(&conn, "SELECT * FROM t WHERE name IS NULL", 51200);
        assert_eq!(v, 2);
        let parsed: serde_json::Value = serde_json::from_str(&s.unwrap()).unwrap();
        assert_eq!(
            parsed,
            serde_json::json!([{"id": 2, "name": null}, {"id": 3, "name": null}])
        );
    }

    #[test]
    fn data_test_sample_probe_zero_rows_yields_null_sample() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (id INT); INSERT INTO t VALUES (1);")
            .unwrap();
        let (v, s) = run_sample_probe(&conn, "SELECT * FROM t WHERE id IS NULL", 51200);
        assert_eq!(v, 0);
        assert!(s.is_none());
    }

    #[test]
    fn data_test_sample_probe_caps_at_20_rows_but_counts_all() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t AS SELECT range AS id FROM range(50);")
            .unwrap();
        let (v, s) = run_sample_probe(&conn, "SELECT * FROM t", 51200);
        assert_eq!(v, 50);
        let parsed: serde_json::Value = serde_json::from_str(&s.unwrap()).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 20);
    }

    #[test]
    fn data_test_sample_probe_oversized_sample_degrades_to_null() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t AS SELECT repeat('x', 1000) AS big FROM range(5);")
            .unwrap();
        let (v, s) = run_sample_probe(&conn, "SELECT * FROM t", 100);
        assert_eq!(v, 5);
        assert!(s.is_none());
    }

    #[test]
    fn data_test_sample_probe_codegen_row_query_shapes() {
        // The exact rows-query shapes emitted by `build_data_test_checks`:
        // unique's `{value, count}` grain and the star-EXCLUDE forms used on
        // partitioned targets (plain and `_wm_src.`-qualified).
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE t (id INT, name VARCHAR, _wm_partition VARCHAR); \
             INSERT INTO t VALUES (1, 'a', 'p'), (1, 'b', 'p'), (2, NULL, 'p');",
        )
        .unwrap();
        let (v, s) = run_sample_probe(
            &conn,
            "SELECT \"id\" AS \"value\", count(*) AS \"count\" FROM t \
             WHERE \"id\" IS NOT NULL GROUP BY \"id\" HAVING count(*) > 1",
            51200,
        );
        assert_eq!(v, 1);
        let parsed: serde_json::Value = serde_json::from_str(&s.unwrap()).unwrap();
        assert_eq!(parsed, serde_json::json!([{"value": 1, "count": 2}]));

        let (v, s) = run_sample_probe(
            &conn,
            "SELECT * EXCLUDE (\"_wm_partition\") FROM t WHERE name IS NULL",
            51200,
        );
        assert_eq!(v, 1);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&s.unwrap()).unwrap(),
            serde_json::json!([{"id": 2, "name": null}])
        );

        let (v, s) = run_sample_probe(
            &conn,
            "SELECT _wm_src.* EXCLUDE (\"_wm_partition\") FROM t _wm_src WHERE _wm_src.name IS NULL",
            51200,
        );
        assert_eq!(v, 1);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&s.unwrap()).unwrap(),
            serde_json::json!([{"id": 2, "name": null}])
        );
    }

    #[test]
    fn data_test_sample_probe_survives_exotic_types() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE t AS SELECT \
                INTERVAL 3 DAY AS iv, \
                12345678901234567890123456789::HUGEINT AS hi, \
                '\\xDE\\xAD'::BLOB AS bl, \
                [1, 2, 3] AS li, \
                {'a': 1, 'b': 'x'} AS st, \
                TIMESTAMPTZ '2026-07-01 23:13:42+00' AS tstz, \
                DECIMAL '12.34' AS dec;",
        )
        .unwrap();
        let (v, s) = run_sample_probe(&conn, "SELECT * FROM t", 51200);
        assert_eq!(v, 1);
        let parsed: serde_json::Value = serde_json::from_str(&s.unwrap()).unwrap();
        let row = &parsed.as_array().unwrap()[0];
        // Exact renderings are DuckDB's to_json choices — the gate is only
        // that every type serializes without error and parses back as JSON.
        assert!(row.get("iv").is_some());
        assert!(row.get("hi").is_some());
        assert!(row.get("bl").is_some());
        assert_eq!(row["li"], serde_json::json!([1, 2, 3]));
        assert_eq!(row["st"], serde_json::json!({"a": 1, "b": "x"}));
        assert!(row.get("tstz").is_some());
        assert!(row.get("dec").is_some());
    }
}

// Cross-engine parity for the `wm_partition` materialize macro. The macro is
// `strftime(ts, '<fmt>')` where `<fmt>` is single-sourced in windmill-parser's
// `PartitionKind::default_time_format` — the SAME format the EE resolver
// (`partition_ee.rs`) uses (via chrono) to stamp the `{partition}` identity.
// The macro only works if the *bundled* DuckDB engine renders that format
// byte-for-byte identically to chrono. Weekly `%G-W%V` (ISO year/week) is the
// one that can diverge at a year boundary, so it's exercised head-on here — a
// gap a pure-Rust unit test can't reach.
#[cfg(test)]
mod wm_partition_parity_tests {
    use super::*;

    // Mirror of windmill-parser `PartitionKind::default_time_format`.
    const FORMATS: &[&str] = &["%Y-%m-%d", "%Y-%m-%dT%H", "%G-W%V", "%Y-%m"];

    fn scalar_str(conn: &duckdb::Connection, sql: &str) -> String {
        let mut stmt = conn.prepare(sql).unwrap();
        let mut rows = stmt.query([]).unwrap();
        row_string(rows.next().unwrap().unwrap())
    }
    fn row_string(row: &Row) -> String {
        row.get::<usize, String>(0).unwrap()
    }

    #[test]
    fn duckdb_strftime_matches_chrono_for_every_grain_format() {
        use chrono::NaiveDate;
        let conn = duckdb::Connection::open_in_memory().unwrap();
        // A normal instant, two same-hour instants, and ISO-week year boundaries
        // where chrono's and DuckDB's ISO rules must agree: 2026-01-01 (Thu) is
        // in 2026-W01 and makes 2026 a 53-week ISO year, so 2027-01-01 (Fri)
        // belongs to 2026-W53; 2023-01-01 (Sun) → 2022-W52; 2021-01-03 (Sun) →
        // 2020-W53.
        let instants = [
            (2026, 7, 5, 23, 10, 0),
            (2026, 7, 5, 23, 55, 0),
            (2026, 1, 1, 0, 0, 0),
            (2027, 1, 1, 0, 0, 0),
            (2023, 1, 1, 0, 0, 0),
            (2021, 1, 3, 23, 59, 0),
            (2020, 12, 31, 12, 0, 0),
        ];
        for (y, mo, d, h, mi, s) in instants {
            let dt = NaiveDate::from_ymd_opt(y, mo, d)
                .unwrap()
                .and_hms_opt(h, mi, s)
                .unwrap();
            let ts_lit = dt.format("%Y-%m-%d %H:%M:%S").to_string();
            for fmt in FORMATS {
                let chrono_val = dt.format(fmt).to_string();
                let duck = scalar_str(
                    &conn,
                    &format!("SELECT strftime(TIMESTAMP '{ts_lit}', '{fmt}')"),
                );
                assert_eq!(
                    duck, chrono_val,
                    "DuckDB strftime disagrees with chrono for {ts_lit} / {fmt}"
                );
            }
        }
        // Explicit year-boundary pin: guards against BOTH engines drifting the
        // same way (2027-01-01 is ISO week 53 of 2026).
        assert_eq!(
            scalar_str(&conn, "SELECT strftime(TIMESTAMP '2027-01-01', '%G-W%V')"),
            "2027-01-01"
                .parse::<chrono::NaiveDate>()
                .unwrap()
                .format("%G-W%V")
                .to_string()
        );
        assert_eq!(
            scalar_str(&conn, "SELECT strftime(TIMESTAMP '2027-01-01', '%G-W%V')"),
            "2026-W53"
        );
    }

    #[test]
    fn temp_macro_is_classified_as_setup() {
        // The macro the executor injects, plus whitespace/keyword variants the
        // workspace-macro splicer can emit.
        assert!(is_setup_statement(
            "CREATE OR REPLACE TEMP MACRO wm_partition(ts) AS strftime(ts, '%Y-%m-%dT%H')"
        ));
        assert!(is_setup_statement("  create temp macro foo(x) as x + 1"));
        assert!(is_setup_statement("CREATE TEMPORARY MACRO foo(x) AS x"));
        assert!(is_setup_statement(
            "CREATE OR REPLACE\n  TEMPORARY MACRO foo(x) AS x"
        ));
        // Persistent (non-TEMP) macros are real user statements, not setup.
        assert!(!is_setup_statement(
            "CREATE OR REPLACE MACRO safe_div(a, b) AS a / b"
        ));
        assert!(!is_setup_statement("CREATE MACRO foo(x) AS x"));
    }

    #[test]
    fn prepare_pass_binds_consumer_only_when_macro_runs_as_setup() {
        // Mirror prepare_duckdb_internal's per-block contract: setup statements
        // are EXECUTED, everything else is only prepared. A block that calls
        // wm_partition binds at prepare time, so it resolves only if the macro
        // block already ran as setup.
        let consumer = "SELECT wm_partition(TIMESTAMP '2026-07-05 23:10:00') AS p";
        let macro_stmt =
            "CREATE OR REPLACE TEMP MACRO wm_partition(ts) AS strftime(ts, '%Y-%m-%dT%H')";

        let conn = duckdb::Connection::open_in_memory().unwrap();
        for block in [macro_stmt, consumer] {
            if is_setup_statement(block) {
                conn.execute_batch(block).unwrap();
            } else {
                conn.prepare(block)
                    .expect("consumer must prepare once the macro ran as setup");
            }
        }

        // Guard: if the macro were NOT setup (the bug), the consumer's prepare
        // fails "function does not exist" — proving the classification matters.
        let fresh = duckdb::Connection::open_in_memory().unwrap();
        assert!(
            fresh.prepare(consumer).is_err(),
            "consumer must fail to prepare when wm_partition was never created"
        );
    }

    #[test]
    fn wm_partition_macro_buckets_whole_slice_and_naive_cast_errors() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        // Exactly the statement the executor injects for an hourly materialize.
        conn.execute_batch(
            "CREATE OR REPLACE TEMP MACRO wm_partition(ts) AS strftime(ts, '%Y-%m-%dT%H');",
        )
        .unwrap();
        conn.execute_batch(
            "CREATE TABLE t(ts TIMESTAMP); INSERT INTO t VALUES \
             (TIMESTAMP '2026-07-05 23:10:00'), (TIMESTAMP '2026-07-05 23:55:00'), \
             (TIMESTAMP '2026-07-05 22:30:00'), (TIMESTAMP '2026-07-06 00:05:00');",
        )
        .unwrap();
        // Filtering with the injected macro against the identity string selects
        // the WHOLE hour bucket (both 23:10 and 23:55) — not just the boundary
        // instant a `= TIMESTAMP {partition}` cast could ever match.
        let n = scalar_str(
            &conn,
            "SELECT count(*)::VARCHAR FROM t WHERE wm_partition(ts) = '2026-07-05T23'",
        );
        assert_eq!(n, "2");
        // The footgun the macro exists to avoid: the weekly/monthly identity
        // strings are not valid TIMESTAMP literals, so the naive cast errors.
        assert!(
            conn.execute_batch("SELECT TIMESTAMP '2026-W27';").is_err(),
            "expected `TIMESTAMP '2026-W27'` to be a Conversion Error"
        );
        assert!(
            conn.execute_batch("SELECT TIMESTAMP '2026-07';").is_err(),
            "expected `TIMESTAMP '2026-07'` to be a Conversion Error"
        );
    }
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
                        .ok_or_else(|| "Could not convert f64 to Decimal".to_string())?
                        .into(),
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

// The bundled engine is not stock (docs/duckdb-isolation.md). These guard the ways an engine
// bump can drop the patches without anything else noticing.
#[cfg(test)]
mod patched_engine_tests {
    use super::*;

    /// Too large to be answered within the `memory_limit` `spill_attempt` sets, so the engine
    /// has to reach the temp directory to finish it.
    const SPILLING_QUERY: &str = "CREATE TABLE t AS SELECT * FROM range(1000000)";

    /// Every connection is locked at setup, so no script statement can move the temp
    /// directory onto a remote filesystem — where the spill fails and the
    /// `~TemporaryDirectoryHandle` cleaning up throws out of a `noexcept` destructor,
    /// aborting the worker and every job colocated on it.
    #[test]
    fn configured_connections_cannot_have_their_temp_directory_moved() {
        let temp_dir = std::env::temp_dir().join(format!(
            "windmill_duckdb_locked_{}_{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let conn = duckdb::Connection::open_in_memory().unwrap();
        configure_duckdb_resource_limits(
            &conn,
            &ResourceLimits {
                memory_limit: Some("2MB".to_string()),
                temp_directory: Some(temp_dir.display().to_string()),
                max_temp_directory_size: None,
            },
        )
        .unwrap();

        for stmt in [
            "SET temp_directory='s3://bucket/tmp'",
            "SET temp_directory TO 's3://bucket/tmp'",
            "PRAGMA temp_directory='s3://bucket/tmp'",
            "RESET temp_directory",
        ] {
            let err = conn
                .execute_batch(stmt)
                .expect_err(&format!("engine accepted `{stmt}` on a locked connection"));
            assert!(
                err.to_string().contains("temp directory has been locked"),
                "unexpected refusal for `{stmt}`: {err}"
            );
        }

        // The lock must not cost the spilling it exists to protect.
        let spilled = conn.execute_batch(SPILLING_QUERY);
        std::fs::remove_dir_all(&temp_dir).ok();
        spilled.expect("a locked temp directory must still be usable for spilling");
    }

    /// `DUCKDB_MAX_TEMP_DIRECTORY_SIZE` is the only thing standing between one query and the
    /// worker's whole disk, so pin that a configured connection really does stop at it, names
    /// the setting when it does, and leaves nothing behind for the next job to trip over.
    #[test]
    fn spilling_past_the_configured_cap_fails_the_query_and_cleans_up() {
        let temp_dir = std::env::temp_dir().join(format!(
            "windmill_duckdb_capped_{}_{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let spill_under_cap = |cap: &str| {
            let conn = duckdb::Connection::open_in_memory().unwrap();
            configure_duckdb_resource_limits(
                &conn,
                &ResourceLimits {
                    memory_limit: Some("2MB".to_string()),
                    temp_directory: Some(temp_dir.display().to_string()),
                    max_temp_directory_size: Some(cap.to_string()),
                },
            )
            .unwrap();
            conn.execute_batch(SPILLING_QUERY)
                .map_err(|e| e.to_string())
        };

        let err = spill_under_cap("1MB").expect_err("the cap did not stop the spill");
        // Verbatim because the worker matches on it (`SPILL_CAP_ENGINE_MARKER` in
        // `duckdb_executor.rs`) to correct the engine's advice to raise the cap, which the
        // lock refuses. An engine bump that rewords this has to update that too.
        assert!(
            err.contains("This limit was set by the 'max_temp_directory_size' setting"),
            "the refusal must name the setting an operator can act on, got: {err}"
        );
        let leftover: Vec<_> = std::fs::read_dir(&temp_dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        assert!(
            leftover.is_empty(),
            "a query killed by the cap must not leave its spill behind: {leftover:?}"
        );

        // DuckDB compresses temp files, so the failure above is only evidence of the cap if
        // the same query gets through once the cap is out of the way.
        let generous = spill_under_cap("1GB");
        std::fs::remove_dir_all(&temp_dir).ok();
        generous.expect("the query must still spill and finish under a cap that fits it");
    }

    fn spill_attempt(temp_dir: &std::path::Path, lock: bool) -> Result<(), String> {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!(
            "SET threads=2; SET memory_limit='2MB'; SET temp_directory='{}';",
            temp_dir.display()
        ))
        .unwrap();
        if lock {
            conn.execute_batch("SET lock_temp_directory=true;")
                .map_err(|e| format!("lock_temp_directory rejected: {e}"))?;
        }
        conn.execute_batch("SET disabled_filesystems='LocalFileSystem';")
            .unwrap();
        conn.execute_batch(SPILLING_QUERY)
            .map_err(|e| e.to_string())
    }

    #[test]
    fn locking_the_temp_directory_keeps_spilling_available() {
        let temp_dir = std::env::temp_dir().join(format!(
            "windmill_duckdb_spill_{}_{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Without the lock the fence reaches the buffer manager too, which is both the
        // stock engine's behaviour and the proof that this query really does spill —
        // otherwise the assertion below would pass on an unpatched engine.
        let unlocked = spill_attempt(&temp_dir, false).expect_err("query did not need to spill");
        assert!(
            unlocked.contains("LocalFileSystem has been disabled"),
            "expected the fence to stop the spill, got: {unlocked}"
        );

        let locked = spill_attempt(&temp_dir, true);
        std::fs::remove_dir_all(&temp_dir).ok();
        locked.expect("a locked temp directory must stay usable behind disabled_filesystems");
    }

    /// The exemption above is only sound while neither the spill's destination nor its size
    /// can be changed: repointing the directory is a write primitive through the very fence
    /// it is exempt from, and raising the cap leaves the disk unbounded. A rebase can drop
    /// either half while leaving the spill test green, so pin both.
    #[test]
    fn locking_the_temp_directory_makes_it_immutable() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch("SET temp_directory='/tmp/windmill_duckdb_lock_probe';")
            .unwrap();
        conn.execute_batch("SET lock_temp_directory=true;")
            .expect("the patched engine must accept lock_temp_directory");

        for stmt in [
            "SET temp_directory='/tmp/windmill_duckdb_elsewhere';",
            "RESET temp_directory;",
            "SET max_temp_directory_size='100GB';",
            "RESET max_temp_directory_size;",
            "SET lock_temp_directory=false;",
            "RESET lock_temp_directory;",
        ] {
            let err = conn.execute_batch(stmt).expect_err(&format!(
                "`{stmt}` must be refused once the directory is locked"
            ));
            assert!(
                err.to_string().contains("locked"),
                "expected a lock refusal for `{stmt}`, got: {err}"
            );
        }
    }

    // The engine loads the *prebuilt* extensions from extensions.duckdb.org, which are
    // compiled against the struct layouts of the release it claims to be. A patch that
    // moves a member those extensions reach does not fail — the extension reads the
    // neighbouring field and LOAD blocks forever — so this is a timeout, not an assertion.
    #[test]
    fn prebuilt_extensions_still_load() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        // Fetching the extension needs egress, which loading it back — the part this guards —
        // does not. Skipping keeps a developer offline from seeing a failure they can't act on,
        // but never in CI, where a skip and a pass would be indistinguishable.
        if let Err(e) = conn.execute_batch("INSTALL httpfs;") {
            assert!(
                std::env::var_os("CI").is_none(),
                "could not install httpfs, so the extension-load guard did not run: {e}"
            );
            eprintln!("skipping: INSTALL httpfs failed ({e})");
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            tx.send(
                conn.execute_batch("LOAD httpfs;")
                    .map_err(|e| e.to_string()),
            )
        });
        match rx.recv_timeout(std::time::Duration::from_secs(120)) {
            Ok(r) => r.expect("could not load httpfs"),
            Err(_) => panic!(
                "LOAD httpfs did not return: the patched engine's struct layout no longer matches \
                 the release the prebuilt extensions were built against"
            ),
        }
    }
}
