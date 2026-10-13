//! Data tables, DuckLake and database resource schemas.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{json, Map, Value};

use super::runs::{backgrounded_summary, run_script_preview, wait_budget, wait_job};
use super::workspace::{api_message, path_enc};
use super::{array, enc, js_str, non_null, opt_str_arg, pretty, str_arg, truthy, Ctx, ToolResult};

pub(super) const TOOLS: &[&str] = &[
    "list_datatables",
    "get_datatable_table_schema",
    "exec_datatable_sql",
    "list_data_metrics",
    "list_ducklakes",
    "get_db_schema",
    "list_datatable_migrations",
    "create_datatable_migration",
    "run_datatable_migrations",
    "rollback_datatable_migration",
];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "list_datatables" => Ok(list_datatables(ctx, args)
            .await
            .unwrap_or_else(|e| format!("Error listing datatables: {e}"))),
        "get_datatable_table_schema" => get_datatable_table_schema(ctx, args).await,
        "exec_datatable_sql" => exec_datatable_sql(ctx, args).await,
        "list_data_metrics" => list_data_metrics(ctx, args).await,
        "list_ducklakes" => Ok(list_ducklakes(ctx)
            .await
            .unwrap_or_else(|e| format!("Error listing DuckLake catalogs: {e}"))),
        "get_db_schema" => get_db_schema(ctx, args).await,
        "list_datatable_migrations" => list_datatable_migrations(ctx, args).await,
        "create_datatable_migration" => create_datatable_migration(ctx, args).await,
        "run_datatable_migrations" => run_datatable_migrations(ctx, args).await,
        "rollback_datatable_migration" => rollback_datatable_migration(ctx, args).await,
        _ => unreachable!("{name} is not a data tool"),
    }
}

const NO_DATATABLES_CONFIGURED_MESSAGE: &str = "No datatables are configured in this workspace. Datatable operations (querying data, creating or altering tables) are blocked until a datatable exists. Datatables are not created by SQL — the user must set one up in the workspace settings (Workspace settings → Data Tables) first. Do not call exec_datatable_sql or assume a \"main\" datatable exists; instead tell the user this is a required prerequisite and ask them to configure a datatable in their workspace settings.";

const NO_DUCKLAKES_CONFIGURED_MESSAGE: &str = "No DuckLake catalogs are configured in this workspace. A data pipeline that materializes DuckLake tables or reads/writes S3 assets cannot run until object storage and a DuckLake catalog are set up. You can still draft the pipeline scripts, but tell the user how to enable it by role: a workspace admin adds object storage under Workspace settings → Object Storage (S3/Azure/GCS), then a DuckLake catalog on top of it; a user without admin rights should ask a workspace admin. Do not assume a default 'main' DuckLake exists.";

const NO_DATA_METRICS_NOTE: &str = "Nothing matched. That does not establish that nothing is declared: declarations whose producing script you cannot read are omitted from this list, not flagged. You may write your own aggregate, but tell the user you found no declared measure you can read rather than stating none is declared.";

const DEFAULT_DATA_METRICS_LIMIT: u64 = 200;
/// Rows of a SELECT handed to the model.
const MAX_ROWS: usize = 100;
/// Seconds a query is awaited before it is left running for `get_run`.
const MAX_SCHEMA_LENGTH: usize = 350_000;

/// The worker's error for a data table that is not configured in the workspace.
static DATATABLE_NOT_FOUND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)datatable\s+\S+\s+not found").expect("valid regex"));

fn datatable_not_configured_message(name: &str) -> String {
    format!(
        "Datatable \"{name}\" is not configured in this workspace, so this operation cannot run. \
         Datatables are not created by SQL — they must be set up first by the user in the workspace settings \
         (Workspace settings → Data Tables) before any table can be queried or created. \
         This is a required, blocking prerequisite: do not retry on this or another name. \
         Tell the user they need to configure a datatable (e.g. named \"{name}\") in their workspace settings, then try again."
    )
}

