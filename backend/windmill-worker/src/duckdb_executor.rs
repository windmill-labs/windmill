use std::cell::RefCell;
use std::env;
use std::ffi::{c_char, c_uint, CStr, CString};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use libloading::{Library, Symbol};
use serde::Serialize;
use serde_json::value::RawValue;
use serde_json::{json, Value};
use uuid::Uuid;
use windmill_common::error::{to_anyhow, Error, Result};
use windmill_common::s3_helpers::{S3Object, S3_PROXY_LAST_ERRORS_CACHE};
use windmill_common::utils::sanitize_string_from_password;
use windmill_common::worker::{Connection, SqlResultCollectionStrategy};
use windmill_common::workspaces::{get_ducklake_from_db_unchecked, DucklakeCatalogResourceType};
use windmill_parser_sql::{parse_duckdb_sig, parse_sql_blocks};
use windmill_queue::{CanceledBy, MiniPulledJob};

use crate::agent_workers::get_ducklake_from_agent_http;
use crate::common::{build_args_values, get_reserved_variables, OccupancyMetrics};
use crate::handle_child::run_future_with_polling_update_job_poller;
#[cfg(feature = "mysql")]
use crate::mysql_executor::MysqlDatabase;
use crate::pg_executor::PgDatabase;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use windmill_common::client::AuthedClient;

