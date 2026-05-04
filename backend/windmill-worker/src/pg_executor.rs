use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use base64::{engine, Engine as _};
use chrono::Utc;
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt, TryStreamExt};
use itertools::Itertools;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde_json::value::RawValue;
use serde_json::Map;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};
use tokio_postgres::Client;
use tokio_postgres::{types::ToSql, Row};
use tokio_postgres::{
    types::{FromSql, IsNull, Kind, Type},
    Column,
};
use uuid::Uuid;
use windmill_common::error::to_anyhow;
use windmill_common::error::{self, Error};
use windmill_common::worker::{
    to_raw_value, Connection, SqlResultCollectionStrategy, CLOUD_HOSTED,
};
use windmill_common::workspaces::get_datatable_resource_from_db_unchecked;
use windmill_common::{PgDatabase, PrepareQueryColumnInfo, PrepareQueryResult, DB};
use windmill_parser::{Arg, Typ};
use windmill_parser_sql::{
    parse_db_resource, parse_pg_statement_arg_positions, parse_pgsql_sig_with_typed_schema,
    parse_s3_mode, parse_sql_blocks,
};
use windmill_queue::{CanceledBy, MiniPulledJob};

use crate::agent_workers::get_datatable_resource_from_agent_http;
use crate::common::{
    build_args_values, get_reserved_variables, s3_mode_args_to_worker_data,
    s3_stream_and_upload_with_logs, sizeof_val, OccupancyMetrics, S3ModeWorkerData,
};
use crate::handle_child::run_future_with_polling_update_job_poller;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use crate::sql_s3_input::fetch_s3object_as_json_text;
use crate::sql_utils::remove_comments;
use crate::MAX_RESULT_SIZE;
use bytes::Buf;
use lazy_static::lazy_static;
use windmill_common::client::AuthedClient;
use windmill_types::s3::S3Object;

lazy_static! {
    pub static ref CONNECTION_CACHE: Arc<Mutex<Option<(String, tokio_postgres::Client)>>> =
        Arc::new(Mutex::new(None));
    pub static ref CONNECTION_COUNTER: Arc<RwLock<HashMap<String, u64>>> =
        Arc::new(RwLock::new(HashMap::new()));
    pub static ref LAST_QUERY: AtomicU64 = AtomicU64::new(0);
    pub static ref CACHE_HITS: AtomicU64 = AtomicU64::new(0);
}

pub async fn clear_pg_cache() {
    *CONNECTION_CACHE.lock().await = None;
    CONNECTION_COUNTER.write().await.clear();
}

async fn new_pg_connection(
    database: &PgDatabase,
    _use_iam_auth: bool,
    main_db: Option<&DB>,
) -> error::Result<(tokio_postgres::Client, tokio::task::JoinHandle<()>)> {
    let (client, connection) = if _use_iam_auth {
        #[cfg(all(feature = "enterprise", feature = "private"))]
        {
            database.connect_with_iam().await?
        }
        #[cfg(not(all(feature = "enterprise", feature = "private")))]
        {
            return Err(Error::ExecutionErr(
                "IAM RDS authentication requires Windmill Enterprise Edition".to_string(),
            ));
        }
    } else {
        database.connect(main_db).await?
    };
    let handle = tokio::spawn(async move {
        if let Err(e) = connection.await {
            let mut mtex = CONNECTION_CACHE.lock().await;
            *mtex = None;
            tracing::error!("connection error: {}", e);
        }
    });
    Ok((client, handle))
}

/// `ToSql` / `FromSql` wrapper for a value whose Postgres wire format is plain
/// UTF-8 text regardless of the column's *type kind*. Vanilla
/// `tokio_postgres`'s `ToSql for String` / `FromSql for String` only accepts a
/// fixed list of base text types (TEXT/VARCHAR/BPCHAR/NAME/UNKNOWN + citext) —
/// they reject user-defined `Kind::Enum` and `Kind::Domain` even though
/// enum/domain wire format is just the variant name / the underlying base
/// type's text. This wrapper plugs that gap on both directions:
///
/// - **bind side** (prepare-fallback path): `INSERT INTO t VALUES
///   ($1::my_enum)` works end-to-end without users needing the
///   `CAST($1::text AS my_enum)` workaround.
/// - **read side** (`pg_cell_to_json_value`'s fallback): `SELECT
///   $1::my_enum`, `SELECT enum_col FROM t`, etc. round-trip into a JSON
///   string instead of erroring with "cannot convert Option<String> and the
///   Postgres type `my_enum`".
#[derive(Debug)]
struct AnyTextValue(String);

fn any_text_accepts(ty: &Type) -> bool {
    // Base text-like types, plus the citext extension type matched by name
    // (it's not in `tokio_postgres::types::Type`'s constants), plus
    // enum/domain kinds. We accept `Kind::Domain` unconditionally — the
    // server is responsible for parsing the bytes and any domain whose
    // base type accepts text on the wire (which is most of them) round-trips
    // naturally.
    matches!(
        *ty,
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME | Type::UNKNOWN
    ) || ty.name() == "citext"
        || matches!(ty.kind(), Kind::Enum(_) | Kind::Domain(_))
}

impl ToSql for AnyTextValue {
    fn to_sql(
        &self,
        _ty: &Type,
        out: &mut bytes::BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        use bytes::BufMut;
        out.put_slice(self.0.as_bytes());
        Ok(IsNull::No)
    }

    fn accepts(ty: &Type) -> bool {
        any_text_accepts(ty)
    }

    tokio_postgres::types::to_sql_checked!();
}

impl<'a> FromSql<'a> for AnyTextValue {
    fn from_sql(
        _ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        // Postgres' text wire format for enums / domains-over-text / the
        // base text types is the same: UTF-8 bytes of the value.
        Ok(AnyTextValue(std::str::from_utf8(raw)?.to_owned()))
    }

    fn accepts(ty: &Type) -> bool {
        any_text_accepts(ty)
    }
}

impl ResultFormatState {
    /// Decide whether to actually run the precision-loss check for this cell.
    /// Returns `true` for the first `NUMERIC_PRECISION_CHECK_BUDGET` calls,
    /// then `false` thereafter — and always `false` once the warning has
    /// already been triggered. Cheap on the hot path: an atomic load + an
    /// atomic decrement (Relaxed ordering), no allocation.
    fn should_check_precision(&self) -> bool {
        use std::sync::atomic::Ordering;
        if self.numeric_precision_loss.load(Ordering::Relaxed) {
            return false;
        }
        // `fetch_sub` returns the value BEFORE the decrement. When that's
        // > 0 we had budget left for this cell. After the budget reaches 0
        // the next call would wrap to `u32::MAX-1`; pin it back to 0.
        let prev = self
            .numeric_precision_check_budget
            .fetch_sub(1, Ordering::Relaxed);
        if prev == 0 {
            self.numeric_precision_check_budget
                .store(0, Ordering::Relaxed);
            false
        } else {
            true
        }
    }
}

/// Emit a single job-log warning if `state.numeric_precision_loss` flipped
/// during the row iteration. The detection itself is bounded by
/// `NUMERIC_PRECISION_CHECK_BUDGET` cells, so this only adds a constant-cost
/// log call at end-of-query.
async fn warn_on_numeric_precision_loss(
    state: &ResultFormatState,
    job_id: Uuid,
    workspace_id: &str,
    log_conn: &Connection,
) {
    use std::sync::atomic::Ordering;
    if state.numeric_precision_loss.load(Ordering::Relaxed) {
        windmill_queue::append_logs(
            &job_id,
            workspace_id,
            "warning: at least one `numeric` value in the result lost precision \
             when serialised as a JSON number (the JSON Number format goes through \
             f64, which has ~15-17 significant digits). To preserve full precision, \
             cast the column to text in your SQL — e.g. `SELECT col::text` — and \
             parse the string client-side with a Decimal library.\n",
            log_conn,
        )
        .await;
    }
}

/// Emit a one-shot warning naming each declared arg the user didn't supply a
/// value for. PG executor binds these as NULL for back-compat — without a
/// warning, a misspelled arg key in the args object silently produces a row
/// of NULLs, which is a notoriously hard DX bug to track down.
async fn warn_on_missing_args(
    missing: &[String],
    job_id: Uuid,
    workspace_id: &str,
    log_conn: &Connection,
) {
    if missing.is_empty() {
        return;
    }
    let names = missing
        .iter()
        .map(|n| format!("`{n}`"))
        .collect::<Vec<_>>()
        .join(", ");
    windmill_queue::append_logs(
        &job_id,
        workspace_id,
        format!(
            "warning: argument(s) {names} declared in the query but not provided in the \
             args object — bound as NULL. Add the value(s) to the job args, declare a \
             default in the SQL (`-- $1 name (type) = default`), or remove the \
             declaration if the arg isn't used.\n"
        ),
        log_conn,
    )
    .await;
}

/// Short stable label for the JSON value's variant — used in error messages
/// so users can see *what kind of value* hit a binding error.
fn json_value_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// rust-postgres reports parameter encoding failures as
/// `error serializing parameter N: <inner>` with N being a 0-based index.
/// Pull N out so we can attach our own metadata.
fn parse_param_index_from_err_msg(msg: &str) -> Option<usize> {
    msg.strip_prefix("error serializing parameter ")
        .and_then(|rest| rest.split(':').next())
        .and_then(|n| n.parse::<usize>().ok())
}