/// The optional `role` argument, held to the rule the server applies to a role name.
fn role_arg(args: &Value) -> Result<Option<&str>, String> {
    let role = opt_str_arg(args, "role");
    match role {
        Some(r)
            if r.is_empty()
                || r.len() > 63
                || !r
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') =>
        {
            Err(format!(
                "Invalid `role` '{r}': only letters, digits, '_' and '-' are allowed (1 to 63 characters)"
            ))
        }
        _ => Ok(role),
    }
}

/// `path?k=v&...` over the parameters that are set.
fn with_query(path: String, params: &[(&str, Option<&str>)]) -> String {
    let query: Vec<String> = params
        .iter()
        .filter_map(|(k, v)| v.map(|v| format!("{k}={}", enc(v))))
        .collect();
    if query.is_empty() {
        path
    } else {
        format!("{path}?{}", query.join("&"))
    }
}

async fn list_datatables(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let role = role_arg(args)?;
    let datatable_name = opt_str_arg(args, "datatable_name");
    if role.is_some() && datatable_name.is_none() {
        return Err("`role` needs `datatable_name`, the datatable it is a role of".to_string());
    }
    let path = with_query(
        ctx.ws("workspaces/list_datatable_tables"),
        &[
            ("datatable_name", datatable_name),
            ("role_for", role.and(datatable_name)),
            ("role", role),
        ],
    );
    let metadata = ctx.get(&path).await?;
    let metadata = array(Some(&metadata));
    if metadata.is_empty() {
        return Ok(NO_DATATABLES_CONFIGURED_MESSAGE.to_string());
    }
    let listed: Vec<Value> = metadata
        .iter()
        .map(|d| {
            let mut out = Map::new();
            out.insert("datatable_name".into(), d["datatable_name"].clone());
            out.insert("schemas".into(), d["schemas"].clone());
            if truthy(d.get("error")) {
                out.insert("error".into(), d["error"].clone());
            }
            if truthy(d.get("permissioned")) {
                for key in ["usable_roles", "default_role"] {
                    if let Some(v) = d.get(key) {
                        out.insert(key.into(), v.clone());
                    }
                }
            }
            Value::Object(out)
        })
        .collect();
    Ok(pretty(&Value::Array(listed)))
}

async fn get_datatable_table_schema(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let datatable_name = str_arg(args, "datatable_name")?;
    let schema_name = str_arg(args, "schema_name")?;
    let table_name = str_arg(args, "table_name")?;
    let role = role_arg(args)?;
    let path = with_query(
        ctx.ws("workspaces/get_datatable_table_schema"),
        &[
            ("datatable_name", Some(datatable_name)),
            ("schema_name", Some(schema_name)),
            ("table_name", Some(table_name)),
            ("role", role),
        ],
    );
    Ok(match ctx.get(&path).await {
        Ok(schema) => pretty(&json!({
            "datatable_name": datatable_name,
            "schema_name": schema_name,
            "table_name": table_name,
            "columns": schema["columns"],
        })),
        Err(e) if DATATABLE_NOT_FOUND.is_match(&e) => {
            datatable_not_configured_message(datatable_name)
        }
        Err(e) => format!("Error getting table schema: {e}"),
    })
}

async fn list_ducklakes(ctx: &Ctx<'_>) -> ToolResult {
    let ducklakes = ctx.get(&ctx.ws("workspaces/list_ducklakes")).await?;
    if ducklakes.as_array().is_none_or(Vec::is_empty) {
        return Ok(NO_DUCKLAKES_CONFIGURED_MESSAGE.to_string());
    }
    Ok(pretty(&json!({ "ducklakes": ducklakes })))
}

