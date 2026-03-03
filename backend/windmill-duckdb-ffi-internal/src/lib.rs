use std::{
    collections::HashMap,
    ffi::{CStr, CString, c_char, c_uint},
    ptr::null_mut,
};

use duckdb::{Row, core::LogicalTypeId, params_from_iter, types::TimeUnit};
use rust_decimal::{Decimal, prelude::FromPrimitive};
use serde::Deserialize;
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

    let (s3_access_key, s3_secret_key) = token.split_at(token.rfind('.').unwrap_or(0));
    let s3_secret_key = &s3_secret_key[1..];
    let (s3_endpoint_ssl, s3_endpoint) = base_internal_url
        .split_once("://")
        .unwrap_or(("http", &base_internal_url));
    let s3_endpoint_ssl = match s3_endpoint_ssl {
        "https" => true,
        _ => false,
    };

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
    .map_err(|e| format!("Error setting up S3 secret: {}", e.to_string()))?;

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