/// Replace a rust-postgres encoder error with one that names the offending
/// arg, its JSON value kind, and the Postgres type we asserted, plus a hint
/// about how to fix it. Other errors are passed through unchanged.
fn wrap_param_encoding_error(
    err: tokio_postgres::Error,
    param_meta: &[(String, &'static str)],
    param_types: &[Type],
) -> Error {
    let msg = err.to_string();
    if let Some(idx) = parse_param_index_from_err_msg(&msg) {
        if let (Some((name, kind)), Some(t)) = (param_meta.get(idx), param_types.get(idx)) {
            return Error::ExecutionErr(format!(
                "Cannot bind arg `{name}` (JSON {kind}) as Postgres type `{t}` ({err}). \
                 Try adding an explicit cast in the SQL — e.g. `${pos}::<column_type>` \
                 or `CAST(${pos} AS <column_type>)` — or declare the type via \
                 `-- ${pos} {name} (<column_type>)`.",
                pos = idx + 1,
            ));
        }
    }
    to_anyhow(err).into()
}

fn otyp_to_pg_type(otyp: &str) -> error::Result<Type> {
    let base = otyp.trim_end_matches("[]");
    let is_array = otyp.ends_with("[]");

    let (scalar, array) = match base {
        "bool" | "boolean" => (Type::BOOL, Type::BOOL_ARRAY),
        "char" | "character" => (Type::CHAR, Type::CHAR_ARRAY),
        "smallint" | "smallserial" | "int2" | "serial2" => (Type::INT2, Type::INT2_ARRAY),
        "int" | "integer" | "int4" | "serial" => (Type::INT4, Type::INT4_ARRAY),
        "bigint" | "bigserial" | "int8" | "serial8" => (Type::INT8, Type::INT8_ARRAY),
        "real" | "float4" => (Type::FLOAT4, Type::FLOAT4_ARRAY),
        "double" | "double precision" | "float8" => (Type::FLOAT8, Type::FLOAT8_ARRAY),
        "numeric" | "decimal" => (Type::NUMERIC, Type::NUMERIC_ARRAY),
        "text" => (Type::TEXT, Type::TEXT_ARRAY),
        "varchar" | "character varying" => (Type::VARCHAR, Type::VARCHAR_ARRAY),
        "uuid" => (Type::UUID, Type::UUID_ARRAY),
        "date" => (Type::DATE, Type::DATE_ARRAY),
        "time" => (Type::TIME, Type::TIME_ARRAY),
        "timetz" => (Type::TIMETZ, Type::TIMETZ_ARRAY),
        "timestamp" => (Type::TIMESTAMP, Type::TIMESTAMP_ARRAY),
        "timestamptz" => (Type::TIMESTAMPTZ, Type::TIMESTAMPTZ_ARRAY),
        "json" => (Type::JSON, Type::JSON_ARRAY),
        "jsonb" => (Type::JSONB, Type::JSONB_ARRAY),
        "bytea" => (Type::BYTEA, Type::BYTEA_ARRAY),
        "oid" => (Type::OID, Type::OID_ARRAY),
        _ => {
            return Err(Error::ExecutionErr(format!(
                "Unsupported PostgreSQL type for typed schema: {}",
                otyp
            )))
        }
    };

    Ok(if is_array { array } else { scalar })
}

fn do_postgresql_inner<'a>(
    mut query: String,
    param_idx_to_arg_and_value: &HashMap<i32, (&Arg, Option<&Value>)>,
    client: &'a Client,
    column_order: Option<&'a mut Option<Vec<String>>>,
    siz: &'a AtomicUsize,
    skip_collect: bool,
    first_row_only: bool,
    s3: Option<S3ModeWorkerData>,
    job_id: Uuid,
    workspace_id: &'a str,
    log_conn: &'a Connection,
) -> error::Result<BoxFuture<'a, error::Result<Vec<Box<RawValue>>>>> {
    let mut query_params = vec![];
    let mut param_types: Vec<Type> = vec![];
    // Per-param metadata used to wrap rust-postgres `error serializing
    // parameter N` errors with actionable context (arg name, JSON value kind,
    // asserted Postgres type) — see error wrapping at the dispatch site.
    let mut param_meta: Vec<(String, &'static str)> = vec![];
    // Track whether every arg has a resolvable Postgres type. We need *both*
    // the parser-supplied otyp to be in `otyp_to_pg_type`'s map (so the arg
    // isn't a custom enum / extension type) *and* convert_val to produce a
    // (binding, type) pair that the encoder can actually serialize. If both
    // hold for every arg, we send the query as an unnamed prepared statement
    // (query_typed_raw) — see the dispatch comment below. Otherwise we fall
    // back to prepare + query_raw and let the server resolve the parameter
    // types from the SQL context.
    let mut all_types_resolved = true;

    // Single tokenizer pass — derive both the index set (for the param
    // dispatch loop below) and the byte ranges (for sparse renumbering) from
    // one walk over the SQL. Positions skip occurrences inside string
    // literals, comments, and dollar-quoted blocks, so the rewrite below
    // doesn't mangle a query like `SELECT 'price: $5' AS lbl, $5 FROM t`.
    let positions = parse_pg_statement_arg_positions(&query);
    let arg_indices: HashSet<i32> = positions.iter().map(|(i, _)| *i).collect();

    // Renumber sparse positional placeholders (e.g. $5, $50 → $1, $2) by
    // byte position, walking back-to-front so earlier positions don't shift.
    let renumber_mapping: HashMap<i32, usize> = arg_indices
        .iter()
        .sorted()
        .enumerate()
        .map(|(i, oidx)| (*oidx, i + 1))
        .collect();
    if renumber_mapping
        .iter()
        .any(|(oidx, new_i)| *oidx as usize != *new_i)
    {
        let mut positions = positions.clone();
        positions.sort_by_key(|(_, range)| std::cmp::Reverse(range.start));
        for (oidx, range) in positions {
            if let Some(new_i) = renumber_mapping.get(&oidx) {
                if oidx as usize != *new_i {
                    query.replace_range(range, &new_i.to_string());
                }
            }
        }
    }

    // Args the user didn't supply a value for — if their declaration doesn't
    // carry a default, we still bind NULL (back-compat with how the PG
    // executor has worked for years), but we collect them here to emit a
    // single one-shot warning to the job logs after query execution so a typo
    // / missing key doesn't silently turn into a row of NULLs.
    let mut missing_args: Vec<String> = Vec::new();
    // Stash declaration-default values so we can borrow them by reference
    // alongside user-supplied values — both paths feed `convert_val(&Value)`.
    let mut default_values: HashMap<i32, serde_json::Value> = HashMap::new();

    for oidx in arg_indices.iter().sorted() {
        if let Some((arg, value)) = param_idx_to_arg_and_value.get(&oidx) {
            // Resolve the value: explicit user value > declaration default > NULL.
            let value: &serde_json::Value = match (value, arg.default.as_ref()) {
                (Some(v), _) => *v,
                (None, Some(d)) => default_values.entry(*oidx).or_insert_with(|| d.clone()),
                (None, None) => {
                    if !arg.has_default && !missing_args.contains(&arg.name) {
                        missing_args.push(arg.name.clone());
                    }
                    &serde_json::Value::Null
                }
            };
            let arg_t = arg
                .otyp
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing otyp for pg arg"))?;
            let typ = &arg.typ;
            let (param, natural_type) = convert_val(value, arg_t, typ, arg.otyp_inferred)?;
            query_params.push(param);
            param_meta.push((arg.name.clone(), json_value_kind(value)));
            if all_types_resolved {
                if otyp_to_pg_type(arg_t).is_ok() {
                    // The Type comes from convert_val (paired with the binding's
                    // concrete Rust type) rather than from `otyp_to_pg_type(arg_t)`
                    // — this prevents the parser-default "text" otyp from
                    // forcing an assertion that the encoder can't satisfy
                    // (e.g. Value::Bool with parser-defaulted text → Type::TEXT
                    // on a Box<bool>).
                    param_types.push(natural_type);
                } else {
                    all_types_resolved = false;
                    param_types.clear();
                }
            }
        }
    }

    let result_f = async move {
        let mut res: Vec<Box<serde_json::value::RawValue>> = vec![];

        // Always prefer query_typed_raw (unnamed prepared statement). It is sent as
        // a single Parse+Bind+Execute+Sync round-trip, so it survives transaction-mode
        // connection poolers (PgBouncer/Supabase pooler/RDS Proxy) where named
        // statements ("s0", "s1", ...) can be reported missing because the prepare
        // and the execute land on different backend connections. Fall back to
        // prepare + query_raw only when an arg has a type unsupported by
        // otyp_to_pg_type (e.g. custom enum, geometry, …) — in that case we lose
        // pooler safety, but the query at least runs against a direct connection.
        let rows = if all_types_resolved {
            let typed_params = query_params
                .iter()
                .zip(param_types.iter())
                .map(|(p, t)| (&**p as &(dyn ToSql + Sync), t.clone()));
            match client.query_typed_raw(&query, typed_params).await {
                Ok(rows) => rows,
                Err(e) => {
                    return Err(wrap_param_encoding_error(e, &param_meta, &param_types));
                }
            }
        } else {
            let query_params = query_params
                .iter()
                .map(|p| &**p as &(dyn ToSql + Sync))
                .collect_vec();
            let statement = client.prepare(&query).await.map_err(to_anyhow)?;
            client
                .query_raw(&statement, query_params)
                .await
                .map_err(to_anyhow)?
        };

        // One state object per query — `pg_cell_to_json_value_with_state`
        // flips `numeric_precision_loss` once if any `numeric` cell can't
        // round-trip through f64. We emit a single warning to the job log
        // after the iteration finishes, instead of error-by-error or per
        // row, and the per-row check short-circuits on the flag so the cost
        // is one branch after the first lossy value.
        let format_state = ResultFormatState::default();

        if skip_collect {
            futures::pin_mut!(rows);
            while rows.try_next().await.map_err(to_anyhow)?.is_some() {}
        } else if let Some(ref s3) = s3 {
            let format_state_ref = &format_state;
            let rows_stream = rows.map_err(to_anyhow).map(move |row_result| {
                row_result.and_then(|row| {
                    postgres_row_to_json_value_with_state(row, format_state_ref).map_err(to_anyhow)
                })
            });

            s3_stream_and_upload_with_logs(
                "PostgreSQL",
                rows_stream.boxed(),
                s3,
                job_id,
                workspace_id,
                log_conn,
            )
            .await?;

            warn_on_numeric_precision_loss(&format_state, job_id, workspace_id, log_conn).await;
            warn_on_missing_args(&missing_args, job_id, workspace_id, log_conn).await;

            return Ok(vec![to_raw_value(&s3.to_return_s3_obj())]);
        } else {
            let rows = if first_row_only {
                rows.take(1).boxed()
            } else {
                rows.boxed()
            };

            let rows = rows.try_collect::<Vec<Row>>().await.map_err(to_anyhow)?;

            if let Some(column_order) = column_order {
                *column_order = Some(
                    rows.first()
                        .map(|x| {
                            x.columns()
                                .iter()
                                .map(|x| x.name().to_string())
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default(),
                );
            }

            for row in rows.into_iter() {
                let r = postgres_row_to_json_value_with_state(row, &format_state);
                if let Ok(v) = r.as_ref() {
                    let size = sizeof_val(v);
                    siz.fetch_add(size, Ordering::Relaxed);
                }
                if *CLOUD_HOSTED {
                    let siz = siz.load(Ordering::Relaxed);
                    if siz > MAX_RESULT_SIZE * 4 {
                        return Err(Error::ExecutionErr(format!(
                            "Query result too large for cloud (size = {} > {})",
                            siz,
                            MAX_RESULT_SIZE & 4,
                        )));
                    }
                }
                if let Ok(v) = r {
                    res.push(to_raw_value(&v));
                } else {
                    return Err(to_anyhow(r.err().unwrap()).into());
                }
            }
        }

        warn_on_numeric_precision_loss(&format_state, job_id, workspace_id, log_conn).await;
        warn_on_missing_args(&missing_args, job_id, workspace_id, log_conn).await;

        Ok(res)
    };

    Ok(result_f.boxed())
}

pub async fn do_postgresql(
    job: &MiniPulledJob,
    client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
    parent_runnable_path: Option<String>,
    run_inline: bool,
) -> error::Result<Box<RawValue>> {
    let mut pg_args = build_args_values(job, client, conn).await?;

    let inline_db_res_path = parse_db_resource(&query);

    let s3 = parse_s3_mode(&query)?.map(|s3| s3_mode_args_to_worker_data(s3, client.clone(), job));

    let db_arg = if let Some(inline_db_res_path) = inline_db_res_path {
        Some(
            client
                .get_resource_value_interpolated::<serde_json::Value>(
                    &inline_db_res_path,
                    Some(job.id.to_string()),
                )
                .await?,
        )
    } else {
        match pg_args.get("database").cloned() {
            Some(Value::String(db_str)) if db_str.starts_with("datatable://") => {
                let db_str = db_str.trim_start_matches("datatable://");
                Some(match conn {
                    Connection::Http(client) => {
                        get_datatable_resource_from_agent_http(client, &db_str, &job.workspace_id)
                            .await?
                    }
                    Connection::Sql(db) => {
                        get_datatable_resource_from_db_unchecked(db, &job.workspace_id, &db_str)
                            .await?
                    }
                })
            }
            database => database,
        }
    };

    let database = if let Some(db) = db_arg {
        serde_json::from_value::<PgDatabase>(db.clone())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?
    } else {
        return Err(Error::BadRequest("Missing database argument".to_string()));
    };

    let annotations = windmill_common::worker::SqlAnnotations::parse(query);
    let collection_strategy = if annotations.return_last_result {
        SqlResultCollectionStrategy::LastStatementAllRows
    } else {
        annotations.result_collection
    };

    let use_iam_auth = database.use_iam_auth == Some(true);

    // Include use_iam_auth in cache key to distinguish IAM vs non-IAM connections to the same host.
    // The cache key is static (doesn't include the token), which is correct because PostgreSQL
    // connections remain valid after initial auth — fresh tokens are generated on cache miss.
    let database_string = if use_iam_auth {
        format!("{}?iam=true", database.to_uri())
    } else {
        database.to_uri()
    };
    let database_string_clone = database_string.clone();

    let cached_client;
    let new_client;
    if !*CLOUD_HOSTED {
        let mut guard = CONNECTION_CACHE.try_lock().ok();
        increment_connection_counter(&database_string).await;

        if guard
            .as_ref()
            .is_some_and(|x| x.as_ref().is_some_and(|y| y.0 == database_string))
        {
            // Probe the cached connection with a curated session reset before
            // reusing it. Each statement targets a specific class of state:
            //
            //   RESET ALL                     — GUC parameters (search_path,
            //                                   application_name, statement_
            //                                   timeout, transaction_*…). Note
            //                                   that this does NOT reset SET
            //                                   ROLE or SET SESSION
            //                                   AUTHORIZATION (security!).
            //   RESET SESSION AUTHORIZATION   — undoes both `SET SESSION
            //                                   AUTHORIZATION` and `SET ROLE`,
            //                                   restoring the connecting user.
            //                                   Without this a previous job
            //                                   leaving an elevated role
            //                                   active would silently leak
            //                                   permissions into the next.
            //   UNLISTEN *                    — drops LISTEN registrations.
            //   CLOSE ALL                     — closes open cursors.
            //   pg_advisory_unlock_all()      — releases any session-scoped
            //                                   advisory locks. Without this
            //                                   a job that called
            //                                   pg_advisory_lock and exited
            //                                   without unlocking would block
            //                                   later jobs holding the same
            //                                   key (DISCARD ALL covered this
            //                                   too).
            //
            // We deliberately do NOT use `DISCARD ALL`. DISCARD includes
            // `DEALLOCATE ALL`, which deallocates *all* prepared statements
            // server-side — including the typeinfo statements that
            // tokio_postgres caches per-Client to resolve custom enum/domain
            // Oids. After DISCARD, tokio_postgres still holds Statement
            // objects whose names the server has forgotten, so the next
            // custom-type query fails with `prepared statement "sN" does not
            // exist`. The trade-off: temp tables and user-PREPARE statements
            // may persist across cached-connection reuse (rare in datatable /
            // script workloads).
            //
            // Doubles as a liveness probe — if the connection is broken any
            // statement in the chain fails and we replace it.
            let probe_client = &guard.as_ref().unwrap().as_ref().unwrap().1;
            if probe_client
                .batch_execute(
                    "RESET ALL; \
                     RESET SESSION AUTHORIZATION; \
                     UNLISTEN *; \
                     CLOSE ALL; \
                     SELECT pg_advisory_unlock_all();",
                )
                .await
                .is_ok()
            {
                tracing::info!("Using cached connection");
                CACHE_HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                LAST_QUERY.store(
                    chrono::Utc::now().timestamp().try_into().unwrap_or(0),
                    std::sync::atomic::Ordering::Relaxed,
                );
                cached_client = guard;
                new_client = None;
            } else {
                tracing::info!("Cached connection is stale, creating new one");
                if let Some(ref mut g) = guard {
                    **g = None;
                }
                drop(guard);
                cached_client = None;
                new_client = Some(new_pg_connection(&database, use_iam_auth, conn.as_sql()).await?);
            }
        } else {
            // Release the lock before connecting so the post-query caching
            // code can re-acquire it.
            drop(guard);
            cached_client = None;
            new_client = Some(new_pg_connection(&database, use_iam_auth, conn.as_sql()).await?);
        }
    } else {
        cached_client = None;
        new_client = Some(new_pg_connection(&database, use_iam_auth, conn.as_sql()).await?);
    }

    let (mut sig, _) = parse_pgsql_sig_with_typed_schema(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?;

    // Materialize any `(s3object)` args into JSON text and rebind them as `jsonb` so
    // `otyp_to_pg_type` picks the right binding. Must run before the param map is
    // built below.
    materialize_s3object_args(&mut sig.args, &mut pg_args, client, &job.workspace_id).await?;

    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

    let (query, _) =
        &sanitize_and_interpolate_unsafe_sql_args(query, &sig.args, &pg_args, &reserved_variables)?;

    let queries = parse_sql_blocks(query, true);

    let (client, handle) = if let Some((client, handle)) = new_client.as_ref() {
        (client, Some(handle))
    } else {
        let (_, client) = cached_client.as_ref().unwrap().as_ref().unwrap();
        (client, None)
    };

    let param_idx_to_arg_and_value = sig
        .args
        .iter()
        .filter_map(|x| x.oidx.map(|oidx| (oidx, (x, pg_args.get(&x.name)))))
        .collect::<HashMap<_, _>>();

    let size = AtomicUsize::new(0);
    let size_ref = &size;
    let result_f = async move {
        let mut results = vec![];
        // Session reset (DISCARD ALL) is now handled eagerly when validating
        // the cached connection — no per-query reset needed here.

        for (i, query) in queries.iter().enumerate() {
            if annotations.prepare {
                let query = remove_comments(query);
                // Used by the data table typechecker to set default schemas
                if query.starts_with("SET search_path") || query.starts_with("RESET search_path") {
                    let _ = client.execute(&query.to_string(), &[]).await;
                    continue;
                }
                let prepared = client.prepare(&query).await;
                let prepared = match prepared {
                    Ok(prepared) => {
                        let columns: Option<Vec<PrepareQueryColumnInfo>> = Some(
                            prepared
                                .columns()
                                .iter()
                                .map(|col| PrepareQueryColumnInfo {
                                    name: col.name().to_string(),
                                    type_name: col.type_().name().to_string(),
                                })
                                .collect(),
                        );
                        PrepareQueryResult { columns, error: None }
                    }
                    Err(e) => PrepareQueryResult { columns: None, error: Some(e.to_string()) },
                };
                results.push(vec![to_raw_value(&prepared)]);
                continue;
            }

            let result = do_postgresql_inner(
                query.to_string(),
                &param_idx_to_arg_and_value,
                client,
                if i == queries.len() - 1
                    && s3.is_none()
                    && collection_strategy.collect_last_statement_only(queries.len())
                    && !collection_strategy.collect_scalar()
                {
                    Some(column_order)
                } else {
                    None
                },
                size_ref,
                collection_strategy.collect_last_statement_only(queries.len())
                    && i < queries.len() - 1,
                collection_strategy.collect_first_row_only(),
                s3.clone(),
                job.id,
                &job.workspace_id,
                conn,
            )?
            .await?;
            results.push(result);
        }

        collection_strategy.collect(results)
    };

    let result = if run_inline {
        result_f.await?
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
        .await?
    };

    // Release the cache lock now that we have the result — allows the
    // post-query caching code below to re-acquire it if needed.
    drop(cached_client);

    *mem_peak = size.load(Ordering::Relaxed) as i32;

    if let Some(handle) = handle {
        if !*CLOUD_HOSTED {
            if let Ok(mut mtex) = CONNECTION_CACHE.try_lock() {
                if mtex.as_ref().is_none_or(|x| x.0 != database_string) {
                    let abort_handler = handle.abort_handle();

                    let mut cache_new_con = false;
                    if let Some(new_client) = new_client {
                        cache_new_con = is_most_used_conn(&database_string).await;
                        if cache_new_con {
                            *mtex = Some((database_string, new_client.0));
                        } else {
                            new_client.1.abort();
                        }
                    } else {
                        handle.abort();
                    }

                    if cache_new_con {
                        LAST_QUERY.store(
                            chrono::Utc::now().timestamp().try_into().unwrap_or(0),
                            std::sync::atomic::Ordering::Relaxed,
                        );
                        tokio::spawn(async move {
                            loop {
                                tokio::time::sleep(Duration::from_secs(5)).await;
                                let last_query =
                                    LAST_QUERY.load(std::sync::atomic::Ordering::Relaxed);
                                let now = chrono::Utc::now().timestamp().try_into().unwrap_or(0);

                                //we cache connection for 5 minutes at most
                                if last_query + 60 * 1 < now {
                                    // tracing::error!("Closing cache connection due to inactivity");
                                    tracing::info!(
                                        "Closing cache pg executor connection due to inactivity"
                                    );
                                    break;
                                }
                                let mtex = CONNECTION_CACHE.lock().await;
                                if mtex.is_none() {
                                    // connection is not in the mutex anymore
                                    break;
                                } else if let Some(mtex) = mtex.as_ref() {
                                    if mtex.0.as_str() != &database_string_clone {
                                        // connection is not the latest one
                                        break;
                                    }
                                }

                                tracing::debug!(
                                    "Keeping cached pg executor connection alive due to activity"
                                )
                            }
                            let mut mtex = CONNECTION_CACHE.lock().await;
                            *mtex = None;
                            abort_handler.abort();
                        });
                    }
                } else {
                    handle.abort();
                }
            } else {
                handle.abort();
            }
        } else {
            handle.abort();
        }
    }
    *mem_peak = (result.get().len() / 1000) as i32;
    // And then check that we got back the same string we sent over.
    return Ok(result);
}

async fn is_most_used_conn(database_string: &str) -> bool {
    let counter_map = CONNECTION_COUNTER.read().await;
    let current_count = counter_map.get(database_string).copied().unwrap_or(0);
    let max_count = counter_map.values().copied().max().unwrap_or(0);
    current_count >= max_count
}

async fn increment_connection_counter(database_string: &str) {
    let mut counter_map = CONNECTION_COUNTER.write().await;
    *counter_map.entry(database_string.to_string()).or_insert(0) += 1;
}

/// For each `(s3object)` arg in `sig_args`: download the referenced file, decode it
/// to JSON text, then rewrite the arg to bind as `jsonb`. Mutates `args_map` in place
/// so the existing bind path picks up the materialized payload.
async fn materialize_s3object_args(
    sig_args: &mut [Arg],
    args_map: &mut HashMap<String, Value>,
    client: &AuthedClient,
    workspace_id: &str,
) -> error::Result<()> {
    for arg in sig_args.iter_mut() {
        if arg.otyp.as_deref() != Some("s3object") {
            continue;
        }
        let raw = args_map.remove(&arg.name).unwrap_or(Value::Null);
        if matches!(raw, Value::Null) {
            return Err(Error::BadRequest(format!(
                "Missing S3Object value for arg `{}`",
                arg.name
            )));
        }
        let s3_obj: S3Object = serde_json::from_value(raw).map_err(|e| {
            Error::ExecutionErr(format!("Invalid S3Object for arg `{}`: {e}", arg.name))
        })?;
        let json_text = fetch_s3object_as_json_text(client, workspace_id, &s3_obj)
            .await
            .map_err(|e| {
                Error::ExecutionErr(format!(
                    "Failed to fetch S3 object for arg `{}`: {e}",
                    arg.name
                ))
            })?;
        // Parse to a Value so `convert_val`'s Array/Object → JSONB branches bind it
        // correctly. A bare String would mismatch the JSONB param type.
        let parsed: Value = serde_json::from_str(&json_text).map_err(|e| {
            Error::ExecutionErr(format!(
                "S3 object for arg `{}` is not valid JSON after decoding: {e}",
                arg.name
            ))
        })?;
        args_map.insert(arg.name.clone(), parsed);
        arg.otyp = Some("jsonb".to_string());
        arg.typ = Typ::Object(windmill_parser::ObjectType::new(None, Some(vec![])));
    }
    Ok(())
}

/// Parse a date string in formats produced by chrono's Display or JS frontends.
fn parse_naive_date(s: &str) -> Result<chrono::NaiveDate, chrono::ParseError> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ"))
        .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ"))
        .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
}

/// Parse a time string in formats produced by chrono's Display or JS frontends.
fn parse_naive_time(s: &str) -> Result<chrono::NaiveTime, chrono::ParseError> {
    chrono::NaiveTime::parse_from_str(s, "%H:%M:%S%.f")
        .or_else(|_| chrono::NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .or_else(|_| chrono::NaiveTime::parse_from_str(s, "%H:%M"))
        .or_else(|_| {
            // Handle full datetime strings by extracting the time part
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ"))
                .map(|dt| dt.time())
        })
}

/// Parse a naive datetime string in formats produced by chrono's Display or JS frontends.
fn parse_naive_datetime(s: &str) -> Result<chrono::NaiveDateTime, chrono::ParseError> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ"))
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ"))
}

/// Parse a timestamptz string in formats produced by chrono's Display or JS frontends.
fn parse_datetime_utc(s: &str) -> Result<chrono::DateTime<Utc>, chrono::ParseError> {
    s.parse::<chrono::DateTime<Utc>>()
        .or_else(|_| {
            // Handle numeric timezone offsets: "2024-01-15 10:30:00+00", "+00:00", "+0000"
            chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f%#z")
                .or_else(|_| chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%#z"))
                .map(|dt| dt.with_timezone(&Utc))
        })
        .or_else(|_| {
            // Handle chrono's Display format: "2024-01-15 10:30:00 UTC"
            let trimmed = s.trim_end_matches(" UTC");
            chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S%.f")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S"))
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S%.fZ")
                })
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S%.f"))
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%SZ"))
                .map(|ndt| ndt.and_utc())
        })
}