async fn list_data_metrics(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let limit = match args.get("limit") {
        None | Some(Value::Null) => DEFAULT_DATA_METRICS_LIMIT,
        Some(v) => v
            .as_u64()
            .filter(|n| (1..=1000).contains(n))
            .ok_or("`limit` must be an integer between 1 and 1000")?,
    };
    let table = opt_str_arg(args, "table");
    let path = with_query(
        ctx.ws("data_metrics/list"),
        &[
            ("table", table),
            ("path_prefix", opt_str_arg(args, "path_prefix")),
            ("per_page", Some(&limit.to_string())),
        ],
    );
    let page = ctx.get(&path).await?;
    let metrics = page.get("metrics").cloned().unwrap_or_else(|| json!([]));
    let note = if truthy(page.get("next_cursor")) {
        Some(format!("More declarations exist beyond the first {limit}. Re-call with table/path_prefix to target what you are looking for, or with a higher limit (max 1000) for the rest of the list, rather than concluding a measure is undeclared."))
    } else if metrics.as_array().is_some_and(Vec::is_empty) {
        // The endpoint expands a table filter only once it contains a `/`, so a bare
        // table name matches nothing whatever is declared.
        let bare = table.map(|t| t.replacen("ducklake://", "", 1));
        Some(match (table, bare) {
            (Some(table), Some(bare)) if !bare.is_empty() && !bare.contains('/') => format!("Nothing matched `{table}`: a table filter must name its lake, as `<lake>/{bare}`. Re-call with the lake before concluding anything about what is declared."),
            _ => NO_DATA_METRICS_NOTE.to_string(),
        })
    } else {
        None
    };
    let mut out = Map::new();
    out.insert("metrics".into(), metrics);
    if let Some(note) = note {
        out.insert("note".into(), note.into());
    }
    Ok(pretty(&Value::Object(out)))
}

async fn exec_datatable_sql(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let name = str_arg(args, "datatable_name")?;
    let sql = str_arg(args, "sql")?;
    let role = role_arg(args)?;
    let failed = |e: String| format!("Failed to execute test run: {e}");

    let database = match role {
        None => format!("datatable://{name}"),
        // A stored name holding a `?` is read whole by the server, so a role cannot follow it.
        Some(role) if name.contains('?') => {
            return Err(failed(format!("Data table '{name}' has a '?' in its name, so it can only be used here as its default role. Rename it to connect as role '{role}'.")));
        }
        Some(role) => format!("datatable://{name}?role={role}"),
    };
    let id = run_script_preview(
        ctx,
        None,
        "postgresql",
        sql,
        json!({ "database": database }),
    )
    .await
    .map_err(failed)?;

    match wait_job(ctx, &id, wait_budget(args), false).await {
        Some(job) => Ok(format_datatable_sql_completion(&job, name)),
        None => Ok(backgrounded_summary(&id, &format!("SQL · {name}"))),
    }
}

fn format_datatable_sql_completion(job: &Value, datatable_name: &str) -> String {
    if job["success"] == true {
        let rows = array(job.get("result"));
        let row_count = rows.len();
        return pretty(&if row_count > MAX_ROWS {
            json!({
                "success": true,
                "rowCount": row_count,
                "result": &rows[..MAX_ROWS],
                "note": format!("Showing first {MAX_ROWS} of {row_count} rows"),
            })
        } else {
            json!({ "success": true, "rowCount": row_count, "result": rows })
        });
    }
    let error = match job.get("result") {
        None => json!("Unknown error"),
        Some(result) => match result.pointer("/error/message") {
            Some(message) if !message.is_null() => message.clone(),
            _ => match result {
                Value::String(_) => result.clone(),
                other => Value::String(other.to_string()),
            },
        },
    };
    let error = match error.as_str() {
        Some(raw) if DATATABLE_NOT_FOUND.is_match(raw) => {
            Value::String(datatable_not_configured_message(datatable_name))
        }
        _ => error,
    };
    json!({ "success": false, "error": error }).to_string()
}

/// The optional `version` argument: a migration's timestamp.
fn version_arg(args: &Value) -> Result<Option<i64>, String> {
    match args.get("version") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .or_else(|| v.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
            .map(Some)
            .ok_or_else(|| "`version` must be an integer".to_string()),
    }
}

/// `workspaces/<endpoint>/<datatable>`, the shape of every migration route.
fn migrations_path(ctx: &Ctx<'_>, endpoint: &str, datatable_name: &str) -> String {
    ctx.ws(&format!("workspaces/{endpoint}/{}", enc(datatable_name)))
}

async fn migrations_status(ctx: &Ctx<'_>, datatable_name: &str) -> Result<Value, String> {
    ctx.get(&migrations_path(
        ctx,
        "datatable_migrations_status",
        datatable_name,
    ))
    .await
    .map_err(|e| api_message(&e).to_string())
}

