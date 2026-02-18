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
use windmill_types::s3::S3Object;
use windmill_object_store::S3_PROXY_LAST_ERRORS_CACHE;
use windmill_common::utils::sanitize_string_from_password;
use windmill_common::worker::{Connection, SqlResultCollectionStrategy};
use windmill_common::workspaces::{
    get_datatable_resource_from_db_unchecked, get_ducklake_from_db_unchecked,
    DucklakeCatalogResourceType,
};
use windmill_common::PgDatabase;
use windmill_parser_sql::{parse_duckdb_sig, parse_sql_blocks};
use windmill_queue::{CanceledBy, MiniPulledJob};

use crate::agent_workers::{get_datatable_resource_from_agent_http, get_ducklake_from_agent_http};
use crate::common::{build_args_values, get_reserved_variables, OccupancyMetrics};
use crate::handle_child::run_future_with_polling_update_job_poller;
#[cfg(feature = "mysql")]
use crate::mysql_executor::MysqlDatabase;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use crate::sql_utils::remove_comments;
use windmill_common::client::AuthedClient;
use windmill_object_store::DEFAULT_STORAGE;

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
    run_inline: bool,
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
                        s3_obj.storage.as_deref().unwrap_or(DEFAULT_STORAGE),
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

        // Replace custom ATTACH statements with the real instructions
        let query_block_list = {
            let mut v = vec![];
            for query_block in query_block_list.iter() {
                let query_block = remove_comments(&query_block);
                if let Some(parsed) = parse_attach_db_resource(query_block) {
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
                } else if let Some(ducklake_query) = transform_attach_ducklake(
                    &query_block,
                    conn,
                    &mut hidden_passwords,
                    &job.workspace_id,
                )
                .await?
                {
                    v.extend(ducklake_query);
                } else if let Some(datatable_query) = transform_attach_datatable(
                    &query_block,
                    conn,
                    &mut hidden_passwords,
                    &job.workspace_id,
                )
                .await?
                {
                    v.extend(datatable_query);
                } else {
                    v.push(query_block.to_string());
                }
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

    let result = if run_inline {
        result_f.await
    } else {
        run_future_with_polling_update_job_poller(
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
        .await
    };

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
            res.to_conn_str()
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
    db_resource_to_attach_statements(db_resource, parsed.name, parsed.db_type, parsed.extra_args)
        .await
}

async fn db_resource_to_attach_statements(
    db_resource: Value,
    ident_name: &str,
    db_type: &str,
    extra_args: Option<&str>,
) -> Result<Vec<String>> {
    let attach_str = format!(
        "ATTACH '{}' as {} (TYPE {}{});",
        format_attach_db_conn_str(db_resource, db_type)?,
        ident_name,
        db_type,
        extra_args.unwrap_or("")
    )
    .to_string();

    Ok(vec![
        get_attach_db_install_str(db_type)?.to_string(),
        format!("LOAD {};", db_type),
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
    let storage = ducklake
        .storage
        .storage
        .as_deref()
        .unwrap_or(DEFAULT_STORAGE);
    let data_path = ducklake.storage.path;

    // Ducklake 0.3 only requires DATA_PATH at creation and then stores it internally in the catalog
    // But it will fail if DATA_PATH changes afterwards which is annoying for us
    // So we always enable override
    let extra_args = if extra_args.contains("OVERRIDE_DATA_PATH") {
        extra_args
    } else {
        format!(", OVERRIDE_DATA_PATH TRUE{extra_args}")
    };
    let extra_args = if let Some(default_extra_args) = ducklake.extra_args {
        // premise : extra_args is always non empty (and doesn't end with a comma given it's valid)
        format!("{},{}", extra_args, default_extra_args)
    } else {
        extra_args
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

async fn transform_attach_datatable(
    query: &str,
    conn: &Connection,
    hidden_passwords: &mut Arc<Mutex<Vec<String>>>,
    w_id: &str,
) -> Result<Option<Vec<String>>> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"(?i)ATTACH\s*'datatable(://[^':]+)?'\s*AS\s+([^ ;]+)").unwrap();
    }
    let Some(cap) = RE.captures(query) else {
        return Ok(None);
    };
    let name = cap.get(1).map(|m| &m.as_str()[3..]).unwrap_or("main");
    let alias_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

    let db_resource = match conn {
        Connection::Http(client) => {
            get_datatable_resource_from_agent_http(client, name, w_id).await?
        }
        Connection::Sql(db) => get_datatable_resource_from_db_unchecked(db, w_id, name).await?,
    };
    let db_type = "postgres";

    if let Some(pwd) = db_resource.get("password").and_then(|p| p.as_str()) {
        hidden_passwords.lock().unwrap().push(pwd.to_string());
    }

    Ok(Some(
        db_resource_to_attach_statements(db_resource, alias_name, db_type, None).await?,
    ))
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
            storage = DEFAULT_STORAGE;

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
// and deleted by do_duckdb after the query is executed.
//
// This relies on the fact that DuckDB does not run in native worker, so
// a worker will only run a single job at a time.
pub struct UseBigQueryCredentialsFile {
    path: String,
}
impl UseBigQueryCredentialsFile {
    fn new(job_id: Uuid, bigquery_resource: &str) -> Result<Self> {
        let path = format!("/tmp/service-account-credentials-{}.json", job_id);
        unsafe {
            env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &path);
        }
        std::fs::write(&path, bigquery_resource)
            .map_err(|e| Error::ExecutionErr(format!("Failed to write BigQuery creds: {e}")))?;
        Ok(Self { path })
    }
}
impl Drop for UseBigQueryCredentialsFile {
    fn drop(&mut self) {
        unsafe {
            env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for parse_attach_db_resource function
    #[test]
    fn test_parse_attach_db_resource_postgres_res_prefix() {
        let query = "ATTACH '$res:u/user/my_postgres' AS mydb (TYPE POSTGRES)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "u/user/my_postgres");
        assert_eq!(parsed.name, "mydb");
        assert_eq!(parsed.db_type, "POSTGRES");
        assert!(parsed.extra_args.is_none() || parsed.extra_args.unwrap().is_empty());
    }

    #[test]
    fn test_parse_attach_db_resource_res_protocol() {
        let query = "ATTACH 'res://f/folder/database' AS db (TYPE postgresql)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "f/folder/database");
        assert_eq!(parsed.name, "db");
        assert_eq!(parsed.db_type, "postgresql");
    }

    #[test]
    fn test_parse_attach_db_resource_mysql() {
        let query = "ATTACH '$res:u/admin/mysql_prod' AS mysql_db (TYPE mysql)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "u/admin/mysql_prod");
        assert_eq!(parsed.name, "mysql_db");
        assert_eq!(parsed.db_type, "mysql");
    }

    #[test]
    fn test_parse_attach_db_resource_bigquery() {
        let query = "ATTACH '$res:u/user/bq_resource' AS bq (TYPE bigquery)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "u/user/bq_resource");
        assert_eq!(parsed.name, "bq");
        assert_eq!(parsed.db_type, "bigquery");
    }

    #[test]
    fn test_parse_attach_db_resource_with_extra_args() {
        let query = "ATTACH '$res:u/user/db' AS mydb (TYPE POSTGRES, READ_ONLY)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "u/user/db");
        assert_eq!(parsed.name, "mydb");
        assert_eq!(parsed.db_type, "POSTGRES");
        assert_eq!(parsed.extra_args.unwrap(), ", READ_ONLY");
    }

    #[test]
    fn test_parse_attach_db_resource_case_insensitive() {
        let query = "attach '$res:u/user/db' as mydb (type postgres)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "u/user/db");
        assert_eq!(parsed.name, "mydb");
        assert_eq!(parsed.db_type, "postgres");
    }

    #[test]
    fn test_parse_attach_db_resource_no_match() {
        let query = "SELECT * FROM table";
        let result = parse_attach_db_resource(query);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_attach_db_resource_regular_attach() {
        // Regular ATTACH without $res: or res:// should not match
        let query = "ATTACH 'mydb.duckdb' AS mydb (TYPE duckdb)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_none());
    }

    // Tests for format_attach_db_conn_str function
    #[test]
    fn test_format_attach_db_conn_str_postgres_full() {
        let db_resource = json!({
            "host": "localhost",
            "port": 5432,
            "user": "admin",
            "password": "secret123",
            "dbname": "mydb",
            "sslmode": "require"
        });
        let result = format_attach_db_conn_str(db_resource, "postgres").unwrap();
        assert!(result.contains("dbname=mydb"));
        assert!(result.contains("user=admin"));
        assert!(result.contains("host=localhost"));
        assert!(result.contains("password=secret123"));
        assert!(result.contains("port=5432"));
        assert!(result.contains("sslmode=require"));
    }

    #[test]
    fn test_format_attach_db_conn_str_postgres_minimal() {
        let db_resource = json!({
            "host": "db.example.com",
            "dbname": "production"
        });
        let result = format_attach_db_conn_str(db_resource, "postgres").unwrap();
        assert!(result.contains("dbname=production"));
        assert!(result.contains("host=db.example.com"));
        // Optional fields should result in empty strings
        assert!(!result.contains("user="));
        assert!(!result.contains("password="));
    }

    #[test]
    fn test_format_attach_db_conn_str_postgresql_alias() {
        let db_resource = json!({
            "host": "localhost",
            "dbname": "test"
        });
        let result = format_attach_db_conn_str(db_resource, "postgresql").unwrap();
        assert!(result.contains("dbname=test"));
        assert!(result.contains("host=localhost"));
    }

    #[test]
    fn test_format_attach_db_conn_str_bigquery() {
        let db_resource = json!({
            "project_id": "my-gcp-project"
        });
        let result = format_attach_db_conn_str(db_resource, "bigquery").unwrap();
        assert_eq!(result, "project=my-gcp-project");
    }

    #[test]
    fn test_format_attach_db_conn_str_bigquery_missing_project_id() {
        let db_resource = json!({
            "other_field": "value"
        });
        let result = format_attach_db_conn_str(db_resource, "bigquery");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("project_id"));
    }

    #[test]
    fn test_format_attach_db_conn_str_unsupported_type() {
        let db_resource = json!({});
        let result = format_attach_db_conn_str(db_resource, "oracle");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported db type"));
    }

    #[test]
    fn test_format_attach_db_conn_str_case_insensitive() {
        let db_resource = json!({
            "host": "localhost",
            "dbname": "test"
        });
        let result = format_attach_db_conn_str(db_resource, "POSTGRES").unwrap();
        assert!(result.contains("dbname=test"));
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_format_attach_db_conn_str_mysql_full() {
        let db_resource = json!({
            "host": "mysql.example.com",
            "port": 3306,
            "user": "root",
            "password": "mysecret",
            "database": "app_db",
            "ssl": true
        });
        let result = format_attach_db_conn_str(db_resource, "mysql").unwrap();
        assert!(result.contains("database=app_db"));
        assert!(result.contains("host=mysql.example.com"));
        assert!(result.contains("ssl_mode=required"));
        assert!(result.contains("password=mysecret"));
        assert!(result.contains("port=3306"));
        assert!(result.contains("user=root"));
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_format_attach_db_conn_str_mysql_ssl_disabled() {
        let db_resource = json!({
            "host": "localhost",
            "database": "test",
            "ssl": false
        });
        let result = format_attach_db_conn_str(db_resource, "mysql").unwrap();
        assert!(result.contains("ssl_mode=disabled"));
    }

    // Tests for get_attach_db_install_str function
    #[test]
    fn test_get_attach_db_install_str_postgres() {
        let result = get_attach_db_install_str("postgres").unwrap();
        assert_eq!(result, "INSTALL postgres;");
    }

    #[test]
    fn test_get_attach_db_install_str_bigquery() {
        let result = get_attach_db_install_str("bigquery").unwrap();
        assert_eq!(result, "INSTALL bigquery FROM community;");
    }

    #[test]
    fn test_get_attach_db_install_str_unsupported() {
        let result = get_attach_db_install_str("sqlite");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported db type"));
    }

    #[test]
    fn test_get_attach_db_install_str_case_insensitive() {
        let result = get_attach_db_install_str("POSTGRES").unwrap();
        assert_eq!(result, "INSTALL postgres;");
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_get_attach_db_install_str_mysql() {
        let result = get_attach_db_install_str("mysql").unwrap();
        assert_eq!(result, "INSTALL mysql;");
    }

    #[cfg(not(feature = "mysql"))]
    #[test]
    fn test_get_attach_db_install_str_mysql_disabled() {
        let result = get_attach_db_install_str("mysql");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("MySQL feature is not enabled"));
    }

    // Tests for transform_s3_uris function
    #[tokio::test]
    async fn test_transform_s3_uris_empty_storage() {
        let query = "SELECT * FROM read_parquet('s3:///path/to/file.parquet')";
        let result = transform_s3_uris(query).await.unwrap();
        assert_eq!(
            result,
            "SELECT * FROM read_parquet('s3://_default_/path/to/file.parquet')"
        );
    }

    #[tokio::test]
    async fn test_transform_s3_uris_with_storage() {
        // URIs with explicit storage should not be transformed
        let query = "SELECT * FROM read_parquet('s3://mybucket/path/to/file.parquet')";
        let result = transform_s3_uris(query).await.unwrap();
        assert_eq!(result, query);
    }

    #[tokio::test]
    async fn test_transform_s3_uris_multiple_empty() {
        let query = "SELECT * FROM read_parquet('s3:///file1.parquet') UNION SELECT * FROM read_parquet('s3:///file2.parquet')";
        let result = transform_s3_uris(query).await.unwrap();
        assert!(result.contains("s3://_default_/file1.parquet"));
        assert!(result.contains("s3://_default_/file2.parquet"));
    }

    #[tokio::test]
    async fn test_transform_s3_uris_no_s3() {
        let query = "SELECT * FROM my_table";
        let result = transform_s3_uris(query).await.unwrap();
        assert_eq!(result, query);
    }

    #[tokio::test]
    async fn test_transform_s3_uris_mixed() {
        let query = "SELECT * FROM read_parquet('s3:///default.parquet'), read_csv('s3://explicit/file.csv')";
        let result = transform_s3_uris(query).await.unwrap();
        assert!(result.contains("s3://_default_/default.parquet"));
        assert!(result.contains("s3://explicit/file.csv"));
    }

    #[tokio::test]
    async fn test_transform_s3_uris_nested_path() {
        let query = "SELECT * FROM read_parquet('s3:///deep/nested/path/file.parquet')";
        let result = transform_s3_uris(query).await.unwrap();
        assert_eq!(
            result,
            "SELECT * FROM read_parquet('s3://_default_/deep/nested/path/file.parquet')"
        );
    }

    // Tests for Arg struct
    #[test]
    fn test_arg_serialization() {
        let arg = Arg {
            name: "test_arg".to_string(),
            arg_type: "string".to_string(),
            json_value: json!("hello"),
        };
        let serialized = serde_json::to_string(&arg).unwrap();
        assert!(serialized.contains("\"name\":\"test_arg\""));
        assert!(serialized.contains("\"arg_type\":\"string\""));
        assert!(serialized.contains("\"json_value\":\"hello\""));
    }

    #[test]
    fn test_arg_serialization_number() {
        let arg = Arg {
            name: "count".to_string(),
            arg_type: "integer".to_string(),
            json_value: json!(42),
        };
        let serialized = serde_json::to_string(&arg).unwrap();
        assert!(serialized.contains("\"json_value\":42"));
    }

    #[test]
    fn test_arg_serialization_null() {
        let arg = Arg {
            name: "optional".to_string(),
            arg_type: "text".to_string(),
            json_value: json!(null),
        };
        let serialized = serde_json::to_string(&arg).unwrap();
        assert!(serialized.contains("\"json_value\":null"));
    }

    #[test]
    fn test_arg_serialization_array() {
        let arg = Arg {
            name: "items".to_string(),
            arg_type: "array".to_string(),
            json_value: json!([1, 2, 3]),
        };
        let serialized = serde_json::to_string(&arg).unwrap();
        assert!(serialized.contains("\"json_value\":[1,2,3]"));
    }

    #[test]
    fn test_arg_serialization_object() {
        let arg = Arg {
            name: "config".to_string(),
            arg_type: "object".to_string(),
            json_value: json!({"key": "value"}),
        };
        let serialized = serde_json::to_string(&arg).unwrap();
        assert!(serialized.contains("\"json_value\":{\"key\":\"value\"}"));
    }
}