pub async fn do_duckdb(
    job: &MiniPulledJob,
    client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    // TODO
    #[allow(unused_variables)] column_order_ref: &mut Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
    parent_runnable_path: Option<String>,
) -> Result<Box<RawValue>> {
    let annotations = windmill_common::worker::SqlAnnotations::parse(query);
    let collection_strategy =
        if annotations.result_collection == SqlResultCollectionStrategy::Legacy {
            // Before result_collection was introduced, duckdb ignored all statements results except the last one
            SqlResultCollectionStrategy::LastStatementAllRows
        } else {
            annotations.result_collection
        };
    if annotations.return_last_result {
        return Err(Error::ExecutionErr(
            "return_last_result annotation is deprecated, use result_collection=last_statement_all_rows instead".to_string(),
        ));
    }

    let token = client.token.clone();
    let hidden_passwords = Arc::new(Mutex::new(Vec::<String>::new()));

    let result_f = async {
        let mut hidden_passwords = hidden_passwords.clone();
        let mut bigquery_credentials = None;

        let sig = parse_duckdb_sig(query)?.args;
        let mut job_args = build_args_values(job, client, conn).await?;

        let reserved_variables =
            get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

        let (query, _) =
            &sanitize_and_interpolate_unsafe_sql_args(query, &sig, &job_args, &reserved_variables)?;
        let query = transform_s3_uris(query).await?;

        let job_args = {
            let mut m = Vec::new();
            for sig_arg in sig.into_iter() {
                let json_value = job_args
                    .remove(&sig_arg.name)
                    .or_else(|| sig_arg.default)
                    .unwrap_or_else(|| json!(null));

                if matches!(&sig_arg.otyp.as_ref().map(String::as_str), Some("s3object")) {
                    let s3_obj = serde_json::from_value::<S3Object>(json_value).map_err(|e| {
                        Error::ExecutionErr(format!("Failed to deserialize S3Object: {}", e))
                    })?;
                    let uri = format!(
                        "s3://{}/{}",
                        s3_obj.storage.as_deref().unwrap_or("_default_"),
                        s3_obj.s3
                    );
                    m.push(Arg {
                        json_value: serde_json::Value::String(uri),
                        name: sig_arg.name,
                        arg_type: "string".to_string(),
                    });
                } else {
                    m.push(Arg {
                        json_value,
                        name: sig_arg.name,
                        arg_type: sig_arg.otyp.unwrap_or_else(|| "text".to_string()),
                    });
                }
            }
            m
        };

        let query_block_list = parse_sql_blocks(&query);

        // Replace windmill resource ATTACH statements with the real instructions
        let query_block_list = {
            let mut v = vec![];
            for query_block in query_block_list.iter() {
                let query_block = remove_comments(&query_block);
                match parse_attach_db_resource(query_block) {
                    Some(parsed) => {
                        v.extend(
                            transform_attach_db_resource_query(
                                &parsed,
                                &job.id,
                                client,
                                &mut hidden_passwords,
                            )
                            .await?,
                        );
                        if parsed.db_type == "bigquery" {
                            bigquery_credentials = Some(UseBigQueryCredentialsFile::new(
                                job.id,
                                parsed.resource_path,
                            )?);
                        }
                    }
                    None => match transform_attach_ducklake(
                        &query_block,
                        conn,
                        &mut hidden_passwords,
                        &job.workspace_id,
                    )
                    .await?
                    {
                        Some(ducklake_query) => v.extend(ducklake_query),
                        None => v.push(query_block.to_string()),
                    },
                };
            }
            v
        };

        let base_internal_url = client.base_internal_url.clone();
        let w_id = job.workspace_id.clone();

        let result = tokio::task::spawn_blocking(move || {
            run_duckdb_ffi_safe(
                query_block_list.iter().map(String::as_str),
                query_block_list.len(),
                job_args,
                &token,
                &base_internal_url,
                &w_id,
                collection_strategy,
            )
        })
        .await
        .map_err(|e| Error::from(to_anyhow(e)))
        .and_then(|r| r);

        let (result, column_order) = match result {
            Ok(r) => r,
            Err(e) => {
                if let Some(s3_proxy_err) = S3_PROXY_LAST_ERRORS_CACHE.get(&client.token) {
                    return Err(Error::ExecutionErr(format!(
                        "{}\n\nS3 Related Error: {}",
                        e.to_string(),
                        s3_proxy_err,
                    )));
                }
                return Err(e);
            }
        };

        drop(bigquery_credentials);

        *column_order_ref = column_order;
        Ok(result)
    };

    let result = run_future_with_polling_update_job_poller(
        job.id,
        job.timeout,
        conn,
        mem_peak,
        canceled_by,
        result_f,
        worker_name,
        &job.workspace_id,
        &mut Some(occupancy_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await;

    match result {
        Ok(result) => Ok(result),
        Err(e) => {
            // Passwords might appear in the error message
            let mut err_str = e.to_string();
            for pwd in hidden_passwords.lock().unwrap().iter() {
                if let Some(sanitized) = sanitize_string_from_password(&err_str, &pwd.clone()) {
                    err_str = sanitized;
                }
            }
            Err(Error::ExecutionErr(err_str))
        }
    }
}

thread_local! {
    static DUCKDB_FFI_LIB_SINGLETON: RefCell<*const DuckDbFfiLib> = RefCell::new(std::ptr::null());
}

struct DuckDbFfiLib {
    run_duckdb_ffi: Symbol<
        'static,
        unsafe extern "C" fn(
            query_block_list: *const *const c_char,
            query_block_list_count: usize,
            job_args: *const c_char,
            token: *const c_char,
            base_internal_url: *const c_char,
            w_id: *const c_char,
            column_order_ptr: *mut *mut c_char,
            collect_last_only: bool,
            collect_first_row_only: bool,
        ) -> *mut c_char,
    >,
    free_cstr: Symbol<'static, unsafe extern "C" fn(string: *mut c_char) -> ()>,
}

impl DuckDbFfiLib {
    fn get_singleton() -> Result<&'static DuckDbFfiLib> {
        DUCKDB_FFI_LIB_SINGLETON.with(|cell| unsafe {
            let mut singleton = cell.borrow_mut();
            if singleton.is_null() {
                let lib = DuckDbFfiLib::init()?;
                let boxed_lib = Box::new(lib);
                let lib_ptr = Box::leak(boxed_lib);
                *singleton = lib_ptr as *const _;
                Ok(NonNull::new_unchecked(*singleton as *mut DuckDbFfiLib).as_ref())
            } else {
                Ok(&**singleton)
            }
        })
    }
    fn init() -> Result<Self> {
        let lib = unsafe {
            Library::new(if cfg!(target_os = "macos") {
                "libwindmill_duckdb_ffi_internal.dylib"
            } else if cfg!(target_os = "windows") {
                "windmill_duckdb_ffi_internal.dll"
            } else {
                "libwindmill_duckdb_ffi_internal.so"
            })
            .map_err(|e| {
                Error::InternalErr(format!(
                    "Could not init duckdb. Make sure you have the latest windmill_duckdb_ffi_lib.{} alongside your binary : https://github.com/windmill-labs/windmill/releases \n{}",
                    if cfg!(target_os = "macos") { "dylib" }
                        else if cfg!(target_os = "windows") { "dll" }
                        else { "so" },
                    e.to_string()
                ))
            })?
        };

        let lib = Box::leak(Box::new(lib));

        // Version mismatch should only be possible on Windows agent workers
        // We check for it because FFI interface mismatch will cause undefined behavior / crashes
        unsafe {
            let expected_version: c_uint = 1;
            let get_version: Symbol<'static, unsafe extern "C" fn() -> c_uint> = 
            lib.get(b"get_version")
                .map_err(|e| return Error::ExecutionErr(format!("Could not find get_version in the duckdb ffi library. If you are not using docker, consider manually upgrading windmill_duckdb_ffi_lib. {}", e.to_string())))?;
            let actual_version = get_version();
            if actual_version < expected_version {
                return Err(Error::InternalErr(
                    format!("Incompatible duckdb ffi library version. Expected: {expected_version}, actual: {actual_version}. Please update to the latest windmill_duckdb_ffi_lib."),
                ));
            } else if actual_version > expected_version {
                return Err(Error::InternalErr(
                    format!("Incompatible duckdb ffi library version. Expected: {expected_version}, actual: {actual_version}. Please upgrade your worker to the latest windmill version."),
                ));
            }
        }

        Ok(DuckDbFfiLib {
            run_duckdb_ffi: unsafe { lib.get(b"run_duckdb_ffi").map_err(to_anyhow)? },
            free_cstr: unsafe { lib.get(b"free_cstr").map_err(to_anyhow)? },
        })
    }
}

// Read backend/windmill-duckdb-ffi-internal/README_DEV.md for details about why we use FFI
fn run_duckdb_ffi_safe<'a>(
    query_block_list: impl Iterator<Item = &'a str>,
    query_block_list_count: usize,
    job_args: Vec<Arg>,
    token: &str,
    base_internal_url: &str,
    w_id: &str,
    collection_strategy: SqlResultCollectionStrategy,
) -> Result<(Box<RawValue>, Option<Vec<String>>)> {
    let query_block_list = query_block_list
        .map(|s| {
            CString::new(s).map_err(|e| {
                Error::ExecutionErr(format!("Failed CString conversion: {}", e.to_string()))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let query_block_list = query_block_list
        .iter()
        .map(|s| s.as_ptr())
        .collect::<Vec<_>>();
    let job_args = serde_json::to_string(&job_args).map_err(to_anyhow)?;

    let job_args = CString::new(job_args).map_err(to_anyhow)?;
    let token = CString::new(token).map_err(to_anyhow)?;
    let base_internal_url = CString::new(base_internal_url).map_err(to_anyhow)?;
    let w_id = CString::new(w_id).map_err(to_anyhow)?;

    let run_duckdb_ffi = &DuckDbFfiLib::get_singleton()?.run_duckdb_ffi;
    let free_cstr = &DuckDbFfiLib::get_singleton()?.free_cstr;
    let mut column_order: *mut c_char = std::ptr::null_mut();
    let result_str = unsafe {
        let ptr = run_duckdb_ffi(
            query_block_list.as_ptr(),
            query_block_list_count,
            job_args.as_ptr(),
            token.as_ptr(),
            base_internal_url.as_ptr(),
            w_id.as_ptr(),
            &mut column_order,
            collection_strategy.collect_last_statement_only(query_block_list_count),
            collection_strategy.collect_first_row_only(),
        );
        let str = CStr::from_ptr(ptr).to_string_lossy().to_string();
        free_cstr(ptr);
        str
    };

    let column_order = if column_order.is_null()
        || !collection_strategy.collect_last_statement_only(query_block_list_count)
        || collection_strategy.collect_scalar()
    {
        None
    } else {
        let str = unsafe { CStr::from_ptr(column_order).to_string_lossy().to_string() };
        unsafe { free_cstr(column_order) };
        Some(serde_json::from_str::<Vec<String>>(&str)?)
    };

    if result_str.starts_with("ERROR") {
        Err(Error::ExecutionErr(result_str[6..].to_string()))
    } else {
        let result = if collection_strategy == SqlResultCollectionStrategy::AllStatementsAllRows {
            // Avoid parsing JSON
            serde_json::value::RawValue::from_string(result_str).map_err(to_anyhow)?
        } else {
            let result =
                serde_json::from_str::<Vec<Vec<Box<RawValue>>>>(&result_str).map_err(to_anyhow)?;
            collection_strategy.collect(result)?
        };
        Ok((result, column_order))
    }
}

struct ParsedAttachDbResource<'a> {
    resource_path: &'a str,
    name: &'a str,
    db_type: &'a str,
    extra_args: Option<&'a str>,
}
fn parse_attach_db_resource<'a>(query: &'a str) -> Option<ParsedAttachDbResource<'a>> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"(?i)ATTACH '(\$res:|res://)([^']+)' AS (\S+) \(TYPE (\w+)(.*)\)").unwrap();
    }

    for cap in RE.captures_iter(query) {
        if let (Some(resource_path), Some(name), Some(db_type)) =
            (cap.get(2), cap.get(3), cap.get(4))
        {
            let extra_args = cap.get(5).map(|m| query[m.start()..m.end()].trim());
            return Some(ParsedAttachDbResource {
                resource_path: query[resource_path.start()..resource_path.end()].trim(),
                name: query[name.start()..name.end()].trim(),
                db_type: query[db_type.start()..db_type.end()].trim(),
                extra_args,
            });
        }
    }
    None
}

fn format_attach_db_conn_str(db_resource: Value, db_type: &str) -> Result<String> {
    let s = match db_type.to_lowercase().as_str() {
        "postgres" | "postgresql" => {
            let res: PgDatabase = serde_json::from_value(db_resource)?;
            format!(
                "dbname={} {} host={} {} {} {}",
                res.dbname,
                res.user.map(|u| format!("user={}", u)).unwrap_or_default(),
                res.host,
                res.password
                    .map(|p| format!("password={}", p))
                    .unwrap_or_default(),
                res.port.map(|p| format!("port={}", p)).unwrap_or_default(),
                res.sslmode
                    .map(|s| format!("sslmode={}", s))
                    .unwrap_or_default(),
            )
        }
        #[cfg(feature = "mysql")]
        "mysql" => {
            let resource: MysqlDatabase = serde_json::from_value(db_resource)?;
            format!(
                "database={} host={} ssl_mode={} {} {} {}",
                resource.database,
                resource.host,
                resource
                    .ssl
                    .map(|ssl| if ssl { "required" } else { "disabled" })
                    .unwrap_or("preferred"),
                resource
                    .password
                    .map(|p| format!("password={}", p))
                    .unwrap_or_default(),
                resource
                    .port
                    .map(|p| format!("port={}", p))
                    .unwrap_or_default(),
                resource
                    .user
                    .map(|u| format!("user={}", u))
                    .unwrap_or_default(),
            )
        }
        "bigquery" => {
            let project_id: String = serde_json::from_value(
                db_resource
                    .get("project_id")
                    .ok_or_else(|| {
                        Error::ExecutionErr("BigQuery resource must contain project_id".to_string())
                    })?
                    .to_owned(),
            )
            .map_err(|_e| Error::ExecutionErr("failed project_id deserialize".to_string()))?;
            format!("project={}", project_id,)
        }
        _ => {
            return Err(Error::ExecutionErr(format!(
                "Unsupported db type in DuckDB ATTACH: {db_type}",
            )))
        }
    };
    Ok(s)
}

fn get_attach_db_install_str(db_type: &str) -> Result<&str> {
    match db_type.to_lowercase().as_str() {
        "postgres" => Ok("INSTALL postgres;"),
        "mysql" => {
            #[cfg(not(feature = "mysql"))]
            return Err(Error::ExecutionErr(
                "MySQL feature is not enabled".to_string(),
            ));
            #[cfg(feature = "mysql")]
            Ok("INSTALL mysql;")
        }
        "bigquery" => Ok("INSTALL bigquery FROM community;"),
        _ => Err(Error::ExecutionErr(format!(
            "Unsupported db type in DuckDB ATTACH: {}",
            db_type
        ))),
    }
}

async fn transform_attach_db_resource_query(
    parsed: &ParsedAttachDbResource<'_>,
    job_id: &Uuid,
    client: &AuthedClient,
    hidden_passwords: &mut Arc<Mutex<Vec<String>>>,
) -> Result<Vec<String>> {
    let db_resource: Value = client
        .get_resource_value_interpolated(parsed.resource_path, Some(job_id.to_string()))
        .await?;
    if let Some(pwd) = db_resource.get("password").and_then(|p| p.as_str()) {
        hidden_passwords.lock().unwrap().push(pwd.to_string());
    }
    let attach_str = format!(
        "ATTACH '{}' as {} (TYPE {}{});",
        format_attach_db_conn_str(db_resource, parsed.db_type)?,
        parsed.name,
        parsed.db_type,
        parsed.extra_args.unwrap_or("")
    )
    .to_string();

    Ok(vec![
        get_attach_db_install_str(parsed.db_type)?.to_string(),
        format!("LOAD {};", parsed.db_type),
        attach_str,
    ])
}

async fn transform_attach_ducklake(
    query: &str,
    conn: &Connection,
    hidden_passwords: &mut Arc<Mutex<Vec<String>>>,
    w_id: &str,
) -> Result<Option<Vec<String>>> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"(?i)ATTACH\s*'ducklake(://[^':]+)?'\s*AS\s+([^ ;]+)\s*(\([^)]*\))?").unwrap();
    }
    let Some(cap) = RE.captures(query) else {
        return Ok(None);
    };
    let name = cap.get(1).map(|m| &m.as_str()[3..]).unwrap_or("main");
    let alias_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
    let extra_args = cap
        .get(3)
        .map(|m| format!(", {}", &m.as_str()[1..m.as_str().len() - 1]))
        .unwrap_or("".to_string());

    let ducklake = match conn {
        Connection::Http(client) => get_ducklake_from_agent_http(client, name, w_id).await?,
        Connection::Sql(db) => get_ducklake_from_db_unchecked(name, w_id, db).await?,
    };
    let db_type = match ducklake.catalog.resource_type {
        DucklakeCatalogResourceType::Instance => "postgres",
        _ => ducklake.catalog.resource_type.as_ref(),
    };

    if let Some(pwd) = ducklake
        .catalog_resource
        .get("password")
        .and_then(|p| p.as_str())
    {
        hidden_passwords.lock().unwrap().push(pwd.to_string());
    }

    let db_conn_str = format_attach_db_conn_str(ducklake.catalog_resource, db_type)?;
    let storage = ducklake.storage.storage.as_deref().unwrap_or("_default_");
    let data_path = ducklake.storage.path;

    // Ducklake 0.3 only requires DATA_PATH at creation and then stores it internally in the catalog
    // But it will fail if DATA_PATH changes afterwards which is annoying for us
    // So we always enable override
    let extra_args = if extra_args.contains("OVERRIDE_DATA_PATH") {
        extra_args
    } else {
        format!(", OVERRIDE_DATA_PATH TRUE{extra_args}")
    };

    let attach_str = format!(
        "ATTACH 'ducklake:{db_type}:{db_conn_str}' AS {alias_name} (DATA_PATH 's3://{storage}/{data_path}'{extra_args});",
    );

    let install_db_ext_str = get_attach_db_install_str(db_type)?;
    Ok(Some(vec![
        "INSTALL ducklake;".to_string(),
        install_db_ext_str.to_string(),
        attach_str,
    ]))
}