/// `POST` without a body: applying and rolling back take their target from the query.
async fn post_migrations(ctx: &Ctx<'_>, path: &str) -> Result<Value, String> {
    let text = ctx
        .request("POST", path, None)
        .await
        .map_err(|e| api_message(&e).to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("unexpected response: {e}"))
}

fn migration_label(version: &Value, name: &Value) -> String {
    format!("{} ({})", js_str(version), js_str(name))
}

fn labels(migrations: Option<&Value>) -> String {
    array(migrations)
        .iter()
        .map(|m| migration_label(&m["version"], &m["name"]))
        .collect::<Vec<_>>()
        .join(", ")
}

fn migration_status(status: &Value) -> &'static str {
    match status.as_str() {
        Some("ran") => "applied",
        Some("not_run") => "pending",
        _ => "unknown",
    }
}

/// The body of a leading comment line that attempts a `-- role` annotation.
fn role_attempt(line: &str) -> Option<&str> {
    let body = line.strip_prefix("--")?.trim_start();
    let after = body
        .get(..4)
        .filter(|w| w.eq_ignore_ascii_case("role"))
        .map(|_| &body[4..])?;
    match after.chars().next() {
        Some(c) if !(c.is_whitespace() || c == ':' || c == '=') => None,
        _ => Some(after),
    }
}