fn map_as_single_type<T>(
    vec: Option<&Vec<Value>>,
    f: impl Fn(&Value) -> Option<T>,
) -> anyhow::Result<Option<Vec<Option<T>>>> {
    if let Some(vec) = vec {
        Ok(Some(
            vec.into_iter()
                .map(|v| {
                    // first option is if the value is of the right type (if none, will stop the collection and throw error)
                    // second option is if the value is null
                    // allow nulls in arrays
                    if matches!(v, Value::Null) {
                        Some(None)
                    } else {
                        f(v).map(Some)
                    }
                })
                .collect::<Option<Vec<Option<T>>>>()
                .ok_or_else(|| anyhow::anyhow!("Mixed types in array"))?,
        ))
    } else {
        Ok(None)
    }
}

/// A boxed `ToSql` value paired with the Postgres `Type` that matches its
/// concrete Rust type. Returned by `convert_val` / `convert_vec_val` so the
/// dispatch in `do_postgresql_inner` always asserts the type that the encoder
/// can actually produce — never a parser-derived guess that drifts from the
/// runtime binding.
type ConvertedParam = (Box<dyn ToSql + Sync + Send>, Type);

fn convert_vec_val(
    vec: Option<&Vec<Value>>,
    arg_t: &String,
) -> windmill_common::error::Result<ConvertedParam> {
    match arg_t.as_str() {
        // Each integer / bool array arm accepts both JSON-native values AND
        // stringified counterparts ("1", "true", …) — same coercion the
        // scalar `Value::String → <type>` arms in `convert_val` apply, so an
        // array passed via `JSON.stringify(BigInt(...))` or hand-quoted
        // values doesn't trip a confusing "Mixed types in array" error.
        "bool" | "boolean" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_bool()
                    .or_else(|| match v.as_str()?.to_ascii_lowercase().as_str() {
                        "true" | "t" | "yes" | "y" | "1" | "on" => Some(true),
                        "false" | "f" | "no" | "n" | "0" | "off" => Some(false),
                        _ => None,
                    })
            })?),
            Type::BOOL_ARRAY,
        )),
        "char" | "character" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_i64()
                    .map(|x| x as i8)
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i8>().ok()))
            })?),
            Type::CHAR_ARRAY,
        )),
        "smallint" | "smallserial" | "int2" | "serial2" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_i64()
                    .map(|x| x as i16)
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i16>().ok()))
            })?),
            Type::INT2_ARRAY,
        )),
        "int" | "integer" | "int4" | "serial" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_i64()
                    .map(|x| x as i32)
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i32>().ok()))
            })?),
            Type::INT4_ARRAY,
        )),
        // Mirror the scalar `Value::String → numeric` parsing arm so an array
        // like `["1.5", "2.5"]` works against `$1::numeric[]` — useful for
        // bulk-loading via `unnest`. Without this the user would see an
        // unhelpful "Mixed types in array" error.
        "numeric" | "decimal" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                if v.is_i64() {
                    Decimal::from_i64(v.as_i64().unwrap())
                } else if v.is_f64() {
                    Decimal::from_f64(v.as_f64().unwrap())
                } else {
                    v.as_str().and_then(|s| s.parse::<Decimal>().ok())
                }
            })?),
            Type::NUMERIC_ARRAY,
        )),
        "oid" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_u64()
                    .map(|x| x as u32)
                    .or_else(|| v.as_str().and_then(|s| s.parse::<u32>().ok()))
            })?),
            Type::OID_ARRAY,
        )),
        "bigint" | "bigserial" | "int8" | "serial8" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_i64()
                    .or_else(|| v.as_u64().map(|x| x as i64))
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
            })?),
            Type::INT8_ARRAY,
        )),
        "real" | "float4" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_f64()
                    .map(|x| x as f32)
                    .or_else(|| v.as_str().and_then(|s| s.parse::<f32>().ok()))
            })?),
            Type::FLOAT4_ARRAY,
        )),
        "double" | "double precision" | "float8" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
            })?),
            Type::FLOAT8_ARRAY,
        )),
        "uuid" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_str().map(|x| Uuid::parse_str(x).ok()).flatten()
            })?),
            Type::UUID_ARRAY,
        )),
        "date" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_str().and_then(|x| parse_naive_date(x).ok())
            })?),
            Type::DATE_ARRAY,
        )),
        "time" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_str().and_then(|x| parse_naive_time(x).ok())
            })?),
            Type::TIME_ARRAY,
        )),
        "timetz" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_str().and_then(|x| parse_naive_time(x).ok())
            })?),
            // chrono's `NaiveTime` only encodes for `TIME` — same caveat as
            // the scalar `timetz` arm. Asserting `TIMETZ_ARRAY` here would
            // fail at the encoder. Postgres has an implicit `time → timetz`
            // assignment cast at the column site.
            Type::TIME_ARRAY,
        )),
        "timestamp" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_str().and_then(|x| parse_naive_datetime(x).ok())
            })?),
            Type::TIMESTAMP_ARRAY,
        )),
        "timestamptz" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_str().and_then(|x| parse_datetime_utc(x).ok())
            })?),
            Type::TIMESTAMPTZ_ARRAY,
        )),
        "jsonb" => Ok((
            Box::new(vec.map(|v| v.clone().into_iter().map(Some).collect_vec())),
            Type::JSONB_ARRAY,
        )),
        "json" => Ok((
            Box::new(vec.map(|v| v.clone().into_iter().map(Some).collect_vec())),
            Type::JSON_ARRAY,
        )),
        "bytea" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_str().map(|x| {
                    engine::general_purpose::STANDARD
                        .decode(x)
                        .unwrap_or(vec![])
                })
            })?),
            Type::BYTEA_ARRAY,
        )),
        "varchar" | "character varying" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_str().map(|x| x.to_string())
            })?),
            Type::VARCHAR_ARRAY,
        )),
        "text" => Ok((
            Box::new(map_as_single_type(vec, |v| {
                v.as_str().map(|x| x.to_string())
            })?),
            Type::TEXT_ARRAY,
        )),
        _ => Err(anyhow::anyhow!("Unsupported JSON array type"))?,
    }
}