async fn transform_s3_uris(query: &str) -> Result<String> {
    let mut transformed_query = None;
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"'s3://([^'/]*)/([^']*)'").unwrap();
    }
    for cap in RE.captures_iter(query) {
        if let (storage, Some(s3_path)) = (cap.get(1), cap.get(2)) {
            let s3_path = s3_path.as_str();
            let mut storage = storage.map(|m| m.as_str()).unwrap_or("");
            if !storage.is_empty() {
                continue;
            }
            let original_str_lit: String = format!("'s3://{}/{}'", storage, s3_path);
            storage = "_default_";

            let new_s3_lit = format!("'s3://{}/{}'", storage, s3_path);
            transformed_query = Some(
                transformed_query
                    .unwrap_or_else(|| query.to_string())
                    .replace(&original_str_lit, &new_s3_lit),
            );
        }
    }
    Ok(transformed_query.unwrap_or(query.to_string()))
}

// BigQuery extension requires a json file as credentials
// The file path is set as an env var by do_duckdb
// It is created by transform_attach_db_resource_query (when bigquery is detected)
// and deleted by do_duckdb after the query is executed
pub struct UseBigQueryCredentialsFile {
    path: String,
}
impl UseBigQueryCredentialsFile {
    fn new(job_id: Uuid, bigquery_resource: &str) -> Result<Self> {
        let path = format!("/tmp/service-account-credentials-{}.json", job_id);
        env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &path);
        std::fs::write(&path, bigquery_resource)
            .map_err(|e| Error::ExecutionErr(format!("Failed to write BigQuery creds: {e}")))?;
        Ok(Self { path })
    }
}
impl Drop for UseBigQueryCredentialsFile {
    fn drop(&mut self) {
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
        if matches!(std::fs::exists(&self.path), Ok(true)) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

// Shared with ffi module
#[derive(Serialize, Clone, Debug, PartialEq, Default)]
pub struct Arg {
    pub name: String,
    pub arg_type: String,
    pub json_value: serde_json::Value,
}

// input should contain a single statement. remove all comments before and after it
fn remove_comments(stmt: &str) -> &str {
    let mut in_stmt = false;
    let mut in_comment = false;
    let mut start = None;
    let mut end = stmt.len();

    let mut c = ' ';
    for (next_i, next_char) in stmt.char_indices() {
        if next_i > 0 {
            let i = next_i - 1;
            if !in_comment && in_stmt && c == ';' {
                end = i + 1;
                break;
            } else if in_comment && c == '\n' {
                in_comment = false;
            } else if c == '-' && next_char == '-' {
                in_comment = true;
            } else if !in_comment && !c.is_whitespace() && start == None {
                start = Some(i);
                in_stmt = true;
            }
        }
        c = next_char;
    }

    return &stmt[start.unwrap_or(0)..end];
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_remove_comments_single_line() {
        let sql = "-- This is a comment\nSELECT * FROM table;";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_multi_line() {
        let sql = "-- This is a comment\nSELECT * FROM table;\n-- Another comment";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_inline_comment() {
        let sql = "   SELECT * FROM table;    -- This is an inline comment  ";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_no_comments() {
        let sql = "SELECT * FROM table;";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_empty_string() {
        let sql = "";
        assert_eq!(remove_comments(sql), "");
    }
    #[test]
    fn test_remove_comments_with_whitespace() {
        let sql = "   -- Comment\n  -- Comment2\n  -- Comment3\n   SELECT\n\n * FROM\n table\n;\n\n -- end comment   ";
        assert_eq!(remove_comments(sql), "SELECT\n\n * FROM\n table\n;");
    }
}