/// `sql` declaring `role`: the server reads the role a migration runs as from a `-- role <name>`
/// annotation in its leading comment block, so any attempt there is replaced by one on top.
fn with_migration_role(sql: &str, role: Option<&str>) -> String {
    let Some(role) = role else {
        return sql.to_string();
    };
    let lines: Vec<&str> = sql.split('\n').collect();
    let leading = lines
        .iter()
        .position(|l| !l.trim().is_empty() && !l.trim().starts_with("--"))
        .unwrap_or(lines.len());
    let rest = lines[..leading]
        .iter()
        .filter(|l| role_attempt(l.trim()).is_none())
        .chain(&lines[leading..])
        .skip_while(|l| l.trim().is_empty());
    std::iter::once(format!("-- role {role}"))
        .chain(rest.map(|l| l.to_string()))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn list_datatable_migrations(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let datatable_name = str_arg(args, "datatable_name")?;
    let version = version_arg(args)?;
    let status = match migrations_status(ctx, datatable_name).await {
        Ok(status) => status,
        Err(e) => return Ok(format!("Error listing migrations: {e}")),
    };
    if !truthy(status.get("enabled")) {
        return Ok(format!("Migrations are not enabled on datatable \"{datatable_name}\": its schema is changed with plain SQL and there is no history to list. Enabling them is a workspace admin's decision, made in Workspace settings → Data Tables."));
    }
    let migrations = array(status.get("migrations"));
    let Some(version) = version else {
        let mut out = Map::new();
        out.insert(
            "migrations".into(),
            migrations
                .iter()
                .map(|m| {
                    json!({
                        "version": m["timestamp"],
                        "name": m["name"],
                        "status": migration_status(&m["status"]),
                    })
                })
                .collect(),
        );
        if truthy(status.get("error")) {
            out.insert("error".into(), status["error"].clone());
        }
        return Ok(pretty(&Value::Object(out)));
    };
    let Some(migration) = migrations
        .iter()
        .find(|m| m["timestamp"].as_i64() == Some(version))
    else {
        return Ok(format!(
            "No migration {version} on datatable \"{datatable_name}\""
        ));
    };
    let mut out = Map::new();
    out.insert("version".into(), version.into());
    out.insert("name".into(), migration["name"].clone());
    out.insert(
        "status".into(),
        migration_status(&migration["status"]).into(),
    );
    out.insert("code_up".into(), migration["code_up"].clone());
    if let Some(code_down) = non_null(migration.get("code_down")) {
        out.insert("code_down".into(), code_down.clone());
    }
    Ok(pretty(&Value::Object(out)))
}

/// Saves the definition and answers its version, or the message the tool returns instead.
async fn save_migration(
    ctx: &Ctx<'_>,
    datatable_name: &str,
    version: Option<i64>,
    mut definition: Map<String, Value>,
) -> Result<Result<i64, String>, String> {
    let Some(version) = version else {
        let created = ctx
            .post(
                &migrations_path(ctx, "create_datatable_migration", datatable_name),
                Value::Object(definition),
            )
            .await?;
        return created["timestamp"]
            .as_i64()
            .map(Ok)
            .ok_or_else(|| "the created migration has no version".to_string());
    };
    // The server inserts at a version it does not know, which would wedge a new migration
    // into the middle of the history, and stores a down left out as none.
    let status = migrations_status(ctx, datatable_name).await?;
    let existing = array(status.get("migrations"))
        .iter()
        .find(|m| m["timestamp"].as_i64() == Some(version));
    let Some(existing) = existing.filter(|m| m["status"] == "not_run") else {
        return Ok(Err(format!("Migration {version} is not a pending migration of datatable \"{datatable_name}\", so it cannot be rewritten. Omit `version` to add a new migration.")));
    };
    definition.insert("timestamp".into(), version.into());
    if let Some(code_down) = non_null(existing.get("code_down")) {
        definition
            .entry("code_down")
            .or_insert_with(|| code_down.clone());
    }
    ctx.post(
        &migrations_path(ctx, "upsert_datatable_migration", datatable_name),
        Value::Object(definition),
    )
    .await?;
    Ok(Ok(version))
}

/// The labels of the migrations before `version` that are not applied yet.
async fn earlier_pending(
    ctx: &Ctx<'_>,
    datatable_name: &str,
    version: i64,
) -> Result<Vec<String>, String> {
    let status = migrations_status(ctx, datatable_name).await?;
    let earlier: Vec<&Value> = array(status.get("migrations"))
        .iter()
        .filter(|m| m["timestamp"].as_i64().is_some_and(|t| t < version))
        .collect();
    // Unreadable is not pending: reporting it so would send the user to apply migrations
    // that already ran.
    if truthy(status.get("error")) {
        return Err(js_str(&status["error"]));
    }
    if earlier.iter().any(|m| m["status"] == "unknown") {
        return Err("the applied migrations could not be read".to_string());
    }
    Ok(earlier
        .into_iter()
        .filter(|m| m["status"] == "not_run")
        .map(|m| migration_label(&m["timestamp"], &m["name"]))
        .collect())
}

async fn create_datatable_migration(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let datatable_name = str_arg(args, "datatable_name")?;
    let name = str_arg(args, "name")?;
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "Invalid `name` '{name}': only letters, digits, '_' and '-' are allowed"
        ));
    }
    let role = role_arg(args)?;
    let version = version_arg(args)?;

    let mut definition = Map::new();
    definition.insert("name".into(), name.into());
    definition.insert(
        "code_up".into(),
        with_migration_role(str_arg(args, "code_up")?, role).into(),
    );
    if let Some(code_down) = opt_str_arg(args, "code_down") {
        // A rollback has to run as the role that created what it drops.
        definition.insert(
            "code_down".into(),
            with_migration_role(code_down, role).into(),
        );
    }
    let version = match save_migration(ctx, datatable_name, version, definition).await {
        Ok(Ok(version)) => version,
        Ok(Err(message)) => return Ok(message),
        Err(e) => return Ok(format!("Error saving migration: {}", api_message(&e))),
    };
    let label = format!("{version} ({name})");
    if args.get("apply") == Some(&Value::Bool(false)) {
        return Ok(format!(
            "Saved migration {label} as pending. It is not applied yet."
        ));
    }

    // Applying it alone would run it ahead of migrations it may depend on.
    let earlier = match earlier_pending(ctx, datatable_name, version).await {
        Ok(earlier) => earlier,
        Err(e) => return Ok(format!("Saved migration {label} as pending, but could not check what else is pending, so it was not applied: {e}")),
    };
    if !earlier.is_empty() {
        return Ok(format!("Saved migration {label} as pending without applying it: earlier migrations are not applied yet ({}) and it may depend on them. Tell the user, then apply them all in order with run_datatable_migrations.", earlier.join(", ")));
    }

    let run = format!(
        "{}?only={version}",
        migrations_path(ctx, "run_datatable_migrations", datatable_name)
    );
    Ok(match post_migrations(ctx, &run).await {
        Ok(_) => format!("Applied migration {label} on datatable \"{datatable_name}\"."),
        Err(e) => format!("Migration {label} is saved but failed to apply: {e}. It stays pending and blocks later migrations. If nothing of it was applied, fix it by calling this tool again with version {version}; otherwise, or to drop it, tell the user, who can edit or delete it in Workspace settings → Data Tables."),
    })
}