fn convert_val(
    value: &Value,
    arg_t: &String,
    typ: &Typ,
    otyp_inferred: bool,
) -> windmill_common::error::Result<ConvertedParam> {
    // Helper: was the user's intent explicitly "text" / "varchar" / "char"?
    // True when the parser saw an inline `$N::text` cast or a `-- $N (text)`
    // declaration. False when the parser fell back to "text" because nothing
    // else was found (in which case the caller has no real target type
    // committed and we should bind the value's natural type).
    let explicit_text_target = !otyp_inferred
        && (matches!(typ, Typ::Str(_))
            && (arg_t == "text"
                || arg_t == "varchar"
                || arg_t == "character varying"
                || arg_t == "char"
                || arg_t == "character"));
    match value {
        Value::Array(vec) if arg_t.ends_with("[]") => {
            let arg_t = arg_t.trim_end_matches("[]").to_string();
            convert_vec_val(Some(vec), &arg_t)
        }
        Value::Null if arg_t.ends_with("[]") => {
            let arg_t = arg_t.trim_end_matches("[]").to_string();
            convert_vec_val(None, &arg_t)
        }
        Value::Null => match arg_t.as_str() {
            "bool" | "boolean" => Ok((Box::new(None::<bool>), Type::BOOL)),
            "char" | "character" => Ok((Box::new(None::<i8>), Type::CHAR)),
            "smallint" | "smallserial" | "int2" | "serial2" => {
                Ok((Box::new(None::<i16>), Type::INT2))
            }
            "int" | "integer" | "int4" | "serial" => Ok((Box::new(None::<i32>), Type::INT4)),
            "numeric" | "decimal" => Ok((Box::new(None::<Decimal>), Type::NUMERIC)),
            "oid" => Ok((Box::new(None::<u32>), Type::OID)),
            "bigint" | "bigserial" | "int8" | "serial8" => Ok((Box::new(None::<i64>), Type::INT8)),
            "real" | "float4" => Ok((Box::new(None::<f32>), Type::FLOAT4)),
            "double" | "double precision" | "float8" => Ok((Box::new(None::<f64>), Type::FLOAT8)),
            "uuid" => Ok((Box::new(None::<Uuid>), Type::UUID)),
            "date" => Ok((Box::new(None::<chrono::NaiveDate>), Type::DATE)),
            "time" => Ok((Box::new(None::<chrono::NaiveTime>), Type::TIME)),
            // chrono's NaiveTime has no timezone, so its ToSql impl only
            // accepts TIME. We assert TIME and rely on Postgres' implicit
            // assignment cast time → timetz at the use site.
            "timetz" => Ok((Box::new(None::<chrono::NaiveTime>), Type::TIME)),
            "timestamp" => Ok((Box::new(None::<chrono::NaiveDateTime>), Type::TIMESTAMP)),
            "timestamptz" => Ok((Box::new(None::<chrono::DateTime<Utc>>), Type::TIMESTAMPTZ)),
            "jsonb" => Ok((Box::new(None::<Value>), Type::JSONB)),
            "json" => Ok((Box::new(None::<Value>), Type::JSON)),
            "bytea" => Ok((Box::new(None::<Vec<u8>>), Type::BYTEA)),
            "varchar" | "character varying" => Ok((Box::new(None::<String>), Type::VARCHAR)),
            "text" => Ok((Box::new(None::<String>), Type::TEXT)),
            // Unrecognised arg_t — bind as TEXT NULL. The dispatch will fall
            // back to prepare + query_raw, where the server resolves the
            // actual column type and `Option<String>`'s ToSql will accept the
            // resolved Type for any text-like base; for enum/domain kinds
            // None is encoded as the literal NULL message body, so the
            // accepts() check is the only place that matters and we just need
            // a binding whose accepts() is permissive enough.
            _ => Ok((Box::new(None::<AnyTextValue>), Type::TEXT)),
        },
        // Bool / Number with an *explicitly* text-typed arg: coerce to
        // String. Used when the user wrote `-- $N (text)` or `$N::text` —
        // they committed to text and may rely on equality comparisons like
        // `WHERE text_col = $1`, which need a `text = text` operator (PG has
        // no implicit `bool/int → text` cast in expression context).
        Value::Bool(b) if explicit_text_target => {
            // `char` (Type::CHAR, OID 18) is single-byte and `to_string()` of
            // a bool is multi-byte ("true"/"false") — we can't bind it as
            // CHAR. Fail explicitly with an actionable hint rather than
            // silently sending BOOL (which the server then can't compare
            // against a CHAR column — `operator does not exist: bool = char`).
            // `character` (= bpchar, fixed-length text) has the same issue.
            // For text/varchar/character varying we coerce to a string.
            match arg_t.as_str() {
                "char" | "character" => Err(Error::ExecutionErr(format!(
                    "Cannot bind a JSON bool to a `{arg_t}` arg. \
                     `char` and `character` are single-byte / fixed-width text — \
                     pass the value as a string (e.g. \"t\" / \"f\") or change \
                     the arg type to `bool`."
                ))),
                "varchar" | "character varying" => Ok((Box::new(b.to_string()), Type::VARCHAR)),
                _ => Ok((Box::new(b.to_string()), Type::TEXT)),
            }
        }
        // Bool: bind as BOOL when no explicit text target. Postgres has an
        // implicit assignment cast bool→text, so INSERTs into text columns
        // still work — this only differs from the explicit-text branch above
        // for expression-context uses (WHERE clauses, etc.).
        Value::Bool(_) if arg_t == "jsonb" => Ok((Box::new(value.clone()), Type::JSONB)),
        Value::Bool(_) if arg_t == "json" => Ok((Box::new(value.clone()), Type::JSON)),
        Value::Bool(b) => Ok((Box::new(b.clone()), Type::BOOL)),
        // Number with an explicitly text-typed arg: coerce to String. Same
        // reasoning as the Bool branch — preserves pre-#8988 behaviour for
        // hand-written PG scripts that use `WHERE text_col = $1` with a
        // numeric value and an explicit text declaration.
        // Skip `char`/`character`: those go to the existing single-byte arm
        // below or the generic INT8 fallthrough.
        Value::Number(n)
            if explicit_text_target
                && (arg_t == "text" || arg_t == "varchar" || arg_t == "character varying") =>
        {
            let t = if arg_t == "varchar" || arg_t == "character varying" {
                Type::VARCHAR
            } else {
                Type::TEXT
            };
            Ok((Box::new(n.to_string()), t))
        }
        Value::Number(n) if arg_t == "char" && n.is_i64() => {
            Ok((Box::new(n.as_i64().unwrap() as i8), Type::CHAR))
        }
        Value::Number(n)
            if (arg_t == "smallint"
                || arg_t == "smallserial"
                || arg_t == "int2"
                || arg_t == "serial2")
                && n.is_i64() =>
        {
            Ok((Box::new(n.as_i64().unwrap() as i16), Type::INT2))
        }
        Value::Number(n)
            if (arg_t == "int" || arg_t == "integer" || arg_t == "int4" || arg_t == "serial")
                && n.is_i64() =>
        {
            Ok((Box::new(n.as_i64().unwrap() as i32), Type::INT4))
        }
        Value::Number(n) if (arg_t == "real" || arg_t == "float4") && n.as_f64().is_some() => {
            Ok((Box::new(n.as_f64().unwrap() as f32), Type::FLOAT4))
        }
        Value::Number(n)
            if (arg_t == "double" || arg_t == "double precision" || arg_t == "float8")
                && n.as_f64().is_some() =>
        {
            Ok((Box::new(n.as_f64().unwrap()), Type::FLOAT8))
        }
        Value::Number(n) if (arg_t == "numeric" || arg_t == "decimal") && n.is_i64() => Ok((
            Box::new(Decimal::from_i64(n.as_i64().unwrap()).unwrap_or_default()),
            Type::NUMERIC,
        )),
        Value::Number(n) if (arg_t == "numeric" || arg_t == "decimal") && n.is_f64() => Ok((
            Box::new(Decimal::from_f64(n.as_f64().unwrap()).unwrap_or_default()),
            Type::NUMERIC,
        )),
        Value::Number(n) if arg_t == "oid" && n.is_u64() => {
            Ok((Box::new(n.as_u64().unwrap() as u32), Type::OID))
        }
        Value::Number(n)
            if (arg_t == "bigint"
                || arg_t == "bigserial"
                || arg_t == "int8"
                || arg_t == "serial8")
                && n.is_u64() =>
        {
            Ok((Box::new(n.as_u64().unwrap() as i64), Type::INT8))
        }
        Value::Number(n) if n.is_i64() => Ok((Box::new(n.as_i64().unwrap()), Type::INT8)),
        Value::Number(n) => Ok((Box::new(n.as_f64().unwrap()), Type::FLOAT8)),
        Value::String(s) if arg_t == "uuid" => Ok((Box::new(Uuid::parse_str(s)?), Type::UUID)),
        Value::String(s)
            if arg_t == "smallint"
                || arg_t == "smallserial"
                || arg_t == "int2"
                || arg_t == "serial2" =>
        {
            s.parse::<i16>()
                .map(|n| (Box::new(n) as Box<dyn ToSql + Sync + Send>, Type::INT2))
                .map_err(|e| anyhow::anyhow!("Cannot parse '{s}' as smallint: {e}").into())
        }
        Value::String(s)
            if arg_t == "int" || arg_t == "integer" || arg_t == "int4" || arg_t == "serial" =>
        {
            s.parse::<i32>()
                .map(|n| (Box::new(n) as Box<dyn ToSql + Sync + Send>, Type::INT4))
                .map_err(|e| anyhow::anyhow!("Cannot parse '{s}' as integer: {e}").into())
        }
        Value::String(s)
            if arg_t == "bigint"
                || arg_t == "bigserial"
                || arg_t == "int8"
                || arg_t == "serial8" =>
        {
            s.parse::<i64>()
                .map(|n| (Box::new(n) as Box<dyn ToSql + Sync + Send>, Type::INT8))
                .map_err(|e| anyhow::anyhow!("Cannot parse '{s}' as bigint: {e}").into())
        }
        Value::String(s) if arg_t == "date" => {
            let date = parse_naive_date(s)
                .map_err(|e| Error::ExecutionErr(format!("Cannot parse '{s}' as date: {e}")))?;
            Ok((Box::new(date), Type::DATE))
        }
        Value::String(s) if arg_t == "time" => {
            let time = parse_naive_time(s)
                .map_err(|e| Error::ExecutionErr(format!("Cannot parse '{s}' as time: {e}")))?;
            Ok((Box::new(time), Type::TIME))
        }
        Value::String(s) if arg_t == "timetz" => {
            let time = parse_naive_time(s)
                .map_err(|e| Error::ExecutionErr(format!("Cannot parse '{s}' as time: {e}")))?;
            // See the timetz Null arm — assert TIME, server casts to TIMETZ.
            Ok((Box::new(time), Type::TIME))
        }
        Value::String(s) if arg_t == "timestamp" => {
            let datetime = parse_naive_datetime(s).map_err(|e| {
                Error::ExecutionErr(format!("Cannot parse '{s}' as timestamp: {e}"))
            })?;
            Ok((Box::new(datetime), Type::TIMESTAMP))
        }
        Value::String(s) if arg_t == "timestamptz" => {
            let datetime = parse_datetime_utc(s).map_err(|e| {
                Error::ExecutionErr(format!("Cannot parse '{s}' as timestamptz: {e}"))
            })?;
            Ok((Box::new(datetime), Type::TIMESTAMPTZ))
        }
        Value::String(s) if arg_t == "bytea" => {
            let bytes = engine::general_purpose::STANDARD
                .decode(s)
                .unwrap_or(vec![]);
            Ok((Box::new(bytes), Type::BYTEA))
        }
        // Parse Strings into the matching native Rust type for the remaining
        // recognised arg_ts that didn't have a dedicated arm. Without these,
        // a string value lands in the generic Value::String fallback below
        // (Box<String> + TEXT) and the server-side comparison
        // `<numeric|real|...> = text` fails since PG has no implicit cast.
        Value::String(s) if arg_t == "numeric" || arg_t == "decimal" => s
            .parse::<Decimal>()
            .map(|d| (Box::new(d) as Box<dyn ToSql + Sync + Send>, Type::NUMERIC))
            .map_err(|e| anyhow::anyhow!("Cannot parse '{s}' as numeric: {e}").into()),
        Value::String(s) if arg_t == "real" || arg_t == "float4" => s
            .parse::<f32>()
            .map(|n| (Box::new(n) as Box<dyn ToSql + Sync + Send>, Type::FLOAT4))
            .map_err(|e| anyhow::anyhow!("Cannot parse '{s}' as real: {e}").into()),
        Value::String(s)
            if arg_t == "double" || arg_t == "double precision" || arg_t == "float8" =>
        {
            s.parse::<f64>()
                .map(|n| (Box::new(n) as Box<dyn ToSql + Sync + Send>, Type::FLOAT8))
                .map_err(|e| anyhow::anyhow!("Cannot parse '{s}' as double: {e}").into())
        }
        Value::String(s) if arg_t == "oid" => s
            .parse::<u32>()
            .map(|n| (Box::new(n) as Box<dyn ToSql + Sync + Send>, Type::OID))
            .map_err(|e| anyhow::anyhow!("Cannot parse '{s}' as oid: {e}").into()),
        Value::String(s) if arg_t == "bool" || arg_t == "boolean" => {
            // Accept the same literals Postgres' boolin() does.
            let b = match s.to_ascii_lowercase().as_str() {
                "true" | "t" | "yes" | "y" | "1" | "on" => true,
                "false" | "f" | "no" | "n" | "0" | "off" => false,
                _ => {
                    return Err(
                        anyhow::anyhow!("Cannot parse '{s}' as bool: invalid literal").into(),
                    )
                }
            };
            Ok((Box::new(b), Type::BOOL))
        }
        Value::String(s) if arg_t == "varchar" || arg_t == "character varying" => {
            Ok((Box::new(s.clone()), Type::VARCHAR))
        }
        // For arg_t in (json, jsonb): bind a JSON-encodable Value with the
        // matching pg type. Falling through to TEXT here would assert TEXT
        // and break query_typed_raw's encoder check.
        // Object / Array (no `[]` suffix): bind as JSONB by default and
        // JSON-stringify when the target is text-like.
        //
        // Note the asymmetry vs the Bool/Number arms above: we coerce to
        // text on `matches!(typ, Typ::Str(_))` (which is true for both
        // explicit `(text)` decls AND parser-default text), not on
        // `explicit_text_target`. Reason: serialising a JSON object/array
        // as JSONB and binding against a parser-default-text arg would
        // assert `JSONB` for what could be a plain-text column. Postgres
        // has no implicit cast `jsonb → text` in expression context, so
        // `WHERE text_col = $1::JSONB` would fail. JSON-stringifying into
        // TEXT is what users almost always want for these JSON shapes
        // (and the result is itself valid JSON, so a `::jsonb` cast in
        // SQL still round-trips). Bool/Number don't need this safety
        // because `bool → text` and `int → text` have implicit assignment
        // casts; the asymmetry is therefore semantic, not a bug.
        Value::Array(_) if arg_t == "jsonb" => Ok((Box::new(value.clone()), Type::JSONB)),
        Value::Array(_) if arg_t == "json" => Ok((Box::new(value.clone()), Type::JSON)),
        Value::Array(_) if matches!(typ, Typ::Str(_)) => {
            let s = serde_json::to_string(value).map_err(|err| {
                Error::ExecutionErr(format!("Failed to convert JSON to text: {}", err))
            })?;
            let t = if arg_t == "varchar" {
                Type::VARCHAR
            } else {
                Type::TEXT
            };
            Ok((Box::new(s), t))
        }
        // Default for arrays without a [] suffix: bind as JSONB.
        Value::Array(_) => Ok((Box::new(value.clone()), Type::JSONB)),
        Value::Object(_) if arg_t == "json" => Ok((Box::new(value.clone()), Type::JSON)),
        Value::Object(_) if arg_t == "varchar" || arg_t == "character varying" => Ok((
            Box::new(serde_json::to_string(value).map_err(|err| {
                Error::ExecutionErr(format!("Failed to convert JSON to text: {}", err))
            })?),
            Type::VARCHAR,
        )),
        Value::Object(_) if arg_t == "text" || matches!(typ, Typ::Str(_)) => Ok((
            Box::new(serde_json::to_string(value).map_err(|err| {
                Error::ExecutionErr(format!("Failed to convert JSON to text: {}", err))
            })?),
            Type::TEXT,
        )),
        Value::Object(_) => Ok((Box::new(value.clone()), Type::JSONB)),
        // Generic String fallback. Use `AnyTextValue` (rather than plain
        // `String`) so the binding's `accepts()` covers `Kind::Enum` and
        // `Kind::Domain` in addition to the base text types — this is what
        // makes `INSERT INTO t VALUES ($1::my_enum)` work end-to-end without
        // users needing the `CAST($1::text AS my_enum)` workaround.
        //
        // We always assert `Type::TEXT` (not `Type::UNKNOWN`): tokio_postgres
        // sends parameter values in binary format, and Postgres rejects
        // binary-formatted bytes for `UNKNOWN` parameters in operator
        // contexts ("incorrect binary data format in bind parameter N"). The
        // trade-off is that bare `$1` against a non-text column still needs
        // an explicit cast (`$1::my_enum`), but the failure mode is a clear
        // server error rather than a cryptic protocol mismatch.
        Value::String(s) => Ok((Box::new(AnyTextValue(s.clone())), Type::TEXT)),
    }
}