async fn run_datatable_migrations(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let datatable_name = str_arg(args, "datatable_name")?;
    let path = migrations_path(ctx, "run_datatable_migrations", datatable_name);
    Ok(match post_migrations(ctx, &path).await {
        Ok(result) => {
            let applied = array(result.get("applied"));
            if applied.is_empty() {
                format!("No pending migration to apply on datatable \"{datatable_name}\".")
            } else {
                format!(
                    "Applied {} migration(s) on datatable \"{datatable_name}\": {}.",
                    applied.len(),
                    labels(result.get("applied"))
                )
            }
        }
        Err(e) => format!("Error applying migrations: {e}. Migrations before the failing one may have been applied: list them to see."),
    })
}

async fn rollback_datatable_migration(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let datatable_name = str_arg(args, "datatable_name")?;
    let version = version_arg(args)?.ok_or("Missing required number argument `version`")?;
    let path = format!(
        "{}?only={version}",
        migrations_path(ctx, "rollback_datatable_migrations", datatable_name)
    );
    Ok(match post_migrations(ctx, &path).await {
        Ok(result) if array(result.get("rolled_back")).is_empty() => format!("Migration {version} is not applied on datatable \"{datatable_name}\", so there is nothing to roll back."),
        Ok(result) => format!(
            "Rolled back migration {} on datatable \"{datatable_name}\". It is pending again.",
            labels(result.get("rolled_back"))
        ),
        Err(e) => format!("Error rolling back: {e}"),
    })
}

const POSTGRESQL_SCHEMA_QUERY: &str = "
SELECT table_name, column_name, udt_name, column_default, is_nullable, nsp.nspname AS table_schema FROM information_schema.columns
RIGHT JOIN pg_namespace nsp ON table_schema = nsp.nspname WHERE nsp.nspname NOT IN ('information_schema', 'pg_toast', 'pg_catalog')
AND NOT starts_with(nsp.nspname, 'pg_') AND has_schema_privilege(nsp.oid, 'USAGE')";
const MYSQL_SCHEMA_QUERY: &str = "SELECT DATABASE() AS default_db_name, TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT FROM information_schema.columns WHERE table_schema = DATABASE() OR table_schema NOT IN ('information_schema', 'mysql', 'performance_schema', 'sys', '_vt');";
const SNOWFLAKE_SCHEMA_QUERY: &str = "select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT, IS_NULLABLE from information_schema.columns where table_schema != 'INFORMATION_SCHEMA'";
const MSSQL_SCHEMA_QUERY: &str = "select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT from information_schema.columns where table_schema != 'sys'";
const BIGQUERY_SCHEMA_SCRIPT: &str = r#"import { BigQuery } from '@google-cloud/bigquery@7.5.0';
export async function main(args: bigquery) {
const bq = new BigQuery({
	credentials: args
})
const [datasets] = await bq.getDatasets();
if (!datasets) return {}
const schema = {} as any
let queries = datasets.map(dataset => `
	SELECT
		table_name,
		'${dataset.id}' as dataset,
		ARRAY_AGG(STRUCT(
			if(is_nullable = 'YES', true, false) AS required,
			column_name AS name,
			data_type AS type,
			if(column_default = 'NULL', null, column_default) AS \`default\`
		) ORDER BY ordinal_position)
		AS schema
	FROM \`${dataset.id}\`.INFORMATION_SCHEMA.COLUMNS
	GROUP BY table_name`
)
let query = queries.join('\nUNION ALL \n')
const [rows] = await bq.query(query)
for (const row of rows) {
	schema[row.dataset] ??= {}
	schema[row.dataset][row.table_name] = {}
	for (const col of row.schema) {
		const colName = col.name
		delete col.name
		if (col.default === null) {
			delete col.default
		}
		schema[row.dataset][row.table_name][colName] = col
	}
}
return schema
}"#;

/// How long a schema introspection is awaited.
const DB_SCHEMA_WAIT_SECS: u64 = 120;

async fn get_db_schema(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = opt_str_arg(args, "resourcePath")
        .filter(|p| !p.is_empty())
        .ok_or("Database path not provided")?;
    let resource = ctx
        .get(&ctx.ws(&format!("resources/get/{}", path_enc(path))))
        .await?;
    let resource_type = resource["resource_type"].as_str().unwrap_or_default();
    let not_found = || "Database not found".to_string();
    let (language, content, arg_name) = match resource_type {
        "postgresql" => ("postgresql", POSTGRESQL_SCHEMA_QUERY, "database"),
        "mysql" => ("mysql", MYSQL_SCHEMA_QUERY, "database"),
        "ms_sql_server" | "mssql" => ("mssql", MSSQL_SCHEMA_QUERY, "database"),
        "snowflake" | "snowflake_oauth" => ("snowflake", SNOWFLAKE_SCHEMA_QUERY, "database"),
        "bigquery" => ("bun", BIGQUERY_SCHEMA_SCRIPT, "args"),
        _ => return Err(not_found()),
    };
    let id = run_script_preview(
        ctx,
        None,
        language,
        content,
        json!({ arg_name: format!("$res:{path}") }),
    )
    .await
    .map_err(|_| not_found())?;
    let job = wait_job(ctx, &id, DB_SCHEMA_WAIT_SECS, false)
        .await
        .ok_or_else(|| {
            format!(
                "The schema query did not finish within {DB_SCHEMA_WAIT_SECS} seconds (job {id})"
            )
        })?;
    if job["success"] != true {
        return Err(not_found());
    }
    let mut stringified =
        stringify_db_schema(resource_type, &job["result"]).ok_or_else(not_found)?;
    if stringified.chars().count() > MAX_SCHEMA_LENGTH {
        stringified = stringified
            .chars()
            .take(MAX_SCHEMA_LENGTH)
            .collect::<String>()
            + "...";
    }
    Ok(format!(
        "DATABASE SCHEMA (each column is in the format [name, type, required, default?]):\n{stringified}"
    ))
}

/// The object under `key`, created empty where a JavaScript object would first get it.
fn slot(map: &mut Map<String, Value>, key: String) -> &mut Map<String, Value> {
    map.entry(key)
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("only objects are inserted")
}