/// Hard cap on how many `numeric` cells we test for f64-precision loss per
/// query. The check is `Decimal -> f64 -> Decimal` round-trip + `==` (~tens
/// of ns each); on a query returning millions of numeric cells, an
/// unbounded check would add measurable latency. After this many "fits
/// fine" observations we assume the rest do too — the pathological case
/// (rows 1..N fit, row N+1 loses precision) goes silently truncated, but
/// users who care about precision in such results can `::text`-cast their
/// SQL anyway. The first cell that does NOT fit short-circuits the budget
/// (the warning fires once and the per-row check stops immediately).
const NUMERIC_PRECISION_CHECK_BUDGET: u32 = 256;

/// Per-query state carried through result formatting. Currently used to
/// detect precision loss on the first `numeric` cell that doesn't round-trip
/// through f64, so the caller can emit a single warning per job rather than
/// silently truncating every row. Uses atomics (rather than `Cell`) so the
/// s3-streaming path — which moves the closure across futures and requires
/// `Send` — can borrow it.
pub struct ResultFormatState {
    /// `true` once we've observed a `numeric` value that loses precision when
    /// converted via f64. Once flipped, the per-row check short-circuits.
    pub numeric_precision_loss: std::sync::atomic::AtomicBool,
    /// Decremented for each `numeric` cell we actually check. When it hits 0
    /// the per-row check is skipped (along with the precision-loss flag) for
    /// the rest of the query — see the rationale on
    /// `NUMERIC_PRECISION_CHECK_BUDGET`.
    numeric_precision_check_budget: std::sync::atomic::AtomicU32,
}

impl Default for ResultFormatState {
    fn default() -> Self {
        Self {
            numeric_precision_loss: std::sync::atomic::AtomicBool::new(false),
            numeric_precision_check_budget: std::sync::atomic::AtomicU32::new(
                NUMERIC_PRECISION_CHECK_BUDGET,
            ),
        }
    }
}

pub fn pg_cell_to_json_value(
    row: &Row,
    column: &Column,
    column_i: usize,
) -> Result<JSONValue, Error> {
    pg_cell_to_json_value_with_state(row, column, column_i, &ResultFormatState::default())
}