/// The introspection result as `{schema: {table: [[name, type, required, default?]]}}`,
/// narrowed to the default schema's tables when the database has one.
fn stringify_db_schema(resource_type: &str, result: &Value) -> Option<String> {
    let mut schemas: Map<String, Value> = Map::new();
    let column = |name: String, ty: &Value, required: Value, default: Option<&Value>| {
        let mut col = vec![Value::String(name), ty.clone(), required];
        if truthy(default) {
            col.extend(default.cloned());
        }
        Value::Array(col)
    };
    if resource_type == "bigquery" {
        for (dataset, tables) in result.as_object()? {
            let dataset = slot(&mut schemas, dataset.clone());
            for (table, columns) in tables.as_object()? {
                let table = slot(dataset, table.clone());
                for (name, col) in columns.as_object()? {
                    let entry = column(
                        name.clone(),
                        &col["type"],
                        col["required"].clone(),
                        col.get("default"),
                    );
                    table.insert(name.clone(), entry);
                }
            }
        }
    } else {
        for row in result.as_array()? {
            let (schema, table, name, ty, required, default) = match resource_type {
                "postgresql" => (
                    "table_schema",
                    "table_name",
                    "column_name",
                    "udt_name",
                    row["is_nullable"] == "NO",
                    row.get("column_default"),
                ),
                // `required` reads IS_NULLABLE the way the session does: "YES" is true.
                "snowflake" | "snowflake_oauth" => (
                    "TABLE_SCHEMA",
                    "TABLE_NAME",
                    "COLUMN_NAME",
                    "DATA_TYPE",
                    row["IS_NULLABLE"] == "YES",
                    row.get("COLUMN_DEFAULT"),
                ),
                // These queries select neither nullability nor a default the session reads.
                _ => (
                    "TABLE_SCHEMA",
                    "TABLE_NAME",
                    "COLUMN_NAME",
                    "DATA_TYPE",
                    false,
                    None,
                ),
            };
            let schema = slot(&mut schemas, js_str(&row[schema]));
            // A schema with no table comes back as one row of nulls.
            if resource_type == "postgresql" && !truthy(row.get(table)) && !truthy(row.get(name)) {
                continue;
            }
            let name = js_str(&row[name]);
            let entry = column(name.clone(), &row[ty], json!(required), default);
            slot(schema, js_str(&row[table])).insert(name, entry);
        }
    }

    for table in schemas.values_mut().filter_map(Value::as_object_mut) {
        for columns in table.values_mut() {
            if let Some(by_name) = columns.as_object_mut() {
                *columns = Value::Array(
                    std::mem::take(by_name)
                        .into_iter()
                        .map(|(_, c)| c)
                        .collect(),
                );
            }
        }
    }
    let default_schema = ["public", "PUBLIC", "dbo"]
        .into_iter()
        .find(|k| schemas.contains_key(*k))
        .or_else(|| {
            (resource_type == "mysql" && schemas.len() == 1)
                .then(|| schemas.keys().next().map(String::as_str))
                .flatten()
        })
        .map(str::to_string);
    Some(match default_schema {
        Some(key) => schemas[&key].to_string(),
        None => Value::Object(schemas).to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_schema_is_compact_and_narrowed_to_public() {
        let rows = json!([
            { "table_schema": "empty", "table_name": null, "column_name": null },
            { "table_schema": "public", "table_name": "t", "column_name": "id", "udt_name": "int4", "is_nullable": "NO", "column_default": "nextval('s')" },
            { "table_schema": "public", "table_name": "t", "column_name": "name", "udt_name": "text", "is_nullable": "YES", "column_default": null },
            { "table_schema": "other", "table_name": "u", "column_name": "x", "udt_name": "bool", "is_nullable": "YES", "column_default": "" },
        ]);
        assert_eq!(
            stringify_db_schema("postgresql", &rows).unwrap(),
            r#"{"t":[["id","int4",true,"nextval('s')"],["name","text",false]]}"#
        );
        let rows = json!([rows[0], rows[3]]);
        assert_eq!(
            stringify_db_schema("postgresql", &rows).unwrap(),
            r#"{"empty":{},"other":{"u":[["x","bool",false]]}}"#
        );
        assert_eq!(stringify_db_schema("postgresql", &json!({})), None);
    }

    #[test]
    fn migration_role_replaces_a_leading_annotation_only() {
        assert_eq!(with_migration_role("BEGIN;\nEND;", None), "BEGIN;\nEND;");
        assert_eq!(
            with_migration_role(
                "\n-- note\n--ROLE: old;\n-- roles kept\nBEGIN;\n-- role below\nEND;",
                Some("app_rw")
            ),
            "-- role app_rw\n-- note\n-- roles kept\nBEGIN;\n-- role below\nEND;"
        );
    }

    #[test]
    fn sql_completion_caps_rows_and_rewrites_the_missing_datatable_error() {
        let rows: Vec<Value> = (0..101).map(|i| json!({ "i": i })).collect();
        let out: Value = serde_json::from_str(&format_datatable_sql_completion(
            &json!({ "success": true, "result": rows }),
            "main",
        ))
        .unwrap();
        assert_eq!(out["rowCount"], 101);
        assert_eq!(out["result"].as_array().unwrap().len(), 100);
        assert_eq!(out["note"], "Showing first 100 of 101 rows");

        assert_eq!(
            format_datatable_sql_completion(&json!({ "success": true, "result": [] }), "main"),
            "{\n  \"success\": true,\n  \"rowCount\": 0,\n  \"result\": []\n}"
        );
        assert_eq!(
            format_datatable_sql_completion(
                &json!({ "success": false, "result": { "error": { "message": "boom" } } }),
                "main"
            ),
            r#"{"success":false,"error":"boom"}"#
        );
        let missing = format_datatable_sql_completion(
            &json!({ "success": false, "result": { "error": { "message": "datatable main not found" } } }),
            "main",
        );
        assert!(missing.contains("Datatable \\\"main\\\" is not configured"));
    }
}