pub fn pg_cell_to_json_value_with_state(
    row: &Row,
    column: &Column,
    column_i: usize,
    state: &ResultFormatState,
) -> Result<JSONValue, Error> {
    // JSON has no encoding for NaN / +Inf / -Inf, but Postgres `float4` /
    // `float8` (and `numeric`, via the special `'NaN'` value) do return them.
    // Pre-fix the worker errored with "invalid json-float", failing the
    // entire query. Round-trip these as JSON strings ("NaN", "Infinity",
    // "-Infinity") so the rest of the row still comes through; users who
    // need numeric semantics can filter them out client-side.
    let f64_to_json_number = |raw_val: f64| -> Result<JSONValue, Error> {
        if raw_val.is_nan() {
            return Ok(JSONValue::String("NaN".to_string()));
        }
        if raw_val.is_infinite() {
            return Ok(JSONValue::String(if raw_val > 0.0 {
                "Infinity".to_string()
            } else {
                "-Infinity".to_string()
            }));
        }
        let temp =
            serde_json::Number::from_f64(raw_val).ok_or(anyhow::anyhow!("invalid json-float"))?;
        Ok(JSONValue::Number(temp))
    };
    Ok(match *column.type_() {
        // for rust-postgres <> postgres type-mappings: https://docs.rs/postgres/latest/postgres/types/trait.FromSql.html#types
        // for postgres types: https://www.postgresql.org/docs/7.4/datatype.html#DATATYPE-TABLE

        // single types
        Type::BOOL => get_basic(row, column, column_i, |a: bool| Ok(JSONValue::Bool(a)))?,
        Type::BIT => get_basic(row, column, column_i, |a: bit_vec::BitVec| match a.len() {
            1 => Ok(JSONValue::Bool(a.get(0).unwrap())),
            _ => Ok(JSONValue::String(
                a.iter()
                    .map(|x| if x { "1" } else { "0" })
                    .collect::<String>(),
            )),
        })?,
        Type::INT2 => get_basic(row, column, column_i, |a: i16| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::INT4 => get_basic(row, column, column_i, |a: i32| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::INT8 => get_basic(row, column, column_i, |a: i64| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::TEXT | Type::VARCHAR => {
            get_basic(row, column, column_i, |a: String| Ok(JSONValue::String(a)))?
        }
        // ISO-8601 / RFC-3339 for temporal types so values round-trip through
        // JS / Python clients (`new Date(s)`, `datetime.fromisoformat(s)`)
        // without manual parsing. chrono's default `to_string()` returns
        // space-separated for naive datetimes and " UTC" suffix for tz-aware,
        // neither of which is parseable as ISO 8601.
        Type::TIMESTAMP => get_basic(row, column, column_i, |a: chrono::NaiveDateTime| {
            Ok(JSONValue::String(format_naive_datetime_iso(&a)))
        })?,
        Type::DATE => get_basic(row, column, column_i, |a: chrono::NaiveDate| {
            // chrono's `NaiveDate::to_string` is already ISO-8601 (`%Y-%m-%d`).
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::TIME => get_basic(row, column, column_i, |a: chrono::NaiveTime| {
            // `NaiveTime::to_string` is already ISO-8601 (`%H:%M:%S` with
            // optional `.f`).
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::TIMETZ => get_basic(row, column, column_i, |a: TimeTZStr| {
            // TimeTZStr's `from_sql` already formats as ISO-8601 (see impl
            // below).
            Ok(JSONValue::String(a.0))
        })?,
        Type::TIMESTAMPTZ => get_basic(row, column, column_i, |a: chrono::DateTime<Utc>| {
            Ok(JSONValue::String(a.to_rfc3339()))
        })?,
        Type::UUID => get_basic(row, column, column_i, |a: uuid::Uuid| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::INET => get_basic(row, column, column_i, |a: IpAddr| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::INTERVAL => get_basic(row, column, column_i, |a: IntervalStr| {
            Ok(JSONValue::String(a.0))
        })?,
        Type::JSON | Type::JSONB => get_basic(row, column, column_i, |a: JSONValue| Ok(a))?,
        Type::FLOAT4 => get_basic(row, column, column_i, |a: f32| {
            Ok(f64_to_json_number(a.into())?)
        })?,
        // Pre-existing behaviour: `numeric` is serialised as a JSON Number
        // via `Decimal::serialize`, which goes through f64 and silently
        // truncates past ~15-17 significant digits. Switching to JSON String
        // would preserve precision but break any user script doing arithmetic
        // / comparison on numeric column results (`row.amount + 1` becomes
        // string concat, `row.amount > 100` is lexicographic). Left as Number
        // for back-compat. Instead, on the FIRST cell whose decimal
        // representation can't round-trip through f64, we flip
        // `state.numeric_precision_loss` so the caller can emit a single
        // job-log warning recommending a `::text` cast. The check is bounded
        // by `NUMERIC_PRECISION_CHECK_BUDGET` cells (see comment there) and
        // short-circuits on the first lossy value, so the hot path on a
        // numeric-heavy result set is two atomic loads + an early return.
        Type::NUMERIC => get_basic(row, column, column_i, |a: Decimal| {
            if state.should_check_precision() && !decimal_fits_f64_losslessly(&a) {
                state
                    .numeric_precision_loss
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Ok(serde_json::to_value(a)
                .map_err(|_| anyhow::anyhow!("Cannot convert decimal to json"))?)
        })?,
        Type::FLOAT8 => get_basic(row, column, column_i, |a: f64| f64_to_json_number(a))?,
        Type::BYTEA => get_basic(row, column, column_i, |a: Vec<u8>| {
            Ok(JSONValue::String(format!("\\x{}", hex::encode(a))))
        })?,
        // these types require a custom StringCollector struct as an intermediary (see struct at bottom)
        Type::TS_VECTOR => get_basic(row, column, column_i, |a: StringCollector| {
            Ok(JSONValue::String(a.0))
        })?,
        Type::OID => get_basic(row, column, column_i, |a: u32| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        // array types
        Type::BOOL_ARRAY => get_array(row, column, column_i, |a: bool| Ok(JSONValue::Bool(a)))?,
        Type::BIT_ARRAY => get_array(row, column, column_i, |a: bit_vec::BitVec| match a.len() {
            1 => Ok(JSONValue::Bool(a.get(0).unwrap())),
            _ => Ok(JSONValue::String(
                a.iter()
                    .map(|x| if x { "1" } else { "0" })
                    .collect::<String>(),
            )),
        })?,
        Type::INT2_ARRAY => get_array(row, column, column_i, |a: i16| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::INT4_ARRAY => get_array(row, column, column_i, |a: i32| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::INT8_ARRAY => get_array(row, column, column_i, |a: i64| {
            Ok(JSONValue::Number(serde_json::Number::from(a)))
        })?,
        Type::TEXT_ARRAY | Type::VARCHAR_ARRAY => {
            get_array(row, column, column_i, |a: String| Ok(JSONValue::String(a)))?
        }
        Type::JSON_ARRAY | Type::JSONB_ARRAY => {
            get_array(row, column, column_i, |a: JSONValue| Ok(a))?
        }
        Type::FLOAT4_ARRAY => get_array(row, column, column_i, |a: f32| {
            Ok(f64_to_json_number(a.into())?)
        })?,
        Type::FLOAT8_ARRAY => {
            get_array(row, column, column_i, |a: f64| Ok(f64_to_json_number(a)?))?
        }
        // See scalar NUMERIC arm — kept as JSON Number for back-compat,
        // with bounded precision-loss detection.
        Type::NUMERIC_ARRAY => get_array(row, column, column_i, |a: Decimal| {
            if state.should_check_precision() && !decimal_fits_f64_losslessly(&a) {
                state
                    .numeric_precision_loss
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Ok(serde_json::to_value(a)
                .map_err(|_| anyhow::anyhow!("Cannot convert decimal to json"))?)
        })?,
        // these types require a custom StringCollector struct as an intermediary (see struct at bottom)
        Type::TS_VECTOR_ARRAY => get_array(row, column, column_i, |a: StringCollector| {
            Ok(JSONValue::String(a.0))
        })?,
        // Same ISO-8601 formatting as the scalar arms above.
        Type::TIMESTAMP_ARRAY => get_array(row, column, column_i, |a: chrono::NaiveDateTime| {
            Ok(JSONValue::String(format_naive_datetime_iso(&a)))
        })?,
        Type::DATE_ARRAY => get_array(row, column, column_i, |a: chrono::NaiveDate| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::TIME_ARRAY => get_array(row, column, column_i, |a: chrono::NaiveTime| {
            Ok(JSONValue::String(a.to_string()))
        })?,
        Type::TIMETZ_ARRAY => get_array(row, column, column_i, |a: TimeTZStr| {
            Ok(JSONValue::String(a.0))
        })?,
        Type::TIMESTAMPTZ_ARRAY => get_array(row, column, column_i, |a: chrono::DateTime<Utc>| {
            Ok(JSONValue::String(a.to_rfc3339()))
        })?,
        Type::BYTEA_ARRAY => get_array(row, column, column_i, |a: Vec<u8>| {
            Ok(JSONValue::String(format!("\\x{}", hex::encode(a))))
        })?,
        Type::VOID => JSONValue::Null,
        // Default fallback for unhandled column types: read as text. We use
        // `AnyTextValue` instead of plain `String` so that `Kind::Enum`,
        // `Kind::Domain`, and citext columns round-trip into JSON strings
        // rather than erroring with `cannot convert between Option<String>
        // and the Postgres type \`<custom>\``.
        _ => get_basic(row, column, column_i, |a: AnyTextValue| {
            Ok(JSONValue::String(a.0))
        })?,
    })
}

pub fn postgres_row_to_json_value(row: Row) -> Result<JSONValue, Error> {
    postgres_row_to_json_value_with_state(row, &ResultFormatState::default())
}

pub fn postgres_row_to_json_value_with_state(
    row: Row,
    state: &ResultFormatState,
) -> Result<JSONValue, Error> {
    let row_data = postgres_row_to_row_data_with_state(row, state)?;
    Ok(JSONValue::Object(row_data))
}

// some type-aliases I use in my project
pub type JSONValue = serde_json::Value;
pub type RowData = Map<String, JSONValue>;

pub fn postgres_row_to_row_data(row: Row) -> Result<RowData, Error> {
    postgres_row_to_row_data_with_state(row, &ResultFormatState::default())
}

pub fn postgres_row_to_row_data_with_state(
    row: Row,
    state: &ResultFormatState,
) -> Result<RowData, Error> {
    let mut result: Map<String, JSONValue> = Map::new();
    for (i, column) in row.columns().iter().enumerate() {
        let name = column.name();
        let json_value = pg_cell_to_json_value_with_state(&row, column, i, state)?;
        result.insert(name.to_string(), json_value);
    }
    Ok(result)
}

/// Returns true if the `Decimal` value can round-trip through `f64` without
/// losing precision. Used to detect when the user's `numeric` results are
/// being silently truncated by the JSON Number serialisation path so the
/// worker can log a one-shot warning recommending a `::text` cast in SQL.
fn decimal_fits_f64_losslessly(d: &Decimal) -> bool {
    use rust_decimal::prelude::ToPrimitive;
    match d.to_f64() {
        Some(f) if f.is_finite() => Decimal::from_f64(f).is_some_and(|round| &round == d),
        _ => false,
    }
}

fn get_basic<'a, T: FromSql<'a>>(
    row: &'a Row,
    column: &Column,
    column_i: usize,
    val_to_json_val: impl Fn(T) -> Result<JSONValue, Error>,
) -> Result<JSONValue, Error> {
    let raw_val = row.try_get::<_, Option<T>>(column_i).with_context(|| {
        format!(
            "conversion issue for value at column_name `{}` with type {:?}",
            column.name(),
            column.type_()
        )
    })?;
    raw_val.map_or(Ok(JSONValue::Null), val_to_json_val)
}

struct IntervalStr(String);

impl<'a> FromSql<'a> for IntervalStr {
    fn from_sql(
        _: &Type,
        mut raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let microseconds = raw.get_i64();
        let days = raw.get_i32();
        let months = raw.get_i32();
        Ok(IntervalStr(format!(
            "{:?} months {:?} days {:?} ms",
            months, days, microseconds
        )))
    }

    fn accepts(ty: &Type) -> bool {
        matches!(ty, &Type::INTERVAL)
    }
}

struct TimeTZStr(String);
impl<'a> FromSql<'a> for TimeTZStr {
    fn from_sql(
        _: &Type,
        mut raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let microsecond = raw.get_i64();
        let offset = raw.get_i32();
        let utc_sec = (microsecond / 1_000_000) + offset as i64;
        let utc = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
            ((utc_sec + 3600 * 24) % (3600 * 24)) as u32,
            ((microsecond % 1_000_000) * 1_000) as u32,
        )
        .ok_or_else(|| anyhow::anyhow!("Invalid time value"))?;
        // ISO-8601: append `+00:00` since TIMETZ is normalised to UTC here.
        Ok(TimeTZStr(format!("{}+00:00", utc)))
    }

    fn accepts(ty: &Type) -> bool {
        matches!(ty, &Type::TIMETZ)
    }
}

/// Format a `NaiveDateTime` as ISO-8601 (`YYYY-MM-DDTHH:MM:SS[.fff…]`).
/// chrono's default `to_string` uses a space separator, which is not parseable
/// by `new Date(s)` in older JS engines or Python's `datetime.fromisoformat`
/// before 3.11. Use the explicit format string so output is portable.
fn format_naive_datetime_iso(dt: &chrono::NaiveDateTime) -> String {
    if dt.and_utc().timestamp_subsec_nanos() == 0 {
        dt.format("%Y-%m-%dT%H:%M:%S").to_string()
    } else {
        dt.format("%Y-%m-%dT%H:%M:%S%.f").to_string()
    }
}

fn get_array<'a, T: FromSql<'a>>(
    row: &'a Row,
    column: &Column,
    column_i: usize,
    val_to_json_val: impl Fn(T) -> Result<JSONValue, Error>,
) -> Result<JSONValue, Error> {
    let raw_val_array = row
        .try_get::<_, Option<Vec<Option<T>>>>(column_i)
        .with_context(|| {
            format!(
                "conversion issue for array at column_name `{}`",
                column.name()
            )
        })?;
    Ok(match raw_val_array {
        Some(val_array) => {
            let mut result = vec![];
            for val in val_array {
                result.push(
                    val.map(|v| val_to_json_val(v))
                        .transpose()?
                        .unwrap_or(Value::Null),
                );
            }
            JSONValue::Array(result)
        }
        None => JSONValue::Null,
    })
}

// you can remove this section if not using TS_VECTOR (or other types requiring an intermediary `FromSQL` struct)
struct StringCollector(String);
impl FromSql<'_> for StringCollector {
    fn from_sql(
        _: &Type,
        raw: &[u8],
    ) -> Result<StringCollector, Box<dyn std::error::Error + Sync + Send>> {
        let result = std::str::from_utf8(raw)?;
        Ok(StringCollector(result.to_owned()))
    }
    fn accepts(_ty: &Type) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_naive_date() {
        // chrono's NaiveDate::to_string() format
        let d = parse_naive_date("2024-01-15").unwrap();
        assert_eq!(d.to_string(), "2024-01-15");

        // JS ISO format
        let d = parse_naive_date("2024-01-15T00:00:00.000Z").unwrap();
        assert_eq!(d.to_string(), "2024-01-15");

        // ISO without fractional seconds
        let d = parse_naive_date("2024-01-15T00:00:00Z").unwrap();
        assert_eq!(d.to_string(), "2024-01-15");
    }

    #[test]
    fn test_parse_naive_time() {
        // chrono's NaiveTime::to_string() format
        let t = parse_naive_time("10:30:00").unwrap();
        assert_eq!(t.to_string(), "10:30:00");

        // With fractional seconds
        let t = parse_naive_time("10:30:00.123456").unwrap();
        assert_eq!(t.to_string(), "10:30:00.123456");

        // Short format
        let t = parse_naive_time("10:30").unwrap();
        assert_eq!(t.to_string(), "10:30:00");

        // From full datetime string (JS frontend)
        let t = parse_naive_time("1970-01-01T10:30:00.000Z").unwrap();
        assert_eq!(t.to_string(), "10:30:00");
    }

    #[test]
    fn test_parse_naive_datetime() {
        // chrono's NaiveDateTime::to_string() format
        let dt = parse_naive_datetime("2024-01-15 10:30:00").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00");

        // With fractional seconds
        let dt = parse_naive_datetime("2024-01-15 10:30:00.123456").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00.123456");

        // ISO format with Z
        let dt = parse_naive_datetime("2024-01-15T10:30:00.000Z").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00");

        // ISO format without Z
        let dt = parse_naive_datetime("2024-01-15T10:30:00.000").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00");

        // ISO without fractional seconds
        let dt = parse_naive_datetime("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00");
    }

    #[test]
    fn test_parse_datetime_utc() {
        // chrono's DateTime<Utc>::to_string() format
        let dt = parse_datetime_utc("2024-01-15 10:30:00 UTC").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00 UTC");

        // With fractional seconds
        let dt = parse_datetime_utc("2024-01-15 10:30:00.123456 UTC").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00.123456 UTC");

        // RFC 3339
        let dt = parse_datetime_utc("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00 UTC");

        // RFC 3339 with fractional seconds
        let dt = parse_datetime_utc("2024-01-15T10:30:00.123Z").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00.123 UTC");

        // Numeric timezone offset (PostgreSQL text representation)
        let dt = parse_datetime_utc("2026-04-14 18:09:00+00").unwrap();
        assert_eq!(dt.to_string(), "2026-04-14 18:09:00 UTC");

        // Numeric timezone offset with fractional seconds
        let dt = parse_datetime_utc("2024-01-15 10:30:00.123+02").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 08:30:00.123 UTC");

        // Full offset format +00:00
        let dt = parse_datetime_utc("2024-01-15 10:30:00+00:00").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00 UTC");

        // ISO without timezone (treated as UTC)
        let dt = parse_datetime_utc("2024-01-15T10:30:00.000").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 10:30:00 UTC");
    }

    #[test]
    fn test_roundtrip_timestamp_formats() {
        // Verify that the format produced by pg_cell_to_json_value can be parsed back
        let original = chrono::NaiveDateTime::parse_from_str(
            "2024-06-15 14:30:45.123",
            "%Y-%m-%d %H:%M:%S%.f",
        )
        .unwrap();
        let serialized = original.to_string();
        let parsed = parse_naive_datetime(&serialized).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_roundtrip_timestamptz_formats() {
        let original = "2024-06-15T14:30:45Z"
            .parse::<chrono::DateTime<Utc>>()
            .unwrap();
        let serialized = original.to_string();
        let parsed = parse_datetime_utc(&serialized).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_roundtrip_time_formats() {
        let original = chrono::NaiveTime::parse_from_str("14:30:45.123", "%H:%M:%S%.f").unwrap();
        let serialized = original.to_string();
        let parsed = parse_naive_time(&serialized).unwrap();
        assert_eq!(original, parsed);
    }

    // ---------------------------------------------------------------------
    // convert_val: exhaustive (Value × otyp) → Type matrix.
    //
    // For every (JSON Value, parser otyp) combination that can occur from
    // either windmill-client SDK output (TS or Python) or a hand-written
    // Postgres script, verify that convert_val returns a `(Box, Type)` pair
    // where the Type matches the Box's concrete Rust type. This is the core
    // invariant that makes `query_typed_raw` safe — if it ever drifts again
    // (the bug introduced by #8988), users get
    // `cannot convert between the Rust type X and the Postgres type Y`.
    //
    // We can't introspect the Box's Rust type at runtime, but we *can* feed
    // each (Box, Type) through `to_sql_checked` against the asserted Type —
    // that's exactly the codepath `query_typed_raw` uses, so any mismatch
    // surfaces here as a `ToSql` error.
    // ---------------------------------------------------------------------

    use bytes::BytesMut;
    use serde_json::json;
    use tokio_postgres::types::IsNull;

    /// Verify that convert_val for `(value, arg_t)` returns a binding whose
    /// Rust type matches the asserted Postgres `Type` — exactly the check
    /// `query_typed_raw` performs when serialising parameters.
    ///
    /// Defaults to `otyp_inferred = false` (= "user explicitly typed this").
    /// Tests that need the parser-default flavour use
    /// `assert_convert_val_consistent_inferred`.
    fn assert_convert_val_consistent(
        label: &str,
        value: Value,
        arg_t: &str,
        typ: Typ,
        expected_type: Type,
    ) {
        assert_convert_val_consistent_full(label, value, arg_t, typ, expected_type, false)
    }

    fn assert_convert_val_consistent_inferred(
        label: &str,
        value: Value,
        arg_t: &str,
        typ: Typ,
        expected_type: Type,
    ) {
        assert_convert_val_consistent_full(label, value, arg_t, typ, expected_type, true)
    }

    fn assert_convert_val_consistent_full(
        label: &str,
        value: Value,
        arg_t: &str,
        typ: Typ,
        expected_type: Type,
        otyp_inferred: bool,
    ) {
        let (boxed, t) = convert_val(&value, &arg_t.to_string(), &typ, otyp_inferred)
            .unwrap_or_else(|e| panic!("{label}: convert_val errored: {e}"));
        assert_eq!(
            t, expected_type,
            "{label}: expected Type {expected_type}, got {t}"
        );
        // Run the encoder check — this is what query_typed_raw does internally
        // when binding the param. A mismatch between the boxed Rust type and
        // the asserted Type fails here as a `WrongType` error.
        let mut buf = BytesMut::new();
        match boxed.to_sql_checked(&t, &mut buf) {
            Ok(IsNull::Yes) | Ok(IsNull::No) => {}
            Err(e) => panic!(
                "{label}: ToSql failed for value={value:?} arg_t={arg_t} (asserted {t}): {e}"
            ),
        }
    }

    fn typ_for(arg_t: &str) -> Typ {
        windmill_parser_sql::parse_pg_typ(arg_t)
    }

    #[test]
    fn convert_val_null_for_every_known_arg_t() {
        // `Value::Null` for every type the parser may resolve, plus an unknown
        // arg_t (custom enum / extension). Each must produce a matching Type
        // and serialise without error.
        let cases: &[(&str, Type)] = &[
            ("bool", Type::BOOL),
            ("boolean", Type::BOOL),
            ("char", Type::CHAR),
            ("character", Type::CHAR),
            ("smallint", Type::INT2),
            ("int2", Type::INT2),
            ("smallserial", Type::INT2),
            ("serial2", Type::INT2),
            ("int", Type::INT4),
            ("integer", Type::INT4),
            ("int4", Type::INT4),
            ("serial", Type::INT4),
            ("bigint", Type::INT8),
            ("int8", Type::INT8),
            ("bigserial", Type::INT8),
            ("serial8", Type::INT8),
            ("real", Type::FLOAT4),
            ("float4", Type::FLOAT4),
            ("double", Type::FLOAT8),
            ("double precision", Type::FLOAT8),
            ("float8", Type::FLOAT8),
            ("numeric", Type::NUMERIC),
            ("decimal", Type::NUMERIC),
            ("oid", Type::OID),
            ("uuid", Type::UUID),
            ("date", Type::DATE),
            ("time", Type::TIME),
            // chrono::NaiveTime can only encode as TIME — see the Null arm.
            ("timetz", Type::TIME),
            ("timestamp", Type::TIMESTAMP),
            ("timestamptz", Type::TIMESTAMPTZ),
            ("json", Type::JSON),
            ("jsonb", Type::JSONB),
            ("bytea", Type::BYTEA),
            ("text", Type::TEXT),
            ("varchar", Type::VARCHAR),
            ("character varying", Type::VARCHAR),
            // Unknown / custom type: convert_val falls back to TEXT NULL — the
            // dispatch then takes the prepare + query_raw path so the server
            // resolves the actual column type.
            ("my_custom_enum", Type::TEXT),
        ];
        for (arg_t, expected) in cases {
            assert_convert_val_consistent(
                &format!("Null/{arg_t}"),
                Value::Null,
                arg_t,
                typ_for(arg_t),
                expected.clone(),
            );
        }
    }

    #[test]
    fn convert_val_bool_against_every_arg_t() {
        // Value::Bool. Pre-#8988 this always produced Box<bool>, which
        // mismatched the asserted Type for parser-default "text" (the
        // regression we fix). Post-fix:
        //  - explicit text-like target (`-- $1 (text)` / `$1::text`):
        //    coerce to Box<String>+TEXT so `WHERE text_col = $1` works.
        //  - parser-default text (bare `$N`, no annotation): bind as BOOL
        //    natively, server casts at the use site.
        //  - any other target: bind as BOOL.
        let bool_targets = ["bool", "boolean"];
        let json_targets = [("json", Type::JSON), ("jsonb", Type::JSONB)];
        let explicit_text_targets = [
            ("text", Type::TEXT),
            ("varchar", Type::VARCHAR),
            ("character varying", Type::VARCHAR),
        ];
        let bind_as_bool = [
            "smallint",
            "int",
            "integer",
            "bigint",
            "int4",
            "int8",
            "real",
            "double",
            "double precision",
            "numeric",
            "uuid",
            "date",
            "time",
            "timestamp",
            "timestamptz",
            "bytea",
            "oid",
            // unknown — server resolves via prepare path
            "my_custom_enum",
        ];

        for v in [true, false] {
            for arg_t in &bool_targets {
                assert_convert_val_consistent(
                    &format!("Bool({v})/{arg_t}"),
                    Value::Bool(v),
                    arg_t,
                    typ_for(arg_t),
                    Type::BOOL,
                );
            }
            for (arg_t, expected) in &json_targets {
                assert_convert_val_consistent(
                    &format!("Bool({v})/{arg_t}"),
                    Value::Bool(v),
                    arg_t,
                    typ_for(arg_t),
                    expected.clone(),
                );
            }
            for (arg_t, expected) in &explicit_text_targets {
                // Explicit (otyp_inferred=false): coerce to text.
                assert_convert_val_consistent(
                    &format!("Bool({v})/{arg_t} explicit"),
                    Value::Bool(v),
                    arg_t,
                    typ_for(arg_t),
                    expected.clone(),
                );
                // Inferred (parser-default): keep BOOL.
                assert_convert_val_consistent_inferred(
                    &format!("Bool({v})/{arg_t} inferred"),
                    Value::Bool(v),
                    arg_t,
                    typ_for(arg_t),
                    Type::BOOL,
                );
            }
            // `char` and `character` (= bpchar) are single-byte / fixed-width
            // text. Explicit decl with a JSON bool errors with an actionable
            // hint instead of silently binding BOOL (which a CHAR column
            // can't compare against). Inferred-default still binds BOOL.
            for arg_t in &["char", "character"] {
                let err = convert_val(
                    &Value::Bool(v),
                    &arg_t.to_string(),
                    &typ_for(arg_t),
                    /* otyp_inferred = */ false,
                )
                .err()
                .unwrap_or_else(|| panic!("Bool({v})/{arg_t} explicit should error"));
                let msg = err.to_string();
                assert!(
                    msg.contains("Cannot bind a JSON bool"),
                    "Bool({v})/{arg_t} explicit error didn't have expected message: {msg}"
                );
                assert_convert_val_consistent_inferred(
                    &format!("Bool({v})/{arg_t} inferred"),
                    Value::Bool(v),
                    arg_t,
                    typ_for(arg_t),
                    Type::BOOL,
                );
            }
            for arg_t in &bind_as_bool {
                assert_convert_val_consistent(
                    &format!("Bool({v})/{arg_t}"),
                    Value::Bool(v),
                    arg_t,
                    typ_for(arg_t),
                    Type::BOOL,
                );
            }
        }
    }

    #[test]
    fn convert_val_integer_number_against_every_arg_t() {
        // JSON integers. Each arg_t selects its matching encoder; for arg_ts
        // that don't have a numeric encoder (uuid, date, …), the value falls
        // through to the generic Number arm — Box<i64> bound as INT8 — and
        // server-side casts handle the rest if the SQL wants it.
        let cases: &[(&str, Type)] = &[
            ("char", Type::CHAR),
            // "character" (= bpchar in PG) doesn't have a Number arm, so it
            // falls through to generic Number → Box<i64> + INT8. Server
            // casts at the SQL site if the column is bpchar.
            ("character", Type::INT8),
            ("smallint", Type::INT2),
            ("smallserial", Type::INT2),
            ("int2", Type::INT2),
            ("serial2", Type::INT2),
            ("int", Type::INT4),
            ("integer", Type::INT4),
            ("int4", Type::INT4),
            ("serial", Type::INT4),
            ("bigint", Type::INT8),
            ("bigserial", Type::INT8),
            ("int8", Type::INT8),
            ("serial8", Type::INT8),
            ("oid", Type::OID),
            ("numeric", Type::NUMERIC),
            ("decimal", Type::NUMERIC),
            // Unknown arg_t falls through to generic Number → INT8.
            ("my_custom_enum", Type::INT8),
        ];
        for (arg_t, expected) in cases {
            assert_convert_val_consistent(
                &format!("Number(42)/{arg_t}"),
                json!(42),
                arg_t,
                typ_for(arg_t),
                expected.clone(),
            );
        }
        // Text targets: split between explicit (coerce to TEXT) and inferred
        // (parser-default, bind as INT8 — server casts at the use site).
        for (arg_t, expected_text) in [("text", Type::TEXT), ("varchar", Type::VARCHAR)] {
            assert_convert_val_consistent(
                &format!("Number(42)/{arg_t} explicit"),
                json!(42),
                arg_t,
                typ_for(arg_t),
                expected_text,
            );
            assert_convert_val_consistent_inferred(
                &format!("Number(42)/{arg_t} inferred"),
                json!(42),
                arg_t,
                typ_for(arg_t),
                Type::INT8,
            );
        }
        // Negative integer (is_u64 false → falls to generic i64 arm for bigint).
        assert_convert_val_consistent(
            "Number(-7)/bigint",
            json!(-7),
            "bigint",
            typ_for("bigint"),
            Type::INT8,
        );
        assert_convert_val_consistent("Number(0)/oid", json!(0), "oid", typ_for("oid"), Type::OID);
    }

    #[test]
    fn convert_val_float_number_against_every_arg_t() {
        let cases: &[(&str, Type)] = &[
            ("real", Type::FLOAT4),
            ("float4", Type::FLOAT4),
            ("double", Type::FLOAT8),
            ("double precision", Type::FLOAT8),
            ("float8", Type::FLOAT8),
            ("numeric", Type::NUMERIC),
            ("decimal", Type::NUMERIC),
            // Unknown arg_t falls through to generic → Box<f64>+FLOAT8.
            ("my_custom_enum", Type::FLOAT8),
        ];
        for (arg_t, expected) in cases {
            assert_convert_val_consistent(
                &format!("Number(3.14)/{arg_t}"),
                json!(3.14),
                arg_t,
                typ_for(arg_t),
                expected.clone(),
            );
        }
        // Text targets: split between explicit (coerce to TEXT/VARCHAR) and
        // inferred (parser-default, bind as FLOAT8 — server casts at use site).
        for (arg_t, expected_text) in [("text", Type::TEXT), ("varchar", Type::VARCHAR)] {
            assert_convert_val_consistent(
                &format!("Number(3.14)/{arg_t} explicit"),
                json!(3.14),
                arg_t,
                typ_for(arg_t),
                expected_text,
            );
            assert_convert_val_consistent_inferred(
                &format!("Number(3.14)/{arg_t} inferred"),
                json!(3.14),
                arg_t,
                typ_for(arg_t),
                Type::FLOAT8,
            );
        }
    }

    #[test]
    fn convert_val_string_against_every_arg_t() {
        // Strings parse into the matching Rust type when arg_t resolves to a
        // numeric / temporal / uuid / bytea type; otherwise they bind as TEXT.
        assert_convert_val_consistent(
            "String('42')/smallint",
            json!("42"),
            "smallint",
            typ_for("smallint"),
            Type::INT2,
        );
        assert_convert_val_consistent(
            "String('42')/int",
            json!("42"),
            "int",
            typ_for("int"),
            Type::INT4,
        );
        assert_convert_val_consistent(
            "String('42')/bigint",
            json!("42"),
            "bigint",
            typ_for("bigint"),
            Type::INT8,
        );
        assert_convert_val_consistent(
            "String(uuid)/uuid",
            json!("550e8400-e29b-41d4-a716-446655440000"),
            "uuid",
            typ_for("uuid"),
            Type::UUID,
        );
        assert_convert_val_consistent(
            "String(date)/date",
            json!("2024-01-15"),
            "date",
            typ_for("date"),
            Type::DATE,
        );
        assert_convert_val_consistent(
            "String(time)/time",
            json!("10:30:00"),
            "time",
            typ_for("time"),
            Type::TIME,
        );
        assert_convert_val_consistent(
            "String(time)/timetz",
            json!("10:30:00"),
            "timetz",
            typ_for("timetz"),
            Type::TIME,
        );
        assert_convert_val_consistent(
            "String(ts)/timestamp",
            json!("2024-01-15T10:30:00"),
            "timestamp",
            typ_for("timestamp"),
            Type::TIMESTAMP,
        );
        assert_convert_val_consistent(
            "String(tstz)/timestamptz",
            json!("2024-01-15T10:30:00Z"),
            "timestamptz",
            typ_for("timestamptz"),
            Type::TIMESTAMPTZ,
        );
        assert_convert_val_consistent(
            "String(b64)/bytea",
            json!("aGVsbG8="),
            "bytea",
            typ_for("bytea"),
            Type::BYTEA,
        );
        // Generic text arms.
        for (arg_t, expected) in [
            ("text", Type::TEXT),
            ("varchar", Type::VARCHAR),
            ("character varying", Type::VARCHAR),
            // Unknown → TEXT (prepare fallback in dispatch).
            ("my_custom_enum", Type::TEXT),
        ] {
            assert_convert_val_consistent(
                &format!("String('hello')/{arg_t}"),
                json!("hello"),
                arg_t,
                typ_for(arg_t),
                expected,
            );
        }
    }

    #[test]
    fn convert_val_object_against_every_arg_t() {
        // Object values: bind as JSONB (default), JSON if explicitly typed,
        // or JSON-stringify into TEXT/VARCHAR when arg_t is text-like.
        assert_convert_val_consistent(
            "Object/jsonb",
            json!({"k": 1}),
            "jsonb",
            typ_for("jsonb"),
            Type::JSONB,
        );
        assert_convert_val_consistent(
            "Object/json",
            json!({"k": 1}),
            "json",
            typ_for("json"),
            Type::JSON,
        );
        assert_convert_val_consistent(
            "Object/text",
            json!({"k": 1}),
            "text",
            typ_for("text"),
            Type::TEXT,
        );
        assert_convert_val_consistent(
            "Object/varchar",
            json!({"k": 1}),
            "varchar",
            typ_for("varchar"),
            Type::VARCHAR,
        );
        // Parser-default text (Typ::Str) still routes to TEXT-string via the
        // `matches!(typ, Typ::Str(_))` arm.
        assert_convert_val_consistent(
            "Object/parser-default-text",
            json!({"k": 1}),
            "text",
            Typ::Str(None),
            Type::TEXT,
        );
        // Unknown arg_t parses to Typ::Str (parser's catch-all), so the
        // text-coercion arm picks it up — Box<String> + TEXT. The dispatch
        // then takes the prepare + query_raw path because otyp_to_pg_type
        // returns Err for the unknown name, letting the server resolve the
        // actual column type (e.g. a custom enum that accepts JSON via cast).
        assert_convert_val_consistent(
            "Object/my_custom_enum",
            json!({"k": 1}),
            "my_custom_enum",
            typ_for("my_custom_enum"),
            Type::TEXT,
        );
    }

    #[test]
    fn convert_val_array_against_every_arg_t() {
        // arg_t with [] suffix routes to convert_vec_val.
        let int_array_cases: &[(&str, Type)] = &[
            ("int[]", Type::INT4_ARRAY),
            ("integer[]", Type::INT4_ARRAY),
            ("int4[]", Type::INT4_ARRAY),
            ("smallint[]", Type::INT2_ARRAY),
            ("bigint[]", Type::INT8_ARRAY),
        ];
        for (arg_t, expected) in int_array_cases {
            assert_convert_val_consistent(
                &format!("Array([1,2])/{arg_t}"),
                json!([1, 2]),
                arg_t,
                typ_for(arg_t),
                expected.clone(),
            );
        }
        assert_convert_val_consistent(
            "Array(strs)/text[]",
            json!(["a", "b"]),
            "text[]",
            typ_for("text[]"),
            Type::TEXT_ARRAY,
        );
        assert_convert_val_consistent(
            "Array(strs)/varchar[]",
            json!(["a", "b"]),
            "varchar[]",
            typ_for("varchar[]"),
            Type::VARCHAR_ARRAY,
        );
        assert_convert_val_consistent(
            "Array(bools)/bool[]",
            json!([true, false]),
            "bool[]",
            typ_for("bool[]"),
            Type::BOOL_ARRAY,
        );
        assert_convert_val_consistent(
            "Array(floats)/double[]",
            json!([1.5, 2.5]),
            "double[]",
            typ_for("double[]"),
            Type::FLOAT8_ARRAY,
        );
        assert_convert_val_consistent(
            "Array(uuids)/uuid[]",
            json!(["550e8400-e29b-41d4-a716-446655440000"]),
            "uuid[]",
            typ_for("uuid[]"),
            Type::UUID_ARRAY,
        );
        // `timetz[]` falls back to TIME_ARRAY for the same reason the scalar
        // `timetz` falls back to TIME — chrono's `NaiveTime` only encodes for
        // TIME. The encoder check (to_sql_checked) catches a mistakenly
        // asserted TIMETZ_ARRAY here.
        assert_convert_val_consistent(
            "Array(times)/timetz[] → TIME_ARRAY",
            json!(["10:30:00", "11:00:00"]),
            "timetz[]",
            typ_for("timetz[]"),
            Type::TIME_ARRAY,
        );
        assert_convert_val_consistent(
            "Array(times)/time[]",
            json!(["10:30:00", "11:00:00"]),
            "time[]",
            typ_for("time[]"),
            Type::TIME_ARRAY,
        );
        // Array without [] suffix on arg_t: bind as JSONB (or JSON / TEXT).
        assert_convert_val_consistent(
            "Array/jsonb (no [])",
            json!([1, 2, 3]),
            "jsonb",
            typ_for("jsonb"),
            Type::JSONB,
        );
        assert_convert_val_consistent(
            "Array/json (no [])",
            json!([1, 2, 3]),
            "json",
            typ_for("json"),
            Type::JSON,
        );
        assert_convert_val_consistent(
            "Array/text (parser-default)",
            json!([1, 2, 3]),
            "text",
            typ_for("text"),
            Type::TEXT,
        );
        // See Object/my_custom_enum: Typ::Str catch-all → TEXT-stringify;
        // dispatch falls back to prepare for the unknown arg_t.
        assert_convert_val_consistent(
            "Array/my_custom_enum",
            json!([1, 2, 3]),
            "my_custom_enum",
            typ_for("my_custom_enum"),
            Type::TEXT,
        );
        // NULL-array shape: Value::Null with arg_t ending in []
        assert_convert_val_consistent(
            "Null/int[]",
            Value::Null,
            "int[]",
            typ_for("int[]"),
            Type::INT4_ARRAY,
        );
    }

    /// Edge cases mirroring what the SDKs (TS / Python) and hand-written PG
    /// scripts can actually emit. Each case is a real input → encode round
    /// trip, and would have failed under #8988 if the asserted Type drifted
    /// from the binding's Rust type.
    #[test]
    fn convert_val_sdk_edge_cases() {
        // TS SDK shapes — `${val}` is auto-tagged with ::TYPE.
        assert_convert_val_consistent(
            "TS SDK ${42}",
            json!(42),
            "bigint",
            typ_for("bigint"),
            Type::INT8,
        );
        assert_convert_val_consistent(
            "TS SDK ${3.14}",
            json!(3.14),
            "double",
            typ_for("double"),
            Type::FLOAT8,
        );
        assert_convert_val_consistent(
            "TS SDK ${true}",
            json!(true),
            "boolean",
            typ_for("boolean"),
            Type::BOOL,
        );
        assert_convert_val_consistent(
            "TS SDK ${\"hello\"}",
            json!("hello"),
            "text",
            typ_for("text"),
            Type::TEXT,
        );
        assert_convert_val_consistent(
            "TS SDK ${{x:1}}",
            json!({"x": 1}),
            "json",
            typ_for("json"),
            Type::JSON,
        );
        assert_convert_val_consistent(
            "TS SDK ${[1,2,3]}",
            json!([1, 2, 3]),
            "json",
            typ_for("json"),
            Type::JSON,
        );
        assert_convert_val_consistent(
            "TS SDK ${null}",
            Value::Null,
            "text",
            typ_for("text"),
            Type::TEXT,
        );

        // CAST(${val} AS T) shape — the SDK strips its own ::TYPE here, so
        // the parser sees a bare $N and otyp defaults to "text" *with
        // otyp_inferred = true*. This is the original regression #8988
        // introduced; the inferred-default flag is what lets convert_val
        // bind the value's natural type rather than coerce to TEXT.
        assert_convert_val_consistent_inferred(
            "CAST AS bool / Bool true",
            json!(true),
            "text",
            typ_for("text"),
            Type::BOOL,
        );
        assert_convert_val_consistent_inferred(
            "CAST AS bool / Bool false",
            json!(false),
            "text",
            typ_for("text"),
            Type::BOOL,
        );
        // Object falls into the text-coercion arm (Object branch checks
        // `matches!(typ, Typ::Str(_))` regardless of otyp_inferred — JSON
        // serialisation is always safer than asserting JSONB for an
        // unannotated arg).
        assert_convert_val_consistent_inferred(
            "CAST AS jsonb / Object",
            json!({"a": 1, "b": [2, 3]}),
            "text",
            typ_for("text"),
            Type::TEXT,
        );
        assert_convert_val_consistent_inferred(
            "CAST AS int / Number",
            json!(7),
            "text",
            typ_for("text"),
            Type::INT8,
        );

        // Python SDK datatable shape — type sits in the declaration comment
        // (`-- $1 arg1 (BIGINT)`). Parser resolves otyp before convert_val.
        assert_convert_val_consistent(
            "Python decl bigint / Number",
            json!(42),
            "bigint",
            typ_for("bigint"),
            Type::INT8,
        );
        assert_convert_val_consistent(
            "Python decl text / String",
            json!("hello"),
            "text",
            typ_for("text"),
            Type::TEXT,
        );
        assert_convert_val_consistent(
            "Python decl jsonb / Object",
            json!({"k": [1, 2]}),
            "jsonb",
            typ_for("jsonb"),
            Type::JSONB,
        );

        // Mismatched-but-coercible JSON shape: JSON int 0/1 into a bool col
        // still works because tokio_postgres encodes Number as INT8 and
        // postgres has int→bool cast at the SQL site. Uses the inferred
        // path (parser-default text otyp).
        assert_convert_val_consistent_inferred(
            "Number(0)/bool (parser default)",
            json!(0),
            "text",
            typ_for("text"),
            Type::INT8,
        );
    }

    /// `decimal_fits_f64_losslessly` returns true for values that round-trip
    /// through f64 and false for values that don't. This is the predicate
    /// behind the one-shot precision-loss warning.
    #[test]
    fn decimal_fits_f64_losslessly_predicate() {
        use std::str::FromStr;
        // Values that fit f64 cleanly:
        for s in &["0", "1", "-1", "3.14", "1234.5", "-0.5", "10000000000"] {
            let d = Decimal::from_str(s).unwrap();
            assert!(
                decimal_fits_f64_losslessly(&d),
                "expected `{s}` to fit f64 losslessly"
            );
        }
        // Values past f64's ~15 significant-digit window lose precision:
        for s in &[
            "12345678901234.56789", // 19 sig digits
            "0.123456789012345678", // 18 sig digits past the decimal
            "99999999999999999999", // 20-digit integer
        ] {
            let d = Decimal::from_str(s).unwrap();
            assert!(
                !decimal_fits_f64_losslessly(&d),
                "expected `{s}` to NOT fit f64 losslessly"
            );
        }
    }

    /// `should_check_precision` returns `true` exactly
    /// `NUMERIC_PRECISION_CHECK_BUDGET` times, then `false` forever — and
    /// `false` immediately once the precision-loss flag has been set, so the
    /// hot path on a numeric-heavy result set is one cheap atomic load after
    /// the first lossy value is observed.
    #[test]
    fn precision_check_budget_caps_per_query_overhead() {
        use std::sync::atomic::Ordering;
        let state = ResultFormatState::default();
        let mut allowed = 0u32;
        let mut denied = 0u32;
        for _ in 0..(NUMERIC_PRECISION_CHECK_BUDGET + 100) {
            if state.should_check_precision() {
                allowed += 1;
            } else {
                denied += 1;
            }
        }
        assert_eq!(allowed, NUMERIC_PRECISION_CHECK_BUDGET);
        assert_eq!(denied, 100);
        // The flag short-circuits the budget — once set, no more checks run
        // even if the budget hadn't been spent.
        let state = ResultFormatState::default();
        state.numeric_precision_loss.store(true, Ordering::Relaxed);
        for _ in 0..10 {
            assert!(!state.should_check_precision());
        }
        // Budget untouched.
        assert_eq!(
            state.numeric_precision_check_budget.load(Ordering::Relaxed),
            NUMERIC_PRECISION_CHECK_BUDGET
        );
    }

    /// Sparse positional placeholders renumber to a contiguous 1..=N without
    /// substring collisions OR mangling string-literal/comment occurrences.
    /// The pre-existing `String::replace` chain turned `$50` into `$10` when
    /// oidx=5 was processed first; even the regex-with-greedy-digits approach
    /// (a regression of its own) walked through string literals. The current
    /// position-aware rewrite uses the parser's tokenizer to skip those.
    #[test]
    fn renumber_sparse_placeholders_no_collision_no_string_mangling() {
        fn renumber(input: &str, mapping: &HashMap<i32, usize>) -> String {
            let mut out = input.to_owned();
            let mut positions = windmill_parser_sql::parse_pg_statement_arg_positions(input);
            positions.sort_by_key(|(_, range)| std::cmp::Reverse(range.start));
            for (oidx, range) in positions {
                if let Some(new_i) = mapping.get(&oidx) {
                    if oidx as usize != *new_i {
                        out.replace_range(range, &new_i.to_string());
                    }
                }
            }
            out
        }

        let mapping: HashMap<i32, usize> = [(5, 1), (50, 2)].into_iter().collect();
        let cases = &[
            // Two placeholders, full rewrite (greedy-digit collision check).
            ("SELECT $5, $50", "SELECT $1, $2"),
            // Same input flipped — order independence.
            ("SELECT $50, $5", "SELECT $2, $1"),
            // Repeat use of an index — every site gets rewritten.
            (
                "SELECT $5 FROM t WHERE id = $5 OR ref = $50",
                "SELECT $1 FROM t WHERE id = $1 OR ref = $2",
            ),
            // Index outside the mapping is left intact.
            ("SELECT $5, $99", "SELECT $1, $99"),
            // String literal containing the same `$N` syntax must not be
            // rewritten — the tokenizer marks it as inside a string.
            (
                "SELECT 'price: $5' AS lbl, $5 FROM t",
                "SELECT 'price: $5' AS lbl, $1 FROM t",
            ),
            // Single-line comment must not be rewritten either.
            ("-- mention $5\nSELECT $5", "-- mention $5\nSELECT $1"),
            // Dollar-quoted block ($$ … $$) must not be rewritten.
            ("SELECT $$body with $5$$, $5", "SELECT $$body with $5$$, $1"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                renumber(input, &mapping).as_str(),
                *expected,
                "input={input}"
            );
        }
    }

    /// Drift-prevention: `otyp_to_pg_type` and `convert_val` must agree on the
    /// Type for every recognised arg_t when the JSON value matches the arg_t's
    /// "natural" Rust kind. Fails if someone adds a new arg_t to one but not
    /// the other, or changes the Type returned by either.
    #[test]
    fn otyp_to_pg_type_and_convert_val_agree_for_recognised_types() {
        // (arg_t, natural-value, expected scalar Type)
        let cases: &[(&str, Value, Type)] = &[
            ("bool", json!(true), Type::BOOL),
            ("boolean", json!(false), Type::BOOL),
            ("char", json!(65), Type::CHAR),
            ("smallint", json!(1), Type::INT2),
            ("smallserial", json!(1), Type::INT2),
            ("int2", json!(1), Type::INT2),
            ("serial2", json!(1), Type::INT2),
            ("int", json!(1), Type::INT4),
            ("integer", json!(1), Type::INT4),
            ("int4", json!(1), Type::INT4),
            ("serial", json!(1), Type::INT4),
            ("bigint", json!(1), Type::INT8),
            ("int8", json!(1), Type::INT8),
            ("bigserial", json!(1), Type::INT8),
            ("serial8", json!(1), Type::INT8),
            ("real", json!(1.5), Type::FLOAT4),
            ("float4", json!(1.5), Type::FLOAT4),
            ("double", json!(1.5), Type::FLOAT8),
            ("double precision", json!(1.5), Type::FLOAT8),
            ("float8", json!(1.5), Type::FLOAT8),
            ("numeric", json!(1), Type::NUMERIC),
            ("decimal", json!(1), Type::NUMERIC),
            ("oid", json!(1), Type::OID),
            (
                "uuid",
                json!("550e8400-e29b-41d4-a716-446655440000"),
                Type::UUID,
            ),
            ("date", json!("2024-01-15"), Type::DATE),
            ("time", json!("10:30:00"), Type::TIME),
            // chrono::NaiveTime can only encode TIME — see the timetz arm.
            ("timetz", json!("10:30:00"), Type::TIME),
            ("timestamp", json!("2024-01-15T10:30:00"), Type::TIMESTAMP),
            (
                "timestamptz",
                json!("2024-01-15T10:30:00Z"),
                Type::TIMESTAMPTZ,
            ),
            ("json", json!({"k": 1}), Type::JSON),
            ("jsonb", json!({"k": 1}), Type::JSONB),
            ("bytea", json!("aGVsbG8="), Type::BYTEA),
            ("text", json!("hello"), Type::TEXT),
            ("varchar", json!("hello"), Type::VARCHAR),
            ("character varying", json!("hello"), Type::VARCHAR),
        ];
        for (arg_t, value, expected) in cases {
            // 1. The dispatch's "is recognised" gate must accept this arg_t.
            //    `timetz` is the one exception where we deliberately return
            //    TIME from convert_val (chrono limitation), but otyp_to_pg_type
            //    returns TIMETZ.
            let from_otyp = otyp_to_pg_type(arg_t)
                .unwrap_or_else(|e| panic!("otyp_to_pg_type lost arg_t `{arg_t}`: {e}"));
            if *arg_t != "timetz" {
                assert_eq!(
                    from_otyp, *expected,
                    "otyp_to_pg_type({arg_t}) drift: expected {expected}, got {from_otyp}"
                );
            }
            // 2. convert_val must produce a binding whose Type matches
            //    `expected`, AND whose Rust type successfully encodes against
            //    that Type (the to_sql_checked round-trip).
            assert_convert_val_consistent(
                &format!("meta/{arg_t}"),
                value.clone(),
                arg_t,
                typ_for(arg_t),
                expected.clone(),
            );
        }
    }
}
