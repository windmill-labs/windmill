use serde::{Deserialize, Deserializer, Serialize};
use windmill_types::scripts::ScriptLang;

fn deserialize_bool_from_null<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<bool>::deserialize(deserializer).map(|v| v.unwrap_or(false))
}

fn deserialize_string_from_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(|v| v.unwrap_or_default())
}

fn deserialize_column_identity_from_null<'de, D>(
    deserializer: D,
) -> Result<ColumnIdentity, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<ColumnIdentity>::deserialize(deserializer).map(|v| v.unwrap_or_default())
}

const WM_INTERNAL_PREFIX: &str = "-- WM_INTERNAL_DB_";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DbType {
    Postgresql,
    Mysql,
    MsSqlServer,
    Snowflake,
    Bigquery,
    Duckdb,
}

impl DbType {
    pub fn from_script_lang(lang: &ScriptLang) -> Option<Self> {
        match lang {
            ScriptLang::Postgresql => Some(DbType::Postgresql),
            ScriptLang::Mysql => Some(DbType::Mysql),
            ScriptLang::Mssql => Some(DbType::MsSqlServer),
            ScriptLang::Snowflake => Some(DbType::Snowflake),
            ScriptLang::Bigquery => Some(DbType::Bigquery),
            ScriptLang::DuckDb => Some(DbType::Duckdb),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnIdentity {
    #[serde(rename = "By Default")]
    ByDefault,
    #[serde(rename = "Always")]
    Always,
    #[serde(rename = "No")]
    No,
}

impl Default for ColumnIdentity {
    fn default() -> Self {
        ColumnIdentity::No
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub field: String,
    pub datatype: String,
    #[serde(default, deserialize_with = "deserialize_string_from_null")]
    pub defaultvalue: String,
    #[serde(default, deserialize_with = "deserialize_bool_from_null")]
    pub isprimarykey: bool,
    #[serde(default, deserialize_with = "deserialize_column_identity_from_null")]
    pub isidentity: ColumnIdentity,
    #[serde(default, deserialize_with = "deserialize_string_from_null")]
    pub isnullable: String,
    #[serde(default, deserialize_with = "deserialize_bool_from_null")]
    pub isenum: bool,
    #[serde(default)]
    pub ignored: Option<bool>,
    #[serde(default, rename = "hideInsert")]
    pub hide_insert: Option<bool>,
    #[serde(default, rename = "overrideDefaultValue")]
    pub override_default_value: Option<bool>,
    #[serde(default, rename = "defaultUserValue")]
    pub default_user_value: Option<serde_json::Value>,
    #[serde(default, rename = "defaultValueNull")]
    pub default_value_null: Option<bool>,
}

impl ColumnDef {
    pub fn simple(field: &str, datatype: &str) -> Self {
        ColumnDef {
            field: field.to_string(),
            datatype: datatype.to_string(),
            defaultvalue: String::new(),
            isprimarykey: false,
            isidentity: ColumnIdentity::No,
            isnullable: "YES".to_string(),
            isenum: false,
            ignored: None,
            hide_insert: None,
            override_default_value: None,
            default_user_value: None,
            default_value_null: None,
        }
    }
}

/// A simpler column reference used in buildParameters and some query contexts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleColumn {
    pub field: String,
    pub datatype: String,
}

pub struct SelectOptions {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ---------------------------------------------------------------------------
// WM_INTERNAL_DB expansion: marker detection and SQL generation
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SelectPayload {
    table: String,
    #[serde(rename = "columnDefs")]
    column_defs: Vec<ColumnDef>,
    #[serde(rename = "whereClause")]
    where_clause: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    #[serde(rename = "fixPgIntTypes")]
    fix_pg_int_types: Option<bool>,
    ducklake: Option<String>,
}

#[derive(Deserialize)]
struct CountPayload {
    table: String,
    #[serde(rename = "columnDefs")]
    column_defs: Vec<ColumnDef>,
    #[serde(rename = "whereClause")]
    where_clause: Option<String>,
    ducklake: Option<String>,
}

#[derive(Deserialize)]
struct DeletePayload {
    table: String,
    columns: Vec<ColumnDef>,
    ducklake: Option<String>,
}

#[derive(Deserialize)]
struct InsertPayload {
    table: String,
    columns: Vec<ColumnDef>,
    ducklake: Option<String>,
}

#[derive(Deserialize)]
struct UpdatePayload {
    table: String,
    column: SimpleColumn,
    columns: Vec<SimpleColumn>,
    ducklake: Option<String>,
}

fn wrap_ducklake_query(query: &str, ducklake: &str) -> String {
    let attach = format!(
        "ATTACH 'ducklake://{}' AS dl;USE dl;\n",
        escape_sql_literal(ducklake)
    );
    // Insert after all leading comment lines (matching JS: /^(--.*\n)*/)
    let mut insert_pos = 0;
    for line in query.lines() {
        if line.starts_with("--") {
            insert_pos += line.len() + 1; // +1 for \n
        } else {
            break;
        }
    }
    // Clamp to string length in case there's no trailing newline
    insert_pos = insert_pos.min(query.len());
    format!("{}{}{}", &query[..insert_pos], attach, &query[insert_pos..])
}

/// Result of expanding a WM_INTERNAL_DB marker.
#[derive(Debug, PartialEq)]
pub struct ExpandedQuery {
    pub code: String,
    /// If set, the worker should use this language instead of the original.
    /// Used when the expanded code is in a different language (e.g. BigQuery all-tables uses Bun).
    pub language_override: Option<ScriptLang>,
}

/// Checks if a SQL script is a WM_INTERNAL_DB marker and expands it into real SQL.
/// Returns `None` if the script is not a marker (regular SQL passthrough).
/// Returns `Some(Ok(expanded))` on successful expansion.
/// Returns `Some(Err(msg))` if the marker is detected but malformed.
pub fn try_expand_internal_db_query(
    code: &str,
    lang: &ScriptLang,
) -> Option<Result<ExpandedQuery, String>> {
    let first_line = code.lines().next()?;
    if !first_line.starts_with(WM_INTERNAL_PREFIX) {
        return None;
    }

    let db_type = match DbType::from_script_lang(lang) {
        Some(dt) => dt,
        None => {
            return Some(Err(format!(
                "WM_INTERNAL_DB markers are not supported for language {:?}",
                lang
            )));
        }
    };

    // Extract operation and JSON: "-- WM_INTERNAL_DB_SELECT {...}"
    let after_prefix = &first_line[WM_INTERNAL_PREFIX.len()..];
    let (op, json_str) = match after_prefix.find(' ') {
        Some(pos) => (&after_prefix[..pos], after_prefix[pos + 1..].trim()),
        None => {
            return Some(Err("WM_INTERNAL_DB marker missing JSON payload".to_string()));
        }
    };

    let result = match op {
        // Data operations
        "SELECT" => expand_select(json_str, db_type).map(ExpandedQuery::sql),
        "COUNT" => expand_count(json_str, db_type).map(ExpandedQuery::sql),
        "DELETE" => expand_delete(json_str, db_type).map(ExpandedQuery::sql),
        "INSERT" => expand_insert(json_str, db_type).map(ExpandedQuery::sql),
        "UPDATE" => expand_update(json_str, db_type).map(ExpandedQuery::sql),
        // Schema DDL operations
        "DROP_TABLE" => expand_drop_table(json_str, db_type).map(ExpandedQuery::sql),
        "CREATE_TABLE" => expand_create_table(json_str, db_type).map(ExpandedQuery::sql),
        "ALTER_TABLE" => expand_alter_table(json_str, db_type).map(ExpandedQuery::sql),
        "CREATE_SCHEMA" => expand_create_schema(json_str, db_type).map(ExpandedQuery::sql),
        "DROP_SCHEMA" => expand_drop_schema(json_str, db_type).map(ExpandedQuery::sql),
        // Metadata queries
        "LOAD_TABLE_METADATA" => expand_load_table_metadata(json_str, db_type),
        "FOREIGN_KEYS" => expand_foreign_keys(json_str, db_type).map(ExpandedQuery::sql),
        "PRIMARY_KEY_CONSTRAINT" => {
            expand_primary_key_constraint(json_str, db_type).map(ExpandedQuery::sql)
        }
        "SNOWFLAKE_PRIMARY_KEYS" => expand_snowflake_primary_keys(json_str).map(ExpandedQuery::sql),
        _ => Err(format!("Unknown WM_INTERNAL_DB operation: {}", op)),
    };

    Some(result)
}

impl ExpandedQuery {
    fn sql(code: String) -> Self {
        Self { code, language_override: None }
    }

    fn with_language(code: String, lang: ScriptLang) -> Self {
        Self { code, language_override: Some(lang) }
    }
}

fn expand_select(json_str: &str, db_type: DbType) -> Result<String, String> {
    let payload: SelectPayload =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid SELECT payload: {}", e))?;

    let options = SelectOptions { limit: payload.limit, offset: payload.offset };
    let breaking = payload
        .fix_pg_int_types
        .map(|v| BreakingFeatures { fix_pg_int_types: v });

    let query = make_select_query(
        &payload.table,
        &payload.column_defs,
        payload.where_clause.as_deref(),
        db_type,
        Some(&options),
        breaking.as_ref(),
    )?;

    Ok(maybe_wrap_ducklake(query, payload.ducklake.as_deref()))
}

fn expand_count(json_str: &str, db_type: DbType) -> Result<String, String> {
    let payload: CountPayload =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid COUNT payload: {}", e))?;

    let query = make_count_query(
        db_type,
        &payload.table,
        payload.where_clause.as_deref(),
        &payload.column_defs,
    )?;

    Ok(maybe_wrap_ducklake(query, payload.ducklake.as_deref()))
}

fn expand_delete(json_str: &str, db_type: DbType) -> Result<String, String> {
    let payload: DeletePayload =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid DELETE payload: {}", e))?;

    let query = make_delete_query(&payload.table, &payload.columns, db_type);
    Ok(maybe_wrap_ducklake(query, payload.ducklake.as_deref()))
}

fn expand_insert(json_str: &str, db_type: DbType) -> Result<String, String> {
    let payload: InsertPayload =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid INSERT payload: {}", e))?;

    let query = make_insert_query(&payload.table, &payload.columns, db_type)?;
    Ok(maybe_wrap_ducklake(query, payload.ducklake.as_deref()))
}

fn expand_update(json_str: &str, db_type: DbType) -> Result<String, String> {
    let payload: UpdatePayload =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid UPDATE payload: {}", e))?;

    let query = make_update_query(&payload.table, &payload.column, &payload.columns, db_type);
    Ok(maybe_wrap_ducklake(query, payload.ducklake.as_deref()))
}

fn maybe_wrap_ducklake(query: String, ducklake: Option<&str>) -> String {
    match ducklake {
        Some(dl) => wrap_ducklake_query(&query, dl),
        None => query,
    }
}

pub struct BreakingFeatures {
    pub fix_pg_int_types: bool,
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

pub fn build_parameters(columns: &[SimpleColumn], db_type: DbType) -> String {
    columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let base_type = col.datatype.split('(').next().unwrap_or(&col.datatype);
            match db_type {
                DbType::Postgresql => format!("-- ${} {}", i + 1, col.field),
                DbType::Mysql => format!("-- :{} ({})", col.field, base_type),
                DbType::MsSqlServer => {
                    format!("-- @p{} {} ({})", i + 1, col.field, base_type)
                }
                DbType::Snowflake => format!("-- ? {} ({})", col.field, base_type),
                DbType::Bigquery => format!("-- @{} ({})", col.field, base_type),
                DbType::Duckdb => format!("-- ${} ({})", col.field, base_type),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn cols_to_simple(cols: &[ColumnDef]) -> Vec<SimpleColumn> {
    cols.iter()
        .map(|c| SimpleColumn { field: c.field.clone(), datatype: c.datatype.clone() })
        .collect()
}

pub fn render_db_quoted_identifier(identifier: &str, db_type: DbType) -> String {
    match db_type {
        DbType::Postgresql | DbType::Snowflake | DbType::Duckdb => {
            format!("\"{}\"", identifier.replace('"', "\"\""))
        }
        DbType::MsSqlServer => format!("[{}]", identifier.replace(']', "]]")),
        DbType::Mysql | DbType::Bigquery => format!("`{}`", identifier.replace('`', "``")),
    }
}

/// Escape a value for safe interpolation inside a SQL single-quoted string literal.
fn escape_sql_literal(s: &str) -> String {
    s.replace('\'', "''")
}

/// Quote a potentially schema-qualified table name (e.g. "public.users" → "\"public\".\"users\"").
/// Each dot-separated part is individually quoted using `render_db_quoted_identifier`.
fn quote_table_name(table: &str, db_type: DbType) -> String {
    table
        .split('.')
        .map(|part| render_db_quoted_identifier(part, db_type))
        .collect::<Vec<_>>()
        .join(".")
}

/// Shorthand alias for `render_db_quoted_identifier`.
fn qi(identifier: &str, db_type: DbType) -> String {
    render_db_quoted_identifier(identifier, db_type)
}

pub fn build_visible_field_list(column_defs: &[ColumnDef], db_type: DbType) -> Vec<String> {
    column_defs
        .iter()
        .filter(|c| c.ignored != Some(true))
        .map(|c| render_db_quoted_identifier(&c.field, db_type))
        .collect()
}

// ---------------------------------------------------------------------------
// SELECT
// ---------------------------------------------------------------------------

fn make_snowflake_select_query(
    table: &str,
    column_defs: &[ColumnDef],
    where_clause: Option<&str>,
    options: Option<&SelectOptions>,
) -> String {
    let limit = options
        .and_then(|o| o.limit)
        .filter(|&l| l != 0)
        .unwrap_or(100);
    let offset = options.and_then(|o| o.offset).unwrap_or(0);

    let mut headers: Vec<SimpleColumn> =
        vec![SimpleColumn { field: "quicksearch".into(), datatype: "text".into() }];

    let filtered_columns = build_visible_field_list(column_defs, DbType::Snowflake);
    let select_clause = filtered_columns.join(", ");

    let mut query = String::from("\n");
    query.push_str(&format!(
        "SELECT {} FROM {}",
        select_clause,
        quote_table_name(table, DbType::Snowflake)
    ));

    // quicksearch condition
    let mut qs_parts: Vec<String> = vec!["LENGTH(?) = 0".to_string()];
    for col in &filtered_columns {
        headers.push(SimpleColumn { field: "quicksearch".into(), datatype: "text".into() });
        qs_parts.push(format!("CONCAT({}) ILIKE CONCAT('%', ?, '%')", col));
    }
    let quicksearch_condition = qs_parts.join(" OR ");

    if let Some(wc) = where_clause {
        query.push_str(&format!(" WHERE {} AND ({})", wc, quicksearch_condition));
    } else {
        query.push_str(&format!(" WHERE {}", quicksearch_condition));
    }

    // ORDER BY
    let order_parts: Vec<String> = column_defs
        .iter()
        .map(|col| {
            headers.push(SimpleColumn {
                field: "order_by".into(),
                datatype: "text".into(),
            });
            headers.push(SimpleColumn {
                field: "is_desc".into(),
                datatype: "boolean".into(),
            });
            headers.push(SimpleColumn {
                field: "order_by".into(),
                datatype: "text".into(),
            });
            headers.push(SimpleColumn {
                field: "is_desc".into(),
                datatype: "boolean".into(),
            });
            {
                let escaped = escape_sql_literal(&col.field);
                let quoted = qi(&col.field, DbType::Snowflake);
                format!(
                    "CASE WHEN ? = '{}' AND ? = FALSE THEN {} END ASC,\n\t\tCASE WHEN ? = '{}' AND ? = TRUE THEN {} END DESC",
                    escaped, quoted, escaped, quoted
                )
            }
        })
        .collect();

    query.push_str(&format!(" ORDER BY {}", order_parts.join(",\n")));
    query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // Prepend parameters
    let params = build_parameters(&headers, DbType::Snowflake);
    format!("{}\n{}", params, query)
}

fn pg_build_order_by(
    field: &str,
    is_desc: bool,
    text_cast: bool,
    check_is_number: Option<bool>,
) -> String {
    let quoted = qi(field, DbType::Postgresql);
    let escaped = escape_sql_literal(field);
    let number_check_expr = match check_is_number {
        Some(true) => format!(
            " pg_typeof({})::text IN ('integer', 'bigint', 'smallint', 'numeric', 'real', 'double precision') AND",
            quoted
        ),
        Some(false) => format!(
            " pg_typeof({})::text NOT IN ('integer', 'bigint', 'smallint', 'numeric', 'real', 'double precision') AND",
            quoted
        ),
        None => String::new(),
    };
    let cast = if text_cast { "::text" } else { "" };
    let desc_suffix = if is_desc { " DESC" } else { "" };
    format!(
        "(CASE WHEN{} $4 = '{}' AND $5 IS {} THEN {}{} END){}",
        number_check_expr, escaped, is_desc, quoted, cast, desc_suffix
    )
}

pub fn make_select_query(
    table: &str,
    column_defs: &[ColumnDef],
    where_clause: Option<&str>,
    db_type: DbType,
    options: Option<&SelectOptions>,
    breaking_features: Option<&BreakingFeatures>,
) -> Result<String, String> {
    if table.is_empty() {
        return Err("Table name is required".to_string());
    }

    let params = vec![
        SimpleColumn {
            field: "limit".into(),
            datatype: if db_type == DbType::Bigquery {
                "integer"
            } else {
                "int"
            }
            .into(),
        },
        SimpleColumn {
            field: "offset".into(),
            datatype: if db_type == DbType::Bigquery {
                "integer"
            } else {
                "int"
            }
            .into(),
        },
        SimpleColumn {
            field: "quicksearch".into(),
            datatype: if db_type == DbType::Bigquery {
                "string"
            } else {
                "text"
            }
            .into(),
        },
        SimpleColumn {
            field: "order_by".into(),
            datatype: if db_type == DbType::Bigquery {
                "string"
            } else {
                "text"
            }
            .into(),
        },
        SimpleColumn {
            field: "is_desc".into(),
            datatype: if db_type == DbType::Bigquery {
                "bool"
            } else {
                "boolean"
            }
            .into(),
        },
    ];

    let filtered_columns = build_visible_field_list(column_defs, db_type);
    let select_clause = filtered_columns.join(", ");

    match db_type {
        DbType::Snowflake => {
            return Ok(make_snowflake_select_query(
                table,
                column_defs,
                where_clause,
                options,
            ));
        }
        DbType::Mysql => {
            let mut query = build_parameters(&params, db_type);
            query.push('\n');

            let order_by: String = column_defs
                .iter()
                .map(|col| {
                    let escaped = escape_sql_literal(&col.field);
                    let quoted = qi(&col.field, db_type);
                    format!(
                        "\nCASE WHEN :order_by = '{}' AND :is_desc IS false THEN {} END,\nCASE WHEN :order_by = '{}' AND :is_desc IS true THEN {} END DESC",
                        escaped, quoted, escaped, quoted
                    )
                })
                .collect::<Vec<_>>()
                .join(",\n");

            let quicksearch = format!(
                " (:quicksearch = '' OR CONCAT_WS(' ', {}) LIKE CONCAT('%', :quicksearch, '%'))",
                filtered_columns.join(", ")
            );

            query.push_str(&format!(
                "SELECT {} FROM {}",
                select_clause,
                quote_table_name(table, db_type)
            ));
            query.push_str(&format!(
                " WHERE {} {}",
                where_clause
                    .map(|wc| format!("{} AND", wc))
                    .unwrap_or_default(),
                quicksearch
            ));
            query.push_str(&format!(" ORDER BY {}", order_by));
            query.push_str(" LIMIT :limit OFFSET :offset");
            Ok(query)
        }
        DbType::Postgresql => {
            let mut query = build_parameters(&params, db_type);
            query.push('\n');

            let fix_pg = breaking_features
                .map(|bf| bf.fix_pg_int_types)
                .unwrap_or(false);

            let order_by_parts: Vec<String> = column_defs
                .iter()
                .map(|col| {
                    if fix_pg {
                        let force_text_cast =
                            col.datatype.contains("[]") || col.datatype.to_lowercase() == "json";
                        if force_text_cast {
                            format!(
                                "\n      {},\n      {}",
                                pg_build_order_by(&col.field, false, true, None),
                                pg_build_order_by(&col.field, true, true, None)
                            )
                        } else {
                            format!(
                                "\n      {},\n      {},\n      {},\n      {}",
                                pg_build_order_by(&col.field, false, true, Some(false)),
                                pg_build_order_by(&col.field, false, false, Some(true)),
                                pg_build_order_by(&col.field, true, true, Some(false)),
                                pg_build_order_by(&col.field, true, false, Some(true))
                            )
                        }
                    } else {
                        format!(
                            "\n      {},\n      {}",
                            pg_build_order_by(&col.field, false, true, None),
                            pg_build_order_by(&col.field, true, true, None)
                        )
                    }
                })
                .collect();

            let order_by = format!("\n      {}", order_by_parts.join(",\n"));

            let quicksearch = format!(
                "($3 = '' OR CONCAT({}) ILIKE '%' || $3 || '%')",
                filtered_columns.join(", ")
            );

            query.push_str(&format!(
                "SELECT {} FROM {}\n",
                filtered_columns
                    .iter()
                    .map(|c| format!("{}::text", c))
                    .collect::<Vec<_>>()
                    .join(", "),
                quote_table_name(table, db_type)
            ));
            query.push_str(&format!(
                " WHERE {} {}\n",
                where_clause
                    .map(|wc| format!("{} AND", wc))
                    .unwrap_or_default(),
                quicksearch
            ));
            query.push_str(&format!(" ORDER BY {}\n", order_by));
            query.push_str(" LIMIT $1::INT OFFSET $2::INT");
            Ok(query)
        }
        DbType::MsSqlServer => {
            let mut query = build_parameters(&params, db_type);
            query.push('\n');

            let unsortable_types = ["text", "ntext", "image"];

            let order_by: String = column_defs
                .iter()
                .filter(|col| !unsortable_types.contains(&col.datatype.to_lowercase().as_str()))
                .map(|col| {
                    let escaped = escape_sql_literal(&col.field);
                    let quoted = qi(&col.field, db_type);
                    format!(
                        "\n(CASE WHEN @p4 = '{}' AND @p5 = 0 THEN {} END) ASC,\n(CASE WHEN @p4 = '{}' AND @p5 = 1 THEN {} END) DESC",
                        escaped, quoted, escaped, quoted
                    )
                })
                .collect::<Vec<_>>()
                .join(",\n");

            // Filter search columns to exclude unsortable types
            let search_clause: String = filtered_columns
                .iter()
                .filter(|col| {
                    let field_name = &col[1..col.len() - 1]; // Remove brackets
                    let def = column_defs.iter().find(|c| c.field == field_name);
                    !def.map_or(false, |d| {
                        unsortable_types.contains(&d.datatype.to_lowercase().as_str())
                    })
                })
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");

            let quicksearch = if !search_clause.is_empty() {
                format!(
                    " (@p3 = '' OR CONCAT({}) LIKE '%' + @p3 + '%')",
                    search_clause
                )
            } else {
                " (@p3 = '')".to_string()
            };

            query.push_str(&format!(
                "SELECT {} FROM {}",
                select_clause,
                quote_table_name(table, db_type)
            ));
            query.push_str(&format!(
                " WHERE {} {}",
                where_clause
                    .map(|wc| format!("{} AND", wc))
                    .unwrap_or_default(),
                quicksearch
            ));
            query.push_str(&format!(" ORDER BY {}", order_by));
            query.push_str(" OFFSET @p2 ROWS FETCH NEXT @p1 ROWS ONLY");
            Ok(query)
        }
        DbType::Bigquery => {
            let mut query = build_parameters(&params, db_type);
            query.push('\n');

            let order_by: String = column_defs
                .iter()
                .map(|col| {
                    let escaped = escape_sql_literal(&col.field);
                    let quoted = qi(&col.field, db_type);
                    let is_complex = col.datatype == "JSON"
                        || col.datatype.starts_with("STRUCT")
                        || col.datatype.starts_with("ARRAY")
                        || col.datatype == "GEOGRAPHY";
                    if is_complex {
                        format!(
                            "\n(CASE WHEN @order_by = '{}' AND @is_desc = false THEN TO_JSON_STRING({}) END) ASC,\n(CASE WHEN @order_by = '{}' AND @is_desc = true THEN TO_JSON_STRING({}) END) DESC",
                            escaped, quoted, escaped, quoted
                        )
                    } else {
                        format!(
                            "\n(CASE WHEN @order_by = '{}' AND @is_desc = false THEN {} END) ASC,\n(CASE WHEN @order_by = '{}' AND @is_desc = true THEN {} END) DESC",
                            escaped, quoted, escaped, quoted
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join(",\n");

            // Build search clause with JSON conversion for complex types
            let search_parts: Vec<String> = filtered_columns
                .iter()
                .map(|col| {
                    let field_name = &col[1..col.len() - 1]; // Remove backticks
                    let def = column_defs.iter().find(|c| c.field == field_name);
                    let is_complex = def.map_or(false, |d| {
                        d.datatype == "JSON"
                            || d.datatype.starts_with("STRUCT")
                            || d.datatype.starts_with("ARRAY")
                            || d.datatype == "GEOGRAPHY"
                    });
                    if is_complex {
                        format!("TO_JSON_STRING({})", col)
                    } else {
                        format!("CAST({} AS STRING)", col)
                    }
                })
                .collect();

            let quicksearch = format!(
                " (@quicksearch = '' OR REGEXP_CONTAINS(CONCAT({}), '(?i)' || @quicksearch))",
                search_parts.join(",")
            );

            query.push_str(&format!(
                "SELECT {} FROM {}",
                select_clause,
                quote_table_name(table, db_type)
            ));
            query.push_str(&format!(
                " WHERE {} {}",
                where_clause
                    .map(|wc| format!("{} AND", wc))
                    .unwrap_or_default(),
                quicksearch
            ));
            query.push_str(&format!(" ORDER BY {}", order_by));
            query.push_str(" LIMIT @limit OFFSET @offset");
            Ok(query)
        }
        DbType::Duckdb => {
            let mut query = build_parameters(&params, db_type);
            query.push('\n');

            let order_by_parts: Vec<String> = column_defs
                .iter()
                .map(|col| {
                    let escaped = escape_sql_literal(&col.field);
                    let quoted = qi(&col.field, db_type);
                    format!(
                        "\n      (CASE WHEN $order_by = '{}' AND $is_desc IS false THEN {}::text END),\n      (CASE WHEN $order_by = '{}' AND $is_desc IS true THEN {}::text END) DESC",
                        escaped, quoted, escaped, quoted
                    )
                })
                .collect();

            let order_by = format!("\n      {}", order_by_parts.join(",\n"));

            let quicksearch = format!(
                "($quicksearch = '' OR CONCAT({}) ILIKE '%' || $quicksearch || '%')",
                filtered_columns.join(", ")
            );

            query.push_str(&format!(
                "SELECT {} FROM {}\n",
                filtered_columns.join(", "),
                quote_table_name(table, db_type)
            ));
            query.push_str(&format!(
                " WHERE {} {}\n",
                where_clause
                    .map(|wc| format!("{} AND", wc))
                    .unwrap_or_default(),
                quicksearch
            ));
            query.push_str(&format!(" ORDER BY {}\n", order_by));
            query.push_str(" LIMIT $limit::INT OFFSET $offset::INT");
            Ok(query)
        }
    }
}

// ---------------------------------------------------------------------------
// COUNT
// ---------------------------------------------------------------------------

pub fn make_count_query(
    db_type: DbType,
    table: &str,
    where_clause: Option<&str>,
    column_defs: &[ColumnDef],
) -> Result<String, String> {
    let where_prefix = " WHERE ";
    let and_condition = " AND ";
    let mut quicksearch_condition = String::new();

    let qs_datatype = if db_type == DbType::Bigquery {
        "string"
    } else {
        "text"
    };

    let mut query = match db_type {
        DbType::Snowflake => String::new(), // Snowflake rebuilds parameters below
        _ => build_parameters(
            &[SimpleColumn { field: "quicksearch".into(), datatype: qs_datatype.into() }],
            db_type,
        ),
    };

    if db_type != DbType::Snowflake {
        query.push('\n');
    }

    if let Some(wc) = where_clause {
        quicksearch_condition = format!(" {} AND ", wc);
    }

    let filtered_columns = build_visible_field_list(column_defs, db_type);

    match db_type {
        DbType::Mysql => {
            if !filtered_columns.is_empty() {
                quicksearch_condition.push_str(&format!(
                    " (:quicksearch = '' OR CONCAT_WS(' ', {}) LIKE CONCAT('%', :quicksearch, '%'))",
                    filtered_columns.join(", ")
                ));
            } else {
                quicksearch_condition.push_str(" (:quicksearch = '' OR 1 = 1)");
            }
            query.push_str(&format!(
                "SELECT COUNT(*) as count FROM {}",
                quote_table_name(table, db_type)
            ));
        }
        DbType::Postgresql => {
            if !filtered_columns.is_empty() {
                quicksearch_condition.push_str(&format!(
                    "($1 = '' OR CONCAT({}) ILIKE '%' || $1 || '%')",
                    filtered_columns.join(", ")
                ));
            } else {
                quicksearch_condition.push_str("($1 = '' OR 1 = 1)");
            }
            query.push_str(&format!(
                "SELECT COUNT(*) as count FROM {}",
                quote_table_name(table, db_type)
            ));
        }
        DbType::MsSqlServer => {
            if !filtered_columns.is_empty() {
                quicksearch_condition.push_str(&format!(
                    "(@p1 = '' OR CONCAT({}) LIKE '%' + @p1 + '%')",
                    filtered_columns.join(", +")
                ));
            } else {
                quicksearch_condition.push_str("(@p1 = '' OR 1 = 1)");
            }
            query.push_str(&format!(
                "SELECT COUNT(*) as count FROM {}",
                quote_table_name(table, db_type)
            ));
        }
        DbType::Snowflake => {
            if !filtered_columns.is_empty() {
                query = build_parameters(
                    &[
                        SimpleColumn { field: "quicksearch".into(), datatype: "text".into() },
                        SimpleColumn { field: "quicksearch".into(), datatype: "text".into() },
                    ],
                    DbType::Snowflake,
                );
                query.push('\n');
                quicksearch_condition.push_str(&format!(
                    "(? = '' OR CONCAT({}) ILIKE '%' || ? || '%')",
                    filtered_columns.join(", ")
                ));
            } else {
                query = build_parameters(
                    &[SimpleColumn { field: "quicksearch".into(), datatype: "text".into() }],
                    DbType::Snowflake,
                );
                query.push('\n');
                quicksearch_condition.push_str("(? = '' OR 1 = 1)");
            }
            query.push_str(&format!(
                "SELECT COUNT(*) as count FROM {}",
                quote_table_name(table, db_type)
            ));
        }
        DbType::Bigquery => {
            if !filtered_columns.is_empty() {
                let search_clause: String = filtered_columns
                    .iter()
                    .map(|col| {
                        let field_name = &col[1..col.len() - 1]; // Remove backticks
                        let def = column_defs.iter().find(|c| c.field == field_name);
                        let is_complex = def.map_or(false, |d| {
                            d.datatype == "JSON"
                                || d.datatype.starts_with("STRUCT")
                                || d.datatype.starts_with("ARRAY")
                        });
                        if is_complex {
                            format!("TO_JSON_STRING({})", col)
                        } else {
                            col.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                quicksearch_condition.push_str(&format!(
                    "(@quicksearch = '' OR REGEXP_CONTAINS(CONCAT({}), '(?i)' || @quicksearch))",
                    search_clause
                ));
            } else {
                quicksearch_condition.push_str("(@quicksearch = '' OR 1 = 1)");
            }
            query.push_str(&format!(
                "SELECT COUNT(*) as count FROM {}",
                quote_table_name(table, db_type)
            ));
        }
        DbType::Duckdb => {
            if !filtered_columns.is_empty() {
                quicksearch_condition.push_str(&format!(
                    " ($quicksearch = '' OR CONCAT(' ', {}) LIKE CONCAT('%', $quicksearch, '%'))",
                    filtered_columns.join(", ")
                ));
            } else {
                quicksearch_condition.push_str(" ($quicksearch = '' OR 1 = 1)");
            }
            query.push_str(&format!(
                "SELECT COUNT(*) as count FROM {}",
                quote_table_name(table, db_type)
            ));
        }
    }

    if where_clause.is_some() {
        query.push_str(where_prefix);
        query.push_str(&quicksearch_condition);
    } else {
        if db_type == DbType::MsSqlServer {
            query.push_str(where_prefix);
        } else {
            query.push_str(and_condition);
        }
        query.push_str(&quicksearch_condition);
    }

    // Replace AND with WHERE for non-mssql when no whereClause
    if where_clause.is_none()
        && matches!(
            db_type,
            DbType::Mysql
                | DbType::Postgresql
                | DbType::Snowflake
                | DbType::Bigquery
                | DbType::Duckdb
        )
    {
        query = query.replacen(and_condition, where_prefix, 1);
    }

    Ok(query)
}

// ---------------------------------------------------------------------------
// DELETE
// ---------------------------------------------------------------------------

pub fn make_delete_query(table: &str, columns: &[ColumnDef], db_type: DbType) -> String {
    let simple_cols = cols_to_simple(columns);
    let params = match db_type {
        DbType::Snowflake => {
            // Snowflake duplicates each column for IS NULL check + value
            let doubled: Vec<SimpleColumn> = columns
                .iter()
                .flat_map(|c| {
                    vec![
                        SimpleColumn { field: c.field.clone(), datatype: c.datatype.clone() },
                        SimpleColumn { field: c.field.clone(), datatype: c.datatype.clone() },
                    ]
                })
                .collect();
            build_parameters(&doubled, db_type)
        }
        _ => build_parameters(&simple_cols, db_type),
    };

    let mut query = params;

    let qt = quote_table_name(table, db_type);

    match db_type {
        DbType::Postgresql => {
            let conditions: String = columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let qf = qi(&c.field, db_type);
                    format!(
                        "(${}::text::{} IS NULL AND {} IS NULL OR {} = ${}::text::{})",
                        i + 1,
                        c.datatype,
                        qf,
                        qf,
                        i + 1,
                        c.datatype
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nDELETE FROM {} \nWHERE {} RETURNING 1;",
                qt, conditions
            ));
        }
        DbType::Mysql => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    let qf = qi(&c.field, db_type);
                    format!(
                        "(:{} IS NULL AND {} IS NULL OR {} = :{})",
                        c.field, qf, qf, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!("\nDELETE FROM {} \nWHERE {}", qt, conditions));
        }
        DbType::MsSqlServer => {
            let conditions: String = columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let qf = qi(&c.field, db_type);
                    format!(
                        "(@p{} IS NULL AND {} IS NULL OR {} = @p{})",
                        i + 1,
                        qf,
                        qf,
                        i + 1
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!("\nDELETE FROM {} \nWHERE {}", qt, conditions));
        }
        DbType::Snowflake => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    let qf = qi(&c.field, db_type);
                    format!("(? = 'null' AND {} IS NULL OR {} = ?)", qf, qf)
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!("\nDELETE FROM {} \nWHERE {}", qt, conditions));
        }
        DbType::Bigquery => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    let qf = qi(&c.field, db_type);
                    format!(
                        "(CAST(@{} AS STRING) = 'null' AND {} IS NULL OR {} = @{})",
                        c.field, qf, qf, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!("\nDELETE FROM {} \nWHERE {}", qt, conditions));
        }
        DbType::Duckdb => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    let qf = qi(&c.field, db_type);
                    format!(
                        "(${} IS NULL AND {} IS NULL OR {} = ${})",
                        c.field, qf, qf, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!("\nDELETE FROM {} \nWHERE {}", qt, conditions));
        }
    }

    query
}

// ---------------------------------------------------------------------------
// INSERT
// ---------------------------------------------------------------------------

fn is_truthy(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map_or(false, |f| f != 0.0),
        serde_json::Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn get_user_default_value(column: &ColumnDef) -> Option<String> {
    if column.default_value_null == Some(true) {
        return Some("NULL".to_string());
    }
    if let Some(ref val) = column.default_user_value {
        if is_truthy(val) {
            return match val {
                serde_json::Value::String(s) => Some(format!("'{}'", s)),
                other => Some(other.to_string()),
            };
        }
    }
    None
}

fn format_insert_values(columns: &[ColumnDef], db_type: DbType, start_index: usize) -> String {
    columns
        .iter()
        .enumerate()
        .map(|(i, c)| match db_type {
            DbType::Mysql => format!(":{}", c.field),
            DbType::Postgresql => format!("${}::{}", start_index + i, c.datatype),
            DbType::MsSqlServer => format!("@p{}", start_index + i),
            DbType::Snowflake => "?".to_string(),
            DbType::Bigquery => format!("@{}", c.field),
            DbType::Duckdb => format!("${}", c.field),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_column_names(columns: &[ColumnDef], db_type: DbType) -> String {
    columns
        .iter()
        .map(|c| qi(&c.field, db_type))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_default_values(columns: &[ColumnDef]) -> String {
    columns
        .iter()
        .map(|c| {
            let user_default = get_user_default_value(c);
            if c.override_default_value == Some(true) {
                user_default.unwrap_or_default()
            } else {
                user_default.unwrap_or_else(|| c.defaultvalue.clone())
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn should_omit_column_in_insert(column: &ColumnDef) -> Result<bool, String> {
    // In JS: if (!column.hideInsert || column.isidentity === ColumnIdentity.Always) return true
    if column.hide_insert != Some(true) || column.isidentity == ColumnIdentity::Always {
        return Ok(true);
    }

    let has_user_default_value = {
        let has_val = column.default_user_value.as_ref().map_or(
            false,
            |v| !matches!(v, serde_json::Value::String(s) if s.is_empty()),
        );
        has_val || column.default_value_null == Some(true)
    };

    let has_db_default = !column.defaultvalue.is_empty();

    if column.isnullable == "NO" {
        if !has_user_default_value && !has_db_default && column.isidentity == ColumnIdentity::No {
            return Err(format!(
                "Column {} is not nullable and has no default value",
                column.field
            ));
        }
        if !has_user_default_value && !has_db_default {
            return Ok(column.isidentity != ColumnIdentity::No);
        }
        return Ok(!has_user_default_value && has_db_default);
    } else if column.isnullable == "YES" {
        return Ok(!has_user_default_value);
    }

    Ok(false)
}

pub fn make_insert_query(
    table: &str,
    columns: &[ColumnDef],
    db_type: DbType,
) -> Result<String, String> {
    if table.is_empty() {
        return Err("Table name is required".to_string());
    }

    let columns_insert: Vec<&ColumnDef> = columns
        .iter()
        .filter(|x| {
            x.hide_insert != Some(true)
                && !(db_type == DbType::Postgresql && x.defaultvalue.starts_with("nextval("))
        })
        .collect();

    let columns_default: Vec<&ColumnDef> = columns
        .iter()
        .filter(|c| match should_omit_column_in_insert(c) {
            Ok(omit) => !omit,
            Err(_) => false, // Error case handled below
        })
        .collect();

    // Check for errors in shouldOmitColumnInInsert
    for c in columns {
        should_omit_column_in_insert(c)?;
    }

    let all_insert_columns: Vec<&ColumnDef> = columns_insert
        .iter()
        .chain(columns_default.iter())
        .copied()
        .collect();

    let insert_cols_owned: Vec<ColumnDef> = columns_insert.iter().map(|c| (*c).clone()).collect();
    let simple_insert_cols = cols_to_simple(&insert_cols_owned);
    let mut query = build_parameters(&simple_insert_cols, db_type);
    query.push('\n');

    let should_insert_comma = !columns_default.is_empty();
    let column_names = format_column_names(
        &all_insert_columns
            .iter()
            .map(|c| (*c).clone())
            .collect::<Vec<_>>(),
        db_type,
    );
    let insert_values = format_insert_values(&insert_cols_owned, db_type, 1);
    let default_cols_owned: Vec<ColumnDef> = columns_default.iter().map(|c| (*c).clone()).collect();
    let default_values = format_default_values(&default_cols_owned);
    let comma_or_empty = if should_insert_comma { ", " } else { "" };
    let values_str = format!("{}{}{}", insert_values, comma_or_empty, default_values);

    let qt = quote_table_name(table, db_type);
    if values_str.trim().is_empty() {
        return Ok(format!("INSERT INTO {} DEFAULT VALUES", qt));
    }

    query.push_str(&format!(
        "INSERT INTO {} ({}) VALUES ({})",
        qt, column_names, values_str
    ));
    Ok(query)
}

// ---------------------------------------------------------------------------
// UPDATE
// ---------------------------------------------------------------------------

pub fn make_update_query(
    table: &str,
    column: &SimpleColumn,
    columns: &[SimpleColumn],
    db_type: DbType,
) -> String {
    let mut param_list: Vec<SimpleColumn> =
        vec![SimpleColumn { field: "value_to_update".into(), datatype: column.datatype.clone() }];

    match db_type {
        DbType::Snowflake => {
            for c in columns {
                param_list
                    .push(SimpleColumn { field: c.field.clone(), datatype: c.datatype.clone() });
                param_list
                    .push(SimpleColumn { field: c.field.clone(), datatype: c.datatype.clone() });
            }
        }
        _ => {
            for c in columns {
                param_list
                    .push(SimpleColumn { field: c.field.clone(), datatype: c.datatype.clone() });
            }
        }
    }

    let mut query = build_parameters(&param_list, db_type);
    query.push('\n');

    let qt = quote_table_name(table, db_type);
    let qcol = qi(&column.field, db_type);

    match db_type {
        DbType::Postgresql => {
            let conditions: String = columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let qf = qi(&c.field, db_type);
                    format!(
                        "(${}::text::{} IS NULL AND {} IS NULL OR {} = ${}::text::{})",
                        i + 2,
                        c.datatype,
                        qf,
                        qf,
                        i + 2,
                        c.datatype
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");

            query.push_str(&format!(
                "\nUPDATE {} SET {} = $1::text::{} \nWHERE {}\tRETURNING 1",
                qt, qcol, column.datatype, conditions
            ));
        }
        DbType::Mysql => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    let qf = qi(&c.field, db_type);
                    format!(
                        "(:{} IS NULL AND {} IS NULL OR {} = :{})",
                        c.field, qf, qf, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nUPDATE {} SET {} = :value_to_update \nWHERE {}",
                qt, qcol, conditions
            ));
        }
        DbType::MsSqlServer => {
            let conditions: String = columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let qf = qi(&c.field, db_type);
                    format!(
                        "(@p{} IS NULL AND {} IS NULL OR {} = @p{})",
                        i + 2,
                        qf,
                        qf,
                        i + 2
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nUPDATE {} SET {} = @p1 \nWHERE {}",
                qt, qcol, conditions
            ));
        }
        DbType::Snowflake => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    let qf = qi(&c.field, db_type);
                    format!("(? = 'null' AND {} IS NULL OR {} = ?)", qf, qf)
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nUPDATE {} SET {} = ? \nWHERE {}",
                qt, qcol, conditions
            ));
        }
        DbType::Bigquery => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    let qf = qi(&c.field, db_type);
                    format!(
                        "(CAST(@{} AS STRING) = 'null' AND {} IS NULL OR {} = @{})",
                        c.field, qf, qf, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nUPDATE {} SET {} = @value_to_update \nWHERE {}",
                qt, qcol, conditions
            ));
        }
        DbType::Duckdb => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    let qf = qi(&c.field, db_type);
                    format!(
                        "(${} IS NULL AND {} IS NULL OR {} = ${})",
                        c.field, qf, qf, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nUPDATE {} SET {} = $value_to_update \nWHERE {}",
                qt, qcol, conditions
            ));
        }
    }

    query
}

// ---------------------------------------------------------------------------
// ===========================================================================
// Schema DDL operations
// ===========================================================================

// --- Payload structs ---

#[derive(Deserialize)]
struct DropTablePayload {
    table: String,
    schema: Option<String>,
    ducklake: Option<String>,
}

#[derive(Deserialize)]
struct CreateSchemaPayload {
    schema: String,
    ducklake: Option<String>,
}

#[derive(Deserialize)]
struct DropSchemaPayload {
    schema: String,
    ducklake: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TableEditorColumn {
    name: String,
    datatype: String,
    #[serde(rename = "primaryKey")]
    primary_key: Option<bool>,
    #[serde(rename = "defaultValue")]
    default_value: Option<String>,
    nullable: Option<bool>,
    datatype_length: Option<i64>,
    #[serde(rename = "initialName")]
    initial_name: Option<String>,
    default_constraint_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ForeignKeyColumn {
    #[serde(rename = "sourceColumn")]
    source_column: Option<String>,
    #[serde(rename = "targetColumn")]
    target_column: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TableEditorForeignKey {
    #[serde(rename = "targetTable")]
    target_table: Option<String>,
    columns: Vec<ForeignKeyColumn>,
    #[serde(rename = "onDelete", default = "default_no_action")]
    on_delete: String,
    #[serde(rename = "onUpdate", default = "default_no_action")]
    on_update: String,
    fk_constraint_name: Option<String>,
}

fn default_no_action() -> String {
    "NO ACTION".to_string()
}

#[derive(Deserialize)]
struct CreateTablePayload {
    name: String,
    columns: Vec<TableEditorColumn>,
    #[serde(rename = "foreignKeys", default)]
    foreign_keys: Vec<TableEditorForeignKey>,
    schema: Option<String>,
    ducklake: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum AlterTableOperation {
    #[serde(rename = "addColumn")]
    AddColumn { column: TableEditorColumn },
    #[serde(rename = "dropColumn")]
    DropColumn { name: String },
    #[serde(rename = "alterColumn")]
    AlterColumn {
        original: TableEditorColumn,
        #[serde(rename = "defaultConstraintName")]
        default_constraint_name: Option<String>,
        changes: serde_json::Value,
    },
    #[serde(rename = "addForeignKey")]
    AddForeignKey {
        #[serde(rename = "foreignKey")]
        foreign_key: TableEditorForeignKey,
    },
    #[serde(rename = "dropForeignKey")]
    DropForeignKey { fk_constraint_name: String },
    #[serde(rename = "renameTable")]
    RenameTable { to: String },
    #[serde(rename = "addPrimaryKey")]
    AddPrimaryKey { columns: Vec<String> },
    #[serde(rename = "dropPrimaryKey")]
    DropPrimaryKey { pk_constraint_name: Option<String> },
}

#[derive(Deserialize)]
struct AlterTablePayload {
    name: String,
    operations: Vec<AlterTableOperation>,
    schema: Option<String>,
    ducklake: Option<String>,
}

// --- Metadata payload structs ---

#[derive(Deserialize)]
struct LoadTableMetadataPayload {
    table: Option<String>,
    /// MySQL requires the database name for metadata queries.
    #[serde(rename = "databaseName")]
    database_name: Option<String>,
    ducklake: Option<String>,
}

#[derive(Deserialize)]
struct ForeignKeysPayload {
    table: String,
    schema: Option<String>,
    ducklake: Option<String>,
}

#[derive(Deserialize)]
struct PrimaryKeyConstraintPayload {
    table: String,
    schema: Option<String>,
    ducklake: Option<String>,
}

// --- Helper functions ---

fn db_supports_schemas(db_type: DbType) -> bool {
    matches!(
        db_type,
        DbType::Postgresql | DbType::Snowflake | DbType::Bigquery
    )
}

fn db_supports_transactional_ddl(db_type: DbType) -> bool {
    matches!(db_type, DbType::Postgresql | DbType::MsSqlServer)
}

fn datatype_has_length(datatype: &str) -> bool {
    let dt = datatype.to_lowercase();
    matches!(
        dt.as_str(),
        "varchar"
            | "char"
            | "nvarchar"
            | "nchar"
            | "varbinary"
            | "binary"
            | "bit"
            | "character varying"
            | "character"
    )
}

fn table_ref(table: &str, schema: Option<&str>, db_type: DbType) -> String {
    if db_supports_schemas(db_type) {
        if let Some(s) = schema.filter(|s| !s.is_empty()) {
            return format!("{}.{}", qi(s.trim(), db_type), qi(table, db_type));
        }
    }
    qi(table, db_type)
}

fn format_default_value(s: &str, datatype: &str, db_type: DbType) -> String {
    if s.is_empty() {
        return String::new();
    }
    if s.starts_with('{') && s.ends_with('}') {
        return s[1..s.len() - 1].to_string();
    }
    let escaped = escape_sql_literal(s);
    if db_type == DbType::Postgresql {
        return format!("CAST('{}' AS {})", escaped, datatype);
    }
    format!("'{}'", escaped)
}

fn render_column_ddl(c: &TableEditorColumn, db_type: DbType, pk_modifier: bool) -> String {
    let datatype = match c.datatype_length {
        Some(l) if datatype_has_length(&c.datatype) => format!("{}({})", c.datatype, l),
        _ => c.datatype.clone(),
    };
    let def_val = c
        .default_value
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| format_default_value(s, &datatype, db_type));

    let mut s = format!("{} {}", qi(&c.name, db_type), datatype);
    if c.nullable != Some(true) {
        s.push_str(" NOT NULL");
    }
    if let Some(ref d) = def_val {
        if !d.is_empty() {
            s.push_str(&format!(" DEFAULT {}", d));
        }
    }
    if pk_modifier {
        s.push_str(" PRIMARY KEY");
    }
    s
}

fn render_fk_ddl(
    fk: &TableEditorForeignKey,
    use_schema: bool,
    db_type: DbType,
    table_name: &str,
) -> String {
    let source_cols_raw: Vec<&str> = fk
        .columns
        .iter()
        .filter_map(|c| c.source_column.as_deref())
        .collect();
    let target_cols_raw: Vec<&str> = fk
        .columns
        .iter()
        .filter_map(|c| c.target_column.as_deref())
        .collect();
    let target_table_raw = match &fk.target_table {
        Some(t) => {
            if use_schema || !t.contains('.') {
                t.clone()
            } else {
                t.split('.').last().unwrap_or(t).to_string()
            }
        }
        None => String::new(),
    };

    let mut sql = String::from("CONSTRAINT ");
    let name_parts: Vec<&str> = std::iter::once(table_name)
        .chain(source_cols_raw.iter().map(|c| &c[..c.len().min(10)]))
        .chain(std::iter::once(target_table_raw.as_str()))
        .chain(target_cols_raw.iter().map(|c| &c[..c.len().min(10)]))
        .collect();
    let full_name = format!("fk_{} ", name_parts.join("_").replace('.', "_"));
    let truncated = &full_name[..full_name.len().min(60)];
    sql.push_str(truncated);

    let source_quoted: Vec<String> = source_cols_raw.iter().map(|c| qi(c, db_type)).collect();
    let target_quoted: Vec<String> = target_cols_raw.iter().map(|c| qi(c, db_type)).collect();

    sql.push_str(&format!(
        " FOREIGN KEY ({}) REFERENCES {} ({})",
        source_quoted.join(", "),
        quote_table_name(&target_table_raw, db_type),
        target_quoted.join(", ")
    ));
    if fk.on_delete != "NO ACTION" {
        sql.push_str(&format!(" ON DELETE {}", fk.on_delete));
    }
    if fk.on_update != "NO ACTION" {
        sql.push_str(&format!(" ON UPDATE {}", fk.on_update));
    }
    sql
}

// --- Schema DDL expand functions ---

fn expand_drop_table(json_str: &str, db_type: DbType) -> Result<String, String> {
    let p: DropTablePayload =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid DROP_TABLE payload: {}", e))?;
    let tref = match p.schema.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => format!("{}.{}", qi(s, db_type), qi(&p.table, db_type)),
        None => qi(&p.table, db_type),
    };
    let query = format!("DROP TABLE {};", tref);
    Ok(maybe_wrap_ducklake(query, p.ducklake.as_deref()))
}

fn expand_create_schema(json_str: &str, db_type: DbType) -> Result<String, String> {
    let p: CreateSchemaPayload = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid CREATE_SCHEMA payload: {}", e))?;
    let query = format!("CREATE SCHEMA {};", qi(&p.schema, db_type));
    Ok(maybe_wrap_ducklake(query, p.ducklake.as_deref()))
}

fn expand_drop_schema(json_str: &str, db_type: DbType) -> Result<String, String> {
    let p: DropSchemaPayload = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid DROP_SCHEMA payload: {}", e))?;
    let query = format!("DROP SCHEMA {} CASCADE;", qi(&p.schema, db_type));
    Ok(maybe_wrap_ducklake(query, p.ducklake.as_deref()))
}

fn expand_create_table(json_str: &str, db_type: DbType) -> Result<String, String> {
    let p: CreateTablePayload = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid CREATE_TABLE payload: {}", e))?;
    let query = make_create_table_query(&p, db_type);
    Ok(maybe_wrap_ducklake(query, p.ducklake.as_deref()))
}

fn make_create_table_query(p: &CreateTablePayload, db_type: DbType) -> String {
    let pk_count = p
        .columns
        .iter()
        .filter(|c| c.primary_key == Some(true))
        .count();
    let use_schema = db_supports_schemas(db_type);
    let schema = if db_type == DbType::Snowflake {
        Some(p.schema.as_deref().unwrap_or("PUBLIC"))
    } else {
        p.schema.as_deref()
    };

    let mut lines: Vec<String> = p
        .columns
        .iter()
        .map(|c| {
            let pk_inline = pk_count == 1 && c.primary_key == Some(true);
            let mut line = format!("  {}", render_column_ddl(c, db_type, false));
            if pk_inline {
                if db_type == DbType::Bigquery {
                    line.push_str(" PRIMARY KEY NOT ENFORCED");
                } else {
                    line.push_str(" PRIMARY KEY");
                }
            }
            line
        })
        .collect();

    for fk in &p.foreign_keys {
        lines.push(format!("  {}", render_fk_inline(fk, use_schema, db_type)));
    }

    if pk_count > 1 {
        let pk_cols: Vec<String> = p
            .columns
            .iter()
            .filter(|c| c.primary_key == Some(true))
            .map(|c| qi(&c.name, db_type))
            .collect();
        let mut pk = format!("  PRIMARY KEY ({})", pk_cols.join(", "));
        if db_type == DbType::Bigquery {
            pk.push_str(" NOT ENFORCED");
        }
        lines.push(pk);
    }

    let tref = if use_schema {
        match schema.filter(|s| !s.is_empty()) {
            Some(s) => format!("{}.{}", qi(s.trim(), db_type), qi(p.name.trim(), db_type)),
            None => qi(p.name.trim(), db_type),
        }
    } else {
        qi(p.name.trim(), db_type)
    };

    format!("CREATE TABLE {} (\n{}\n);", tref, lines.join(",\n"))
}

/// Render FK for CREATE TABLE (inline, no CONSTRAINT name prefix).
fn render_fk_inline(fk: &TableEditorForeignKey, use_schema: bool, db_type: DbType) -> String {
    let source_cols: Vec<String> = fk
        .columns
        .iter()
        .filter_map(|c| c.source_column.as_deref().map(|s| qi(s, db_type)))
        .collect();
    let target_cols: Vec<String> = fk
        .columns
        .iter()
        .filter_map(|c| c.target_column.as_deref().map(|s| qi(s, db_type)))
        .collect();
    let target_table = match &fk.target_table {
        Some(t) => {
            let raw = if use_schema || !t.contains('.') {
                t.clone()
            } else {
                t.split('.').last().unwrap_or(t).to_string()
            };
            quote_table_name(&raw, db_type)
        }
        None => String::new(),
    };
    let mut sql = format!(
        "FOREIGN KEY ({}) REFERENCES {} ({})",
        source_cols.join(", "),
        target_table,
        target_cols.join(", ")
    );
    if fk.on_delete != "NO ACTION" {
        sql.push_str(&format!(" ON DELETE {}", fk.on_delete));
    }
    if fk.on_update != "NO ACTION" {
        sql.push_str(&format!(" ON UPDATE {}", fk.on_update));
    }
    sql
}

// --- ALTER TABLE ---

fn expand_alter_table(json_str: &str, db_type: DbType) -> Result<String, String> {
    let p: AlterTablePayload = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid ALTER_TABLE payload: {}", e))?;
    let queries = make_alter_table_queries(&p, db_type)?;
    if queries.is_empty() {
        return Ok(String::new());
    }
    let joined = queries.join("\n");
    let query = if db_supports_transactional_ddl(db_type) {
        if db_type == DbType::MsSqlServer {
            format!("BEGIN TRANSACTION;\n{}\nCOMMIT TRANSACTION;", joined)
        } else {
            format!("BEGIN;\n{}\nCOMMIT;", joined)
        }
    } else {
        joined
    };
    Ok(maybe_wrap_ducklake(query, p.ducklake.as_deref()))
}

fn make_alter_table_queries(p: &AlterTablePayload, db_type: DbType) -> Result<Vec<String>, String> {
    let use_schema = db_supports_schemas(db_type);
    let tref = if use_schema {
        match p.schema.as_deref().filter(|s| !s.is_empty()) {
            Some(s) => format!("{}.{}", qi(s.trim(), db_type), qi(&p.name, db_type)),
            None => qi(&p.name, db_type),
        }
    } else {
        qi(&p.name, db_type)
    };

    let mut queries = Vec::new();

    for op in &p.operations {
        match op {
            AlterTableOperation::AddColumn { column } => {
                queries.push(format!(
                    "ALTER TABLE {} ADD COLUMN {};",
                    tref,
                    render_column_ddl(column, db_type, false)
                ));
            }
            AlterTableOperation::DropColumn { name } => {
                queries.push(format!(
                    "ALTER TABLE {} DROP COLUMN {};",
                    tref,
                    render_db_quoted_identifier(name, db_type)
                ));
            }
            AlterTableOperation::AlterColumn { original, default_constraint_name, changes } => {
                queries.extend(render_alter_column(
                    &tref,
                    original,
                    default_constraint_name.as_deref(),
                    changes,
                    db_type,
                )?);
            }
            AlterTableOperation::AddForeignKey { foreign_key } => {
                queries.push(format!(
                    "ALTER TABLE {} ADD {};",
                    tref,
                    render_fk_ddl(foreign_key, use_schema, db_type, &p.name)
                ));
            }
            AlterTableOperation::DropForeignKey { fk_constraint_name } => {
                if db_type == DbType::Mysql {
                    queries.push(format!(
                        "ALTER TABLE {} DROP FOREIGN KEY {};",
                        tref,
                        render_db_quoted_identifier(fk_constraint_name, db_type)
                    ));
                } else {
                    queries.push(format!(
                        "ALTER TABLE {} DROP CONSTRAINT {};",
                        tref,
                        render_db_quoted_identifier(fk_constraint_name, db_type)
                    ));
                }
            }
            AlterTableOperation::RenameTable { to } => {
                let new_ref = if db_type == DbType::Snowflake {
                    match p.schema.as_deref().filter(|s| !s.is_empty()) {
                        Some(s) => format!("{}.{}", qi(s.trim(), db_type), qi(to, db_type)),
                        None => qi(to, db_type),
                    }
                } else {
                    qi(to, db_type)
                };
                queries.push(format!("ALTER TABLE {} RENAME TO {};", tref, new_ref));
            }
            AlterTableOperation::AddPrimaryKey { columns } => {
                let quoted_cols: Vec<String> = columns.iter().map(|c| qi(c, db_type)).collect();
                if db_type == DbType::Snowflake {
                    let cname = format!("{}_pk", p.name);
                    queries.push(format!(
                        "ALTER TABLE {} ADD CONSTRAINT {} PRIMARY KEY ({});",
                        tref,
                        qi(&cname, db_type),
                        quoted_cols.join(", ")
                    ));
                } else {
                    let not_enforced = if db_type == DbType::Bigquery {
                        " NOT ENFORCED"
                    } else {
                        ""
                    };
                    queries.push(format!(
                        "ALTER TABLE {} ADD PRIMARY KEY ({}){};",
                        tref,
                        quoted_cols.join(", "),
                        not_enforced
                    ));
                }
            }
            AlterTableOperation::DropPrimaryKey { pk_constraint_name } => {
                if db_type == DbType::Mysql || pk_constraint_name.is_none() {
                    queries.push(format!("ALTER TABLE {} DROP PRIMARY KEY;", tref));
                } else {
                    queries.push(format!(
                        "ALTER TABLE {} DROP CONSTRAINT {};",
                        tref,
                        render_db_quoted_identifier(
                            pk_constraint_name.as_deref().unwrap_or(""),
                            db_type
                        )
                    ));
                }
            }
        }
    }

    Ok(queries)
}

fn render_alter_column(
    table_ref: &str,
    original: &TableEditorColumn,
    default_constraint_name: Option<&str>,
    changes: &serde_json::Value,
    db_type: DbType,
) -> Result<Vec<String>, String> {
    let mut queries = Vec::new();

    let base_datatype = changes
        .get("datatype")
        .and_then(|v| v.as_str())
        .unwrap_or(&original.datatype);
    let dt_length = if datatype_has_length(base_datatype) {
        changes
            .get("datatype_length")
            .and_then(|v| v.as_i64())
            .or(original.datatype_length)
    } else {
        None
    };
    let datatype = match dt_length {
        Some(l) => format!("{}({})", base_datatype, l),
        None => base_datatype.to_string(),
    };

    // Alter datatype
    if changes.get("datatype").is_some() || changes.get("datatype_length").is_some() {
        queries.push(render_alter_datatype(
            table_ref,
            &original.name,
            &datatype,
            db_type,
        ));
    }

    // Default value changes
    if let Some(def_val) = changes.get("defaultValue") {
        if def_val.is_null() && original.default_value.is_some() {
            queries.push(render_drop_default_value(
                table_ref,
                &original.name,
                &datatype,
                db_type,
                default_constraint_name,
            ));
        } else if let Some(s) = def_val.as_str() {
            let formatted = format_default_value(s, &original.datatype, db_type);
            queries.push(render_add_default_value(
                table_ref,
                &original.name,
                &formatted,
                db_type,
            ));
        }
    }

    // Nullable changes
    if let Some(nullable) = changes.get("nullable").and_then(|v| v.as_bool()) {
        queries.push(render_alter_nullable(
            table_ref,
            &original.name,
            nullable,
            &datatype,
            db_type,
        ));
    }

    // Rename
    if let Some(new_name) = changes.get("name").and_then(|v| v.as_str()) {
        queries.push(render_rename_column(
            table_ref,
            &original.name,
            new_name,
            db_type,
        ));
    }

    Ok(queries)
}

fn render_alter_datatype(table_ref: &str, col: &str, datatype: &str, db_type: DbType) -> String {
    let qc = qi(col, db_type);
    match db_type {
        DbType::Postgresql | DbType::Duckdb => {
            format!(
                "ALTER TABLE {} ALTER COLUMN {} TYPE {};",
                table_ref, qc, datatype
            )
        }
        DbType::MsSqlServer => {
            format!(
                "ALTER TABLE {} ALTER COLUMN {} {};",
                table_ref, qc, datatype
            )
        }
        DbType::Mysql => {
            format!(
                "ALTER TABLE {} MODIFY COLUMN {} {};",
                table_ref, qc, datatype
            )
        }
        DbType::Snowflake | DbType::Bigquery => {
            format!(
                "ALTER TABLE {} ALTER COLUMN {} SET DATA TYPE {};",
                table_ref, qc, datatype
            )
        }
    }
}

fn render_drop_default_value(
    table_ref: &str,
    col: &str,
    _datatype: &str,
    db_type: DbType,
    constraint_name: Option<&str>,
) -> String {
    let qc = qi(col, db_type);
    match db_type {
        DbType::MsSqlServer => format!(
            "ALTER TABLE {} DROP CONSTRAINT {};",
            table_ref,
            render_db_quoted_identifier(constraint_name.unwrap_or(""), db_type)
        ),
        _ => format!(
            "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;",
            table_ref, qc
        ),
    }
}

fn render_add_default_value(
    table_ref: &str,
    col: &str,
    default_value: &str,
    db_type: DbType,
) -> String {
    let qc = qi(col, db_type);
    match db_type {
        DbType::MsSqlServer => format!(
            "ALTER TABLE {} ADD CONSTRAINT DF_{}_{} DEFAULT {} FOR {};",
            table_ref,
            table_ref
                .replace('.', "_")
                .replace('"', "")
                .replace('[', "")
                .replace(']', ""),
            col,
            default_value,
            qc
        ),
        _ => format!(
            "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};",
            table_ref, qc, default_value
        ),
    }
}

fn render_alter_nullable(
    table_ref: &str,
    col: &str,
    nullable: bool,
    datatype: &str,
    db_type: DbType,
) -> String {
    let qc = qi(col, db_type);
    match db_type {
        DbType::Postgresql | DbType::Duckdb | DbType::Snowflake | DbType::Bigquery => {
            let action = if nullable { "DROP" } else { "SET" };
            format!(
                "ALTER TABLE {} ALTER COLUMN {} {} NOT NULL;",
                table_ref, qc, action
            )
        }
        DbType::MsSqlServer => {
            let null_str = if nullable { "NULL" } else { "NOT NULL" };
            format!(
                "ALTER TABLE {} ALTER COLUMN {} {} {};",
                table_ref, qc, datatype, null_str
            )
        }
        DbType::Mysql => {
            let null_str = if nullable { "NULL" } else { "NOT NULL" };
            format!(
                "ALTER TABLE {} MODIFY COLUMN {} {} {};",
                table_ref, qc, datatype, null_str
            )
        }
    }
}

fn render_rename_column(
    table_ref: &str,
    old_name: &str,
    new_name: &str,
    db_type: DbType,
) -> String {
    match db_type {
        DbType::MsSqlServer => {
            format!(
                "EXEC sp_rename '{}.{}', '{}', 'COLUMN';",
                escape_sql_literal(table_ref),
                escape_sql_literal(old_name),
                escape_sql_literal(new_name)
            )
        }
        _ => format!(
            "ALTER TABLE {} RENAME COLUMN {} TO {};",
            table_ref,
            qi(old_name, db_type),
            qi(new_name, db_type)
        ),
    }
}

// ===========================================================================
// Metadata queries
// ===========================================================================

fn expand_load_table_metadata(json_str: &str, db_type: DbType) -> Result<ExpandedQuery, String> {
    let p: LoadTableMetadataPayload = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid LOAD_TABLE_METADATA payload: {}", e))?;

    // BigQuery all-tables requires a Bun (JS) script, not SQL
    if db_type == DbType::Bigquery && p.table.is_none() {
        let code = make_bigquery_all_tables_bun_script();
        return Ok(ExpandedQuery::with_language(code, ScriptLang::Bun));
    }

    let query =
        make_load_table_metadata_query(db_type, p.table.as_deref(), p.database_name.as_deref())?;
    Ok(ExpandedQuery::sql(maybe_wrap_ducklake(
        query,
        p.ducklake.as_deref(),
    )))
}

fn make_load_table_metadata_query(
    db_type: DbType,
    table: Option<&str>,
    database_name: Option<&str>,
) -> Result<String, String> {
    match db_type {
        DbType::Duckdb => {
            // For ducklake, the ducklake ATTACH is handled by the ducklake wrapper.
            let mut q = String::from(
                "SELECT
    COLUMN_NAME as field,
    DATA_TYPE as DataType,
    COLUMN_DEFAULT as DefaultValue,
    false as IsPrimaryKey,
    false as IsIdentity,
    IS_NULLABLE as IsNullable,
    false as IsEnum,
    TABLE_NAME as table_name
FROM information_schema.columns c
WHERE table_schema = current_schema()",
            );
            if let Some(t) = table {
                q.push_str(&format!(" AND TABLE_NAME = '{}'", escape_sql_literal(t)));
            }
            Ok(q)
        }
        DbType::Mysql => {
            let db_name = database_name.unwrap_or("");
            let table_filter = if let Some(t) = table {
                let parts: Vec<&str> = t.split('.').collect();
                let tname = parts[parts.len() - 1];
                let schema = if parts.len() > 1 { parts[0] } else { db_name };
                format!(
                    "\nWHERE\n    TABLE_NAME = '{}' AND TABLE_SCHEMA = '{}'",
                    escape_sql_literal(tname),
                    escape_sql_literal(schema)
                )
            } else {
                "\nWHERE\n    TABLE_SCHEMA NOT IN ('mysql', 'performance_schema', 'information_schema', 'sys', '_vt')".to_string()
            };
            let extra_col = if table.is_none() {
                ",\n    TABLE_NAME as table_name"
            } else {
                ""
            };
            Ok(format!(
                "SELECT \n    COLUMN_NAME as field,\n    COLUMN_TYPE as DataType,\n    COLUMN_DEFAULT as DefaultValue,\n    CASE WHEN COLUMN_KEY = 'PRI' THEN 1 ELSE 0 END as IsPrimaryKey,\n    CASE WHEN EXTRA like '%auto_increment%' THEN 'YES' ELSE 'NO' END as IsIdentity,\n    CASE WHEN IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,\n    CASE WHEN DATA_TYPE = 'enum' THEN true ELSE false END as IsEnum{}\nFROM \n    INFORMATION_SCHEMA.COLUMNS{}\nORDER BY\n    TABLE_NAME,\n    ORDINAL_POSITION;",
                extra_col, table_filter
            ))
        }
        DbType::Postgresql => {
            let (where_clause, extra_cols, joins, order) = if let Some(t) = table {
                let parts: Vec<&str> = t.split('.').collect();
                let tname = parts[parts.len() - 1];
                let schema = if parts.len() > 1 { parts[0] } else { "public" };
                (
                    format!(
                        "\nWHERE a.attrelid = (SELECT c.oid FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace ns ON c.relnamespace = ns.oid WHERE relname = '{}' AND ns.nspname = '{}')\n    AND a.attnum > 0 AND NOT a.attisdropped",
                        escape_sql_literal(tname),
                        escape_sql_literal(schema)
                    ),
                    String::new(),
                    String::new(),
                    "a.attnum".to_string(),
                )
            } else {
                (
                    "\nWHERE c.relkind = 'r' AND a.attnum > 0 AND NOT a.attisdropped\n    AND ns.nspname != 'pg_catalog' AND ns.nspname != 'information_schema'".to_string(),
                    ",\n    ns.nspname AS schema_name,\n    c.relname AS table_name".to_string(),
                    "\nJOIN pg_catalog.pg_class c ON a.attrelid = c.oid\nJOIN pg_catalog.pg_namespace ns ON c.relnamespace = ns.oid".to_string(),
                    "ns.nspname, c.relname, a.attnum".to_string(),
                )
            };
            Ok(format!(
                "SELECT \n    a.attname as field,\n    pg_catalog.format_type(a.atttypid, a.atttypmod) as DataType,\n    (SELECT substring(pg_catalog.pg_get_expr(d.adbin, d.adrelid, true) for 128)\n     FROM pg_catalog.pg_attrdef d\n     WHERE d.adrelid = a.attrelid AND d.adnum = a.attnum AND a.atthasdef) as DefaultValue,\n    (SELECT CASE WHEN i.indisprimary THEN true ELSE 'NO' END\n     FROM pg_catalog.pg_class tbl, pg_catalog.pg_class idx, pg_catalog.pg_index i, pg_catalog.pg_attribute att\n     WHERE tbl.oid = a.attrelid AND idx.oid = i.indexrelid AND att.attrelid = tbl.oid\n                     AND i.indrelid = tbl.oid AND att.attnum = any(i.indkey) AND att.attname = a.attname LIMIT 1) as IsPrimaryKey,\n    CASE a.attidentity\n            WHEN 'd' THEN 'By Default'\n            WHEN 'a' THEN 'Always'\n            ELSE 'No'\n    END as IsIdentity,\n    CASE a.attnotnull\n            WHEN false THEN 'YES'\n            ELSE 'NO'\n    END as IsNullable,\n    (SELECT true\n     FROM pg_catalog.pg_enum e\n     WHERE e.enumtypid = a.atttypid FETCH FIRST ROW ONLY) as IsEnum{}\nFROM pg_catalog.pg_attribute a{}{}\nORDER BY {};",
                extra_cols, joins, where_clause, order
            ))
        }
        DbType::MsSqlServer => {
            let table_filter = if let Some(t) = table {
                format!("\nWHERE\n    c.TABLE_NAME = '{}'", escape_sql_literal(t))
            } else {
                "\nWHERE\n    c.TABLE_SCHEMA != 'sys'".to_string()
            };
            let extra_col = if table.is_none() {
                ",\n    c.TABLE_NAME as table_name"
            } else {
                ""
            };
            Ok(format!(
                "SELECT\n    c.COLUMN_NAME as field,\n    c.DATA_TYPE as DataType,\n    c.COLUMN_DEFAULT as DefaultValue,\n    CASE WHEN COLUMNPROPERTY(OBJECT_ID(c.TABLE_SCHEMA + '.' + c.TABLE_NAME), c.COLUMN_NAME, 'IsIdentity') = 1 THEN 'By Default' ELSE 'No' END as IsIdentity,\n    CASE WHEN pk.COLUMN_NAME IS NOT NULL THEN 1 ELSE 0 END as IsPrimaryKey,\n    CASE WHEN c.IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,\n    CASE WHEN c.DATA_TYPE = 'enum' THEN 1 ELSE 0 END as IsEnum,\n    dc.name as default_constraint_name{}\nFROM\n    INFORMATION_SCHEMA.COLUMNS c\n    LEFT JOIN (\n        SELECT\n            ku.TABLE_SCHEMA,\n            ku.TABLE_NAME,\n            ku.COLUMN_NAME\n        FROM\n            INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc\n            INNER JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE ku\n                ON tc.CONSTRAINT_TYPE = 'PRIMARY KEY'\n                AND tc.CONSTRAINT_NAME = ku.CONSTRAINT_NAME\n                AND tc.TABLE_SCHEMA = ku.TABLE_SCHEMA\n                AND tc.TABLE_NAME = ku.TABLE_NAME\n    ) pk ON c.TABLE_SCHEMA = pk.TABLE_SCHEMA\n        AND c.TABLE_NAME = pk.TABLE_NAME\n        AND c.COLUMN_NAME = pk.COLUMN_NAME\n    LEFT JOIN sys.default_constraints dc\n        ON dc.parent_object_id = OBJECT_ID(c.TABLE_SCHEMA + '.' + c.TABLE_NAME)\n        AND dc.parent_column_id = COLUMNPROPERTY(OBJECT_ID(c.TABLE_SCHEMA + '.' + c.TABLE_NAME), c.COLUMN_NAME, 'ColumnId'){}\nORDER BY\n    c.ORDINAL_POSITION;",
                extra_col, table_filter
            ))
        }
        DbType::Snowflake => {
            let table_filter = if let Some(t) = table {
                let parts: Vec<&str> = t.split('.').collect();
                let tname = parts[parts.len() - 1];
                let schema = if parts.len() > 1 { parts[0] } else { "PUBLIC" };
                format!(
                    "\nwhere table_name = '{}' and table_schema = '{}'",
                    escape_sql_literal(tname),
                    escape_sql_literal(schema)
                )
            } else {
                "\nwhere table_schema <> 'INFORMATION_SCHEMA'\n".to_string()
            };
            let extra_col = if table.is_none() {
                ",\n    table_name as table_name,\n    table_schema as schema_name"
            } else {
                ""
            };
            Ok(format!(
                "select COLUMN_NAME as field,\n    DATA_TYPE as DataType,\n    COLUMN_DEFAULT as DefaultValue,\n    CASE WHEN COLUMN_DEFAULT like 'AUTOINCREMENT%' THEN 'By Default' ELSE 'No' END as IsIdentity,\n    0 as IsPrimaryKey,\n    CASE WHEN IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,\n    CASE WHEN DATA_TYPE = 'enum' THEN 1 ELSE 0 END as IsEnum{}\nfrom information_schema.columns{}\norder by ORDINAL_POSITION;",
                extra_col, table_filter
            ))
        }
        DbType::Bigquery => {
            let t = table.ok_or("BigQuery table must be in dataset.table format")?;
            let parts: Vec<&str> = t.splitn(2, '.').collect();
            if parts.len() < 2 {
                return Err("BigQuery table must be in dataset.table format".to_string());
            }
            let dataset = parts[0];
            let tname = parts[1];
            Ok(format!(
                "SELECT \n    c.COLUMN_NAME as field,\n    DATA_TYPE as DataType,\n    CASE WHEN COLUMN_DEFAULT = 'NULL' THEN '' ELSE COLUMN_DEFAULT END as DefaultValue,\n    CASE WHEN constraint_name is not null THEN true ELSE false END as IsPrimaryKey,\n    'No' as IsIdentity,\n    IS_NULLABLE as IsNullable,\n    false as IsEnum\nFROM\n    {}.INFORMATION_SCHEMA.COLUMNS c\n    LEFT JOIN\n    {}.INFORMATION_SCHEMA.KEY_COLUMN_USAGE p\n    on c.table_name = p.table_name AND c.column_name = p.COLUMN_NAME\nWHERE   \n    c.TABLE_NAME = '{}'\norder by c.ORDINAL_POSITION;",
                dataset, dataset, escape_sql_literal(tname)
            ))
        }
    }
}

/// BigQuery all-tables metadata requires querying each dataset's INFORMATION_SCHEMA,
/// which can only be done via the BigQuery JS SDK. This generates a Bun script.
fn make_bigquery_all_tables_bun_script() -> String {
    r#"import { BigQuery } from '@google-cloud/bigquery@7.5.0';
export async function main(database: bigquery) {
const bq = new BigQuery({
	credentials: database
})
const [datasets] = await bq.getDatasets();
if (!datasets) return {}
const schema = {} as any
let queries = datasets.map(dataset => `
	(SELECT
    c.COLUMN_NAME as field,
		'${dataset.id}' as schema_name,
		c.TABLE_NAME as table_name,
    DATA_TYPE as DataType,
    CASE WHEN COLUMN_DEFAULT = 'NULL' THEN '' ELSE COLUMN_DEFAULT END as DefaultValue,
    CASE WHEN constraint_name is not null THEN true ELSE false END as IsPrimaryKey,
    'No' as IsIdentity,
    IS_NULLABLE as IsNullable,
    false as IsEnum
FROM
    \`${dataset.id}\`.INFORMATION_SCHEMA.COLUMNS c
    LEFT JOIN
    \`${dataset.id}\`.INFORMATION_SCHEMA.KEY_COLUMN_USAGE p
    on c.table_name = p.table_name AND c.column_name = p.COLUMN_NAME
ORDER BY c.ORDINAL_POSITION)`
)
let query = queries.join('\nUNION ALL \n')
const [rows] = await bq.query(query)
return rows
}"#
    .to_string()
}

// --- Relational keys queries ---

fn expand_foreign_keys(json_str: &str, db_type: DbType) -> Result<String, String> {
    let p: ForeignKeysPayload = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid FOREIGN_KEYS payload: {}", e))?;
    let query = make_foreign_keys_query(db_type, &p.table, p.schema.as_deref())?;
    Ok(maybe_wrap_ducklake(query, p.ducklake.as_deref()))
}

fn expand_primary_key_constraint(json_str: &str, db_type: DbType) -> Result<String, String> {
    let p: PrimaryKeyConstraintPayload = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid PRIMARY_KEY_CONSTRAINT payload: {}", e))?;
    let query = make_primary_key_constraint_query(db_type, &p.table, p.schema.as_deref())?;
    Ok(maybe_wrap_ducklake(query, p.ducklake.as_deref()))
}

#[derive(Deserialize)]
struct SnowflakePrimaryKeysPayload {
    table: Option<String>,
}

fn expand_snowflake_primary_keys(json_str: &str) -> Result<String, String> {
    let p: SnowflakePrimaryKeysPayload = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid SNOWFLAKE_PRIMARY_KEYS payload: {}", e))?;
    match p.table {
        Some(table) => Ok(format!("SHOW PRIMARY KEYS IN TABLE {}", table)),
        None => Ok("SHOW PRIMARY KEYS IN ACCOUNT".to_string()),
    }
}

fn make_foreign_keys_query(
    db_type: DbType,
    table: &str,
    default_schema: Option<&str>,
) -> Result<String, String> {
    let parts: Vec<&str> = table.split('.').collect();
    let table_name = parts[parts.len() - 1];
    let schema_name = if parts.len() > 1 {
        parts[0]
    } else {
        default_schema.unwrap_or(match db_type {
            DbType::Postgresql => "public",
            DbType::Duckdb => "main",
            DbType::Mysql => "",
            DbType::MsSqlServer => "dbo",
            DbType::Snowflake => "PUBLIC",
            DbType::Bigquery => return Err("BigQuery requires a schema for FK queries".to_string()),
        })
    };

    let tn = escape_sql_literal(table_name);
    let sn = escape_sql_literal(schema_name);

    match db_type {
        DbType::Postgresql => Ok(format!(
            "SELECT\n    tc.constraint_name as fk_constraint_name,\n    kcu.column_name as source_column,\n    ccu.table_schema || '.' || ccu.table_name as target_table,\n    ccu.column_name as target_column,\n    COALESCE(rc.delete_rule, 'NO ACTION') as on_delete,\n    COALESCE(rc.update_rule, 'NO ACTION') as on_update\nFROM\n    information_schema.table_constraints AS tc\n    JOIN information_schema.key_column_usage AS kcu\n        ON tc.constraint_name = kcu.constraint_name\n        AND tc.table_schema = kcu.table_schema\n    JOIN information_schema.constraint_column_usage AS ccu\n        ON ccu.constraint_name = tc.constraint_name\n        AND ccu.table_schema = tc.table_schema\n    LEFT JOIN information_schema.referential_constraints AS rc\n        ON rc.constraint_name = tc.constraint_name\n        AND rc.constraint_schema = tc.table_schema\nWHERE\n    tc.constraint_type = 'FOREIGN KEY'\n    AND tc.table_name = '{}'\n    AND tc.table_schema = '{}'\nORDER BY\n    tc.constraint_name, kcu.ordinal_position;",
            tn, sn
        )),
        DbType::Mysql => {
            let where_schema = if schema_name.is_empty() {
                "AND kcu.TABLE_SCHEMA = DATABASE()".to_string()
            } else {
                format!("AND kcu.TABLE_SCHEMA = '{}'", sn)
            };
            Ok(format!(
                "SELECT\n    kcu.CONSTRAINT_NAME as fk_constraint_name,\n    kcu.COLUMN_NAME as source_column,\n    CONCAT(kcu.REFERENCED_TABLE_SCHEMA, '.', kcu.REFERENCED_TABLE_NAME) as target_table,\n    kcu.REFERENCED_COLUMN_NAME as target_column,\n    COALESCE(rc.DELETE_RULE, 'NO ACTION') as on_delete,\n    COALESCE(rc.UPDATE_RULE, 'NO ACTION') as on_update\nFROM\n    INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu\n    JOIN INFORMATION_SCHEMA.REFERENTIAL_CONSTRAINTS rc\n        ON kcu.CONSTRAINT_NAME = rc.CONSTRAINT_NAME\n        AND kcu.TABLE_SCHEMA = rc.CONSTRAINT_SCHEMA\nWHERE\n    kcu.TABLE_NAME = '{}'\n    {}\n    AND kcu.REFERENCED_TABLE_NAME IS NOT NULL\nORDER BY\n    kcu.CONSTRAINT_NAME, kcu.ORDINAL_POSITION;",
                tn, where_schema
            ))
        }
        DbType::MsSqlServer => Ok(format!(
            "SELECT\n    fk.name as fk_constraint_name,\n    COL_NAME(fkc.parent_object_id, fkc.parent_column_id) as source_column,\n    OBJECT_SCHEMA_NAME(fkc.referenced_object_id) + '.' + OBJECT_NAME(fkc.referenced_object_id) as target_table,\n    COL_NAME(fkc.referenced_object_id, fkc.referenced_column_id) as target_column,\n    CASE fk.delete_referential_action\n        WHEN 0 THEN 'NO ACTION'\n        WHEN 1 THEN 'CASCADE'\n        WHEN 2 THEN 'SET NULL'\n        WHEN 3 THEN 'SET DEFAULT'\n    END as on_delete,\n    CASE fk.update_referential_action\n        WHEN 0 THEN 'NO ACTION'\n        WHEN 1 THEN 'CASCADE'\n        WHEN 2 THEN 'SET NULL'\n        WHEN 3 THEN 'SET DEFAULT'\n    END as on_update\nFROM\n    sys.foreign_keys fk\n    INNER JOIN sys.foreign_key_columns fkc\n        ON fk.object_id = fkc.constraint_object_id\n    INNER JOIN sys.tables t\n        ON fk.parent_object_id = t.object_id\n    INNER JOIN sys.schemas s\n        ON t.schema_id = s.schema_id\nWHERE\n    t.name = '{}'\n    AND s.name = '{}'\nORDER BY\n    fk.name, fkc.constraint_column_id;",
            tn, sn
        )),
        DbType::Snowflake => Ok(format!(
            "SHOW IMPORTED KEYS IN TABLE {}.{}",
            schema_name, table_name
        )),
        DbType::Bigquery => Ok(format!(
            "SELECT\n    tc.constraint_name as fk_constraint_name,\n    kcu.column_name as source_column,\n    ccu.table_name as target_table,\n    ccu.column_name as target_column,\n    'NO ACTION' as on_delete,\n    'NO ACTION' as on_update\nFROM\n    `{}.INFORMATION_SCHEMA.TABLE_CONSTRAINTS` tc\n    JOIN `{}.INFORMATION_SCHEMA.KEY_COLUMN_USAGE` kcu\n        ON tc.constraint_name = kcu.constraint_name\n    JOIN `{}.INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE` ccu\n        ON tc.constraint_name = ccu.constraint_name\nWHERE\n    tc.constraint_type = 'FOREIGN KEY'\n    AND tc.table_name = '{}'\nORDER BY\n    tc.constraint_name, kcu.ordinal_position;",
            schema_name, schema_name, schema_name, tn
        )),
        DbType::Duckdb => Ok(format!(
            "SELECT\n    fk_constraint.constraint_name as fk_constraint_name,\n    kcu.column_name as source_column,\n    ccu.table_schema || '.' || ccu.table_name as target_table,\n    ccu.column_name as target_column,\n    'NO ACTION' as on_delete,\n    'NO ACTION' as on_update\nFROM\n    information_schema.table_constraints fk_constraint\n    JOIN information_schema.key_column_usage kcu\n        ON fk_constraint.constraint_name = kcu.constraint_name\n        AND fk_constraint.constraint_schema = kcu.constraint_schema\n    JOIN information_schema.constraint_column_usage ccu\n        ON fk_constraint.constraint_name = ccu.constraint_name\n        AND fk_constraint.constraint_schema = ccu.constraint_schema\nWHERE\n    fk_constraint.constraint_type = 'FOREIGN KEY'\n    AND fk_constraint.table_name = '{}'\n    AND fk_constraint.table_schema = '{}'\nORDER BY\n    fk_constraint.constraint_name, kcu.ordinal_position;",
            tn, sn
        )),
    }
}

fn make_primary_key_constraint_query(
    db_type: DbType,
    table: &str,
    default_schema: Option<&str>,
) -> Result<String, String> {
    let parts: Vec<&str> = table.split('.').collect();
    let table_name = parts[parts.len() - 1];
    let schema_name = if parts.len() > 1 {
        parts[0]
    } else {
        default_schema.unwrap_or(match db_type {
            DbType::Postgresql => "public",
            DbType::Duckdb => "main",
            DbType::Mysql => "",
            DbType::MsSqlServer => "dbo",
            DbType::Snowflake => "PUBLIC",
            DbType::Bigquery => return Err("BigQuery requires a schema for PK queries".to_string()),
        })
    };

    let tn = escape_sql_literal(table_name);
    let sn = escape_sql_literal(schema_name);

    match db_type {
        DbType::Postgresql | DbType::Duckdb => Ok(format!(
            "SELECT\n    tc.constraint_name\nFROM\n    information_schema.table_constraints AS tc\nWHERE\n    tc.constraint_type = 'PRIMARY KEY'\n    AND tc.table_name = '{}'\n    AND tc.table_schema = '{}'\nLIMIT 1;",
            tn, sn
        )),
        DbType::Mysql => {
            let where_schema = if schema_name.is_empty() {
                "AND tc.TABLE_SCHEMA = DATABASE()".to_string()
            } else {
                format!("AND tc.TABLE_SCHEMA = '{}'", sn)
            };
            Ok(format!(
                "SELECT\n    tc.CONSTRAINT_NAME as constraint_name\nFROM\n    INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc\nWHERE\n    tc.CONSTRAINT_TYPE = 'PRIMARY KEY'\n    AND tc.TABLE_NAME = '{}'\n    {}\nLIMIT 1;",
                tn, where_schema
            ))
        }
        DbType::MsSqlServer => Ok(format!(
            "SELECT\n    kc.name as constraint_name\nFROM\n    sys.key_constraints kc\n    INNER JOIN sys.tables t\n        ON kc.parent_object_id = t.object_id\n    INNER JOIN sys.schemas s\n        ON t.schema_id = s.schema_id\nWHERE\n    kc.type = 'PK'\n    AND t.name = '{}'\n    AND s.name = '{}';",
            tn, sn
        )),
        DbType::Snowflake => Ok(format!(
            "SELECT\n    constraint_name\nFROM\n    information_schema.table_constraints\nWHERE\n    constraint_type = 'PRIMARY KEY'\n    AND table_name = '{}'\n    AND table_schema = '{}'\nLIMIT 1;",
            tn, sn
        )),
        DbType::Bigquery => Ok(format!(
            "SELECT\n    constraint_name\nFROM\n    `{}.INFORMATION_SCHEMA.TABLE_CONSTRAINTS`\nWHERE\n    constraint_type = 'PRIMARY KEY'\n    AND table_name = '{}'\nLIMIT 1;",
            schema_name, tn
        )),
    }
}

// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn col(field: &str, datatype: &str) -> ColumnDef {
        ColumnDef::simple(field, datatype)
    }

    fn simple_col(field: &str, datatype: &str) -> SimpleColumn {
        SimpleColumn { field: field.to_string(), datatype: datatype.to_string() }
    }

    // -----------------------------------------------------------------------
    // build_parameters
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_parameters_postgresql() {
        let cols = vec![simple_col("limit", "int"), simple_col("offset", "int")];
        let result = build_parameters(&cols, DbType::Postgresql);
        assert_eq!(result, "-- $1 limit\n-- $2 offset");
    }

    #[test]
    fn test_build_parameters_mysql() {
        let cols = vec![
            simple_col("limit", "int"),
            simple_col("name", "varchar(255)"),
        ];
        let result = build_parameters(&cols, DbType::Mysql);
        assert_eq!(result, "-- :limit (int)\n-- :name (varchar)");
    }

    #[test]
    fn test_build_parameters_mssql() {
        let cols = vec![
            simple_col("limit", "int"),
            simple_col("name", "nvarchar(50)"),
        ];
        let result = build_parameters(&cols, DbType::MsSqlServer);
        assert_eq!(result, "-- @p1 limit (int)\n-- @p2 name (nvarchar)");
    }

    #[test]
    fn test_build_parameters_snowflake() {
        let cols = vec![
            simple_col("quicksearch", "text"),
            simple_col("order_by", "text"),
        ];
        let result = build_parameters(&cols, DbType::Snowflake);
        assert_eq!(result, "-- ? quicksearch (text)\n-- ? order_by (text)");
    }

    #[test]
    fn test_build_parameters_bigquery() {
        let cols = vec![
            simple_col("quicksearch", "string"),
            simple_col("order_by", "string"),
        ];
        let result = build_parameters(&cols, DbType::Bigquery);
        assert_eq!(result, "-- @quicksearch (string)\n-- @order_by (string)");
    }

    #[test]
    fn test_build_parameters_duckdb() {
        let cols = vec![simple_col("limit", "int"), simple_col("offset", "int")];
        let result = build_parameters(&cols, DbType::Duckdb);
        assert_eq!(result, "-- $limit (int)\n-- $offset (int)");
    }

    // -----------------------------------------------------------------------
    // render_db_quoted_identifier
    // -----------------------------------------------------------------------

    #[test]
    fn test_quoted_identifiers() {
        assert_eq!(
            render_db_quoted_identifier("col", DbType::Postgresql),
            "\"col\""
        );
        assert_eq!(render_db_quoted_identifier("col", DbType::Mysql), "`col`");
        assert_eq!(
            render_db_quoted_identifier("col", DbType::MsSqlServer),
            "[col]"
        );
        assert_eq!(
            render_db_quoted_identifier("col", DbType::Snowflake),
            "\"col\""
        );
        assert_eq!(
            render_db_quoted_identifier("col", DbType::Bigquery),
            "`col`"
        );
        assert_eq!(
            render_db_quoted_identifier("col", DbType::Duckdb),
            "\"col\""
        );
    }

    // -----------------------------------------------------------------------
    // build_visible_field_list
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_visible_field_list_filters_ignored() {
        let cols = vec![
            col("id", "int4"),
            {
                let mut c = col("hidden", "text");
                c.ignored = Some(true);
                c
            },
            col("name", "text"),
        ];
        let result = build_visible_field_list(&cols, DbType::Postgresql);
        assert_eq!(result, vec!["\"id\"", "\"name\""]);
    }

    // -----------------------------------------------------------------------
    // SELECT - PostgreSQL
    // -----------------------------------------------------------------------

    #[test]
    fn test_select_postgresql_basic() {
        let cols = vec![col("id", "int4"), col("name", "text")];
        let result =
            make_select_query("my_table", &cols, None, DbType::Postgresql, None, None).unwrap();

        assert!(result.starts_with(
            "-- $1 limit\n-- $2 offset\n-- $3 quicksearch\n-- $4 order_by\n-- $5 is_desc\n"
        ));
        assert!(result.contains("SELECT \"id\"::text, \"name\"::text FROM \"my_table\"\n"));
        assert!(result.contains("($3 = '' OR CONCAT(\"id\", \"name\") ILIKE '%' || $3 || '%')"));
        assert!(result.contains("LIMIT $1::INT OFFSET $2::INT"));
        assert!(result.contains("$4 = 'id' AND $5 IS false THEN \"id\"::text"));
        assert!(result.contains("$4 = 'name' AND $5 IS true THEN \"name\"::text END) DESC"));
    }

    #[test]
    fn test_select_postgresql_with_where() {
        let cols = vec![col("id", "int4")];
        let result = make_select_query(
            "my_table",
            &cols,
            Some("status = 'active'"),
            DbType::Postgresql,
            None,
            None,
        )
        .unwrap();
        assert!(result.contains(
            "WHERE status = 'active' AND ($3 = '' OR CONCAT(\"id\") ILIKE '%' || $3 || '%')"
        ));
    }

    #[test]
    fn test_select_postgresql_fix_pg_int_types() {
        let cols = vec![col("id", "int4"), col("data", "json")];
        let bf = BreakingFeatures { fix_pg_int_types: true };
        let result =
            make_select_query("my_table", &cols, None, DbType::Postgresql, None, Some(&bf))
                .unwrap();

        // json column should get text cast only (no number check)
        assert!(result.contains("$4 = 'data' AND $5 IS false THEN \"data\"::text END)"));
        // int column should have number check variants
        assert!(result.contains("pg_typeof(\"id\")::text NOT IN"));
        assert!(result.contains("pg_typeof(\"id\")::text IN"));
    }

    #[test]
    fn test_select_postgresql_fix_pg_array_type() {
        let cols = vec![col("tags", "text[]")];
        let bf = BreakingFeatures { fix_pg_int_types: true };
        let result =
            make_select_query("my_table", &cols, None, DbType::Postgresql, None, Some(&bf))
                .unwrap();

        // Array type should force text cast (no number check variants)
        assert!(result.contains("$4 = 'tags' AND $5 IS false THEN \"tags\"::text END)"));
        // Should NOT contain pg_typeof checks
        assert!(!result.contains("pg_typeof(\"tags\")"));
    }

    // -----------------------------------------------------------------------
    // SELECT - MySQL
    // -----------------------------------------------------------------------

    #[test]
    fn test_select_mysql_basic() {
        let cols = vec![col("id", "int"), col("name", "varchar")];
        let result = make_select_query("my_table", &cols, None, DbType::Mysql, None, None).unwrap();

        assert!(result.starts_with("-- :limit (int)\n-- :offset (int)\n-- :quicksearch (text)\n-- :order_by (text)\n-- :is_desc (boolean)\n"));
        assert!(result.contains("SELECT `id`, `name` FROM `my_table`"));
        assert!(result.contains("CONCAT_WS(' ', `id`, `name`) LIKE CONCAT('%', :quicksearch, '%')"));
        assert!(result.contains("LIMIT :limit OFFSET :offset"));
        assert!(result.contains(":order_by = 'id' AND :is_desc IS false THEN `id` END"));
    }

    // -----------------------------------------------------------------------
    // SELECT - MS SQL Server
    // -----------------------------------------------------------------------

    #[test]
    fn test_select_mssql_basic() {
        let cols = vec![col("id", "int"), col("name", "nvarchar")];
        let result =
            make_select_query("my_table", &cols, None, DbType::MsSqlServer, None, None).unwrap();

        assert!(result.contains("SELECT [id], [name] FROM [my_table]"));
        assert!(result.contains("@p3 = '' OR CONCAT([id], [name]) LIKE '%' + @p3 + '%'"));
        assert!(result.contains("OFFSET @p2 ROWS FETCH NEXT @p1 ROWS ONLY"));
    }

    #[test]
    fn test_select_mssql_unsortable_type() {
        let cols = vec![col("id", "int"), col("body", "text")];
        let result =
            make_select_query("my_table", &cols, None, DbType::MsSqlServer, None, None).unwrap();

        // text type should be excluded from ORDER BY
        assert!(!result.contains("@p4 = 'body'"));
        // text type should be excluded from search clause
        assert!(!result.contains("CONCAT([id], [body])"));
        assert!(result.contains("CONCAT([id])"));
    }

    // -----------------------------------------------------------------------
    // SELECT - Snowflake
    // -----------------------------------------------------------------------

    #[test]
    fn test_select_snowflake_basic() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result =
            make_select_query("my_table", &cols, None, DbType::Snowflake, None, None).unwrap();

        // Parameters: quicksearch (1 + N filtered columns), then 4 per column for order_by
        assert!(result.contains("-- ? quicksearch (text)"));
        assert!(result.contains("SELECT \"id\", \"name\" FROM \"my_table\""));
        assert!(result.contains("LENGTH(?) = 0"));
        assert!(result.contains("CONCAT(\"id\") ILIKE CONCAT('%', ?, '%')"));
        assert!(result.contains("LIMIT 100 OFFSET 0"));
    }

    #[test]
    fn test_select_snowflake_custom_limit() {
        let cols = vec![col("id", "int")];
        let opts = SelectOptions { limit: Some(50), offset: Some(10) };
        let result = make_select_query(
            "my_table",
            &cols,
            None,
            DbType::Snowflake,
            Some(&opts),
            None,
        )
        .unwrap();
        assert!(result.contains("LIMIT 50 OFFSET 10"));
    }

    // -----------------------------------------------------------------------
    // SELECT - BigQuery
    // -----------------------------------------------------------------------

    #[test]
    fn test_select_bigquery_basic() {
        let cols = vec![col("id", "INTEGER"), col("name", "STRING")];
        let result =
            make_select_query("my_table", &cols, None, DbType::Bigquery, None, None).unwrap();

        assert!(result.starts_with("-- @limit (integer)\n-- @offset (integer)\n-- @quicksearch (string)\n-- @order_by (string)\n-- @is_desc (bool)\n"));
        assert!(result.contains("SELECT `id`, `name` FROM `my_table`"));
        assert!(result.contains("CAST(`id` AS STRING)"));
        assert!(result.contains("LIMIT @limit OFFSET @offset"));
    }

    #[test]
    fn test_select_bigquery_json_type() {
        let cols = vec![col("id", "INTEGER"), col("data", "JSON")];
        let result =
            make_select_query("my_table", &cols, None, DbType::Bigquery, None, None).unwrap();

        // JSON column should use TO_JSON_STRING in search and order
        assert!(result.contains("TO_JSON_STRING(`data`)"));
        assert!(result.contains("TO_JSON_STRING(data)"));
    }

    // -----------------------------------------------------------------------
    // SELECT - DuckDB
    // -----------------------------------------------------------------------

    #[test]
    fn test_select_duckdb_basic() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result =
            make_select_query("my_table", &cols, None, DbType::Duckdb, None, None).unwrap();

        assert!(result.starts_with("-- $limit (int)\n-- $offset (int)\n-- $quicksearch (text)\n-- $order_by (text)\n-- $is_desc (boolean)\n"));
        assert!(result.contains("SELECT \"id\", \"name\" FROM \"my_table\"\n"));
        assert!(result.contains("CONCAT(\"id\", \"name\") ILIKE '%' || $quicksearch || '%'"));
        assert!(result.contains("LIMIT $limit::INT OFFSET $offset::INT"));
    }

    // -----------------------------------------------------------------------
    // SELECT - error cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_select_empty_table_error() {
        let cols = vec![col("id", "int")];
        let result = make_select_query("", &cols, None, DbType::Postgresql, None, None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Table name is required");
    }

    // -----------------------------------------------------------------------
    // COUNT - PostgreSQL
    // -----------------------------------------------------------------------

    #[test]
    fn test_count_postgresql_basic() {
        let cols = vec![col("id", "int4"), col("name", "text")];
        let result = make_count_query(DbType::Postgresql, "my_table", None, &cols).unwrap();

        assert!(result.contains("-- $1 quicksearch"));
        assert!(result.contains("SELECT COUNT(*) as count FROM \"my_table\""));
        assert!(result.contains("($1 = '' OR CONCAT(\"id\", \"name\") ILIKE '%' || $1 || '%')"));
        // Should use WHERE not AND
        assert!(result.contains(" WHERE "));
        assert!(!result.contains("\"my_table\" AND "));
    }

    #[test]
    fn test_count_postgresql_with_where() {
        let cols = vec![col("id", "int4")];
        let result = make_count_query(
            DbType::Postgresql,
            "my_table",
            Some("status = 'active'"),
            &cols,
        )
        .unwrap();

        assert!(result.contains(
            "WHERE  status = 'active' AND ($1 = '' OR CONCAT(\"id\") ILIKE '%' || $1 || '%')"
        ));
    }

    #[test]
    fn test_count_postgresql_no_visible_columns() {
        let cols = vec![{
            let mut c = col("id", "int4");
            c.ignored = Some(true);
            c
        }];
        let result = make_count_query(DbType::Postgresql, "my_table", None, &cols).unwrap();
        assert!(result.contains("($1 = '' OR 1 = 1)"));
    }

    // -----------------------------------------------------------------------
    // COUNT - MySQL
    // -----------------------------------------------------------------------

    #[test]
    fn test_count_mysql_basic() {
        let cols = vec![col("id", "int"), col("name", "varchar")];
        let result = make_count_query(DbType::Mysql, "my_table", None, &cols).unwrap();

        assert!(result.contains("-- :quicksearch (text)"));
        assert!(result.contains("SELECT COUNT(*) as count FROM `my_table`"));
        assert!(result.contains("CONCAT_WS(' ', `id`, `name`) LIKE CONCAT('%', :quicksearch, '%')"));
    }

    // -----------------------------------------------------------------------
    // COUNT - MS SQL Server
    // -----------------------------------------------------------------------

    #[test]
    fn test_count_mssql_basic() {
        let cols = vec![col("id", "int"), col("name", "nvarchar")];
        let result = make_count_query(DbType::MsSqlServer, "my_table", None, &cols).unwrap();

        assert!(result.contains("SELECT COUNT(*) as count FROM [my_table]"));
        assert!(result.contains("(@p1 = '' OR CONCAT([id], +[name]) LIKE '%' + @p1 + '%')"));
    }

    // -----------------------------------------------------------------------
    // COUNT - Snowflake
    // -----------------------------------------------------------------------

    #[test]
    fn test_count_snowflake_basic() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result = make_count_query(DbType::Snowflake, "my_table", None, &cols).unwrap();

        // Two quicksearch params for snowflake with visible columns
        assert!(result.contains("-- ? quicksearch (text)\n-- ? quicksearch (text)"));
        assert!(result.contains("SELECT COUNT(*) as count FROM \"my_table\""));
        assert!(result.contains("(? = '' OR CONCAT(\"id\", \"name\") ILIKE '%' || ? || '%')"));
    }

    #[test]
    fn test_count_snowflake_no_visible_columns() {
        let cols = vec![{
            let mut c = col("id", "int");
            c.ignored = Some(true);
            c
        }];
        let result = make_count_query(DbType::Snowflake, "my_table", None, &cols).unwrap();
        // One quicksearch param
        let param_lines: Vec<&str> = result.lines().filter(|l| l.starts_with("-- ?")).collect();
        assert_eq!(param_lines.len(), 1);
        assert!(result.contains("(? = '' OR 1 = 1)"));
    }

    // -----------------------------------------------------------------------
    // COUNT - BigQuery
    // -----------------------------------------------------------------------

    #[test]
    fn test_count_bigquery_basic() {
        let cols = vec![col("id", "INTEGER"), col("name", "STRING")];
        let result = make_count_query(DbType::Bigquery, "my_table", None, &cols).unwrap();

        assert!(result.contains("-- @quicksearch (string)"));
        assert!(result.contains("SELECT COUNT(*) as count FROM `my_table`"));
        assert!(result.contains("REGEXP_CONTAINS(CONCAT(`id`,`name`), '(?i)' || @quicksearch)"));
    }

    #[test]
    fn test_count_bigquery_json_type() {
        let cols = vec![col("id", "INTEGER"), col("data", "JSON")];
        let result = make_count_query(DbType::Bigquery, "my_table", None, &cols).unwrap();
        assert!(result.contains("TO_JSON_STRING(`data`)"));
    }

    // -----------------------------------------------------------------------
    // COUNT - DuckDB
    // -----------------------------------------------------------------------

    #[test]
    fn test_count_duckdb_basic() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result = make_count_query(DbType::Duckdb, "my_table", None, &cols).unwrap();

        assert!(result.contains("-- $quicksearch (text)"));
        assert!(result.contains("SELECT COUNT(*) as count FROM \"my_table\""));
        assert!(
            result.contains("CONCAT(' ', \"id\", \"name\") LIKE CONCAT('%', $quicksearch, '%')")
        );
    }

    // -----------------------------------------------------------------------
    // DELETE - all DB types
    // -----------------------------------------------------------------------

    #[test]
    fn test_delete_postgresql() {
        let cols = vec![col("id", "int4"), col("name", "text")];
        let result = make_delete_query("my_table", &cols, DbType::Postgresql);

        assert!(result.contains("-- $1 id\n-- $2 name"));
        assert!(result.contains("DELETE FROM \"my_table\""));
        assert!(result
            .contains("($1::text::int4 IS NULL AND \"id\" IS NULL OR \"id\" = $1::text::int4)"));
        assert!(result.contains(
            "($2::text::text IS NULL AND \"name\" IS NULL OR \"name\" = $2::text::text)"
        ));
        assert!(result.contains("RETURNING 1;"));
    }

    #[test]
    fn test_delete_mysql() {
        let cols = vec![col("id", "int"), col("name", "varchar")];
        let result = make_delete_query("my_table", &cols, DbType::Mysql);

        assert!(result.contains("-- :id (int)\n-- :name (varchar)"));
        assert!(result.contains("DELETE FROM `my_table`"));
        assert!(result.contains("(:id IS NULL AND `id` IS NULL OR `id` = :id)"));
        assert!(result.contains("(:name IS NULL AND `name` IS NULL OR `name` = :name)"));
    }

    #[test]
    fn test_delete_mssql() {
        let cols = vec![col("id", "int"), col("name", "nvarchar")];
        let result = make_delete_query("my_table", &cols, DbType::MsSqlServer);

        assert!(result.contains("-- @p1 id (int)\n-- @p2 name (nvarchar)"));
        assert!(result.contains("DELETE FROM [my_table]"));
        assert!(result.contains("(@p1 IS NULL AND [id] IS NULL OR [id] = @p1)"));
        assert!(result.contains("(@p2 IS NULL AND [name] IS NULL OR [name] = @p2)"));
    }

    #[test]
    fn test_delete_snowflake() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result = make_delete_query("my_table", &cols, DbType::Snowflake);

        // Snowflake doubles columns
        assert!(result.contains("-- ? id (int)\n-- ? id (int)\n-- ? name (text)\n-- ? name (text)"));
        assert!(result.contains("DELETE FROM \"my_table\""));
        assert!(result.contains("(? = 'null' AND \"id\" IS NULL OR \"id\" = ?)"));
        assert!(result.contains("(? = 'null' AND \"name\" IS NULL OR \"name\" = ?)"));
    }

    #[test]
    fn test_delete_bigquery() {
        let cols = vec![col("id", "INTEGER"), col("name", "STRING")];
        let result = make_delete_query("my_table", &cols, DbType::Bigquery);

        assert!(result.contains("-- @id (INTEGER)\n-- @name (STRING)"));
        assert!(result.contains("DELETE FROM `my_table`"));
        assert!(result.contains("(CAST(@id AS STRING) = 'null' AND `id` IS NULL OR `id` = @id)"));
    }

    #[test]
    fn test_delete_duckdb() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result = make_delete_query("my_table", &cols, DbType::Duckdb);

        assert!(result.contains("-- $id (int)\n-- $name (text)"));
        assert!(result.contains("DELETE FROM \"my_table\""));
        assert!(result.contains("($id IS NULL AND \"id\" IS NULL OR \"id\" = $id)"));
    }

    // -----------------------------------------------------------------------
    // INSERT - basic cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_insert_postgresql_basic() {
        let cols = vec![col("id", "int4"), col("name", "text")];
        let result = make_insert_query("my_table", &cols, DbType::Postgresql).unwrap();

        assert!(result.contains("-- $1 id\n-- $2 name"));
        assert!(result
            .contains("INSERT INTO \"my_table\" (\"id\", \"name\") VALUES ($1::int4, $2::text)"));
    }

    #[test]
    fn test_insert_mysql_basic() {
        let cols = vec![col("id", "int"), col("name", "varchar")];
        let result = make_insert_query("my_table", &cols, DbType::Mysql).unwrap();

        assert!(result.contains("INSERT INTO `my_table` (`id`, `name`) VALUES (:id, :name)"));
    }

    #[test]
    fn test_insert_mssql_basic() {
        let cols = vec![col("id", "int"), col("name", "nvarchar")];
        let result = make_insert_query("my_table", &cols, DbType::MsSqlServer).unwrap();

        assert!(result.contains("INSERT INTO [my_table] ([id], [name]) VALUES (@p1, @p2)"));
    }

    #[test]
    fn test_insert_snowflake_basic() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result = make_insert_query("my_table", &cols, DbType::Snowflake).unwrap();

        assert!(result.contains("INSERT INTO \"my_table\" (\"id\", \"name\") VALUES (?, ?)"));
    }

    #[test]
    fn test_insert_bigquery_basic() {
        let cols = vec![col("id", "INTEGER"), col("name", "STRING")];
        let result = make_insert_query("my_table", &cols, DbType::Bigquery).unwrap();

        assert!(result.contains("INSERT INTO `my_table` (`id`, `name`) VALUES (@id, @name)"));
    }

    #[test]
    fn test_insert_duckdb_basic() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result = make_insert_query("my_table", &cols, DbType::Duckdb).unwrap();

        assert!(result.contains("INSERT INTO \"my_table\" (\"id\", \"name\") VALUES ($id, $name)"));
    }

    // -----------------------------------------------------------------------
    // INSERT - with defaults and identity
    // -----------------------------------------------------------------------

    #[test]
    fn test_insert_postgresql_skip_nextval() {
        let mut id_col = col("id", "int4");
        id_col.defaultvalue = "nextval('my_table_id_seq')".to_string();
        let name_col = col("name", "text");
        let result =
            make_insert_query("my_table", &[id_col, name_col], DbType::Postgresql).unwrap();

        // id should be skipped from insert columns (has nextval default in pg)
        assert!(result.contains("INSERT INTO \"my_table\" (\"name\") VALUES ($1::text)"));
    }

    #[test]
    fn test_insert_with_hidden_column_and_user_default() {
        let mut id_col = col("id", "int4");
        id_col.hide_insert = Some(true);
        id_col.default_user_value = Some(serde_json::Value::String("42".to_string()));

        let name_col = col("name", "text");
        let result =
            make_insert_query("my_table", &[id_col, name_col], DbType::Postgresql).unwrap();

        // name should be in insert params, id should be in defaults
        assert!(
            result.contains("INSERT INTO \"my_table\" (\"name\", \"id\") VALUES ($1::text, '42')")
        );
    }

    #[test]
    fn test_insert_with_hidden_column_null_default() {
        let mut id_col = col("id", "int4");
        id_col.hide_insert = Some(true);
        id_col.default_value_null = Some(true);

        let name_col = col("name", "text");
        let result =
            make_insert_query("my_table", &[id_col, name_col], DbType::Postgresql).unwrap();

        assert!(result.contains("VALUES ($1::text, NULL)"));
    }

    #[test]
    fn test_insert_with_hidden_column_db_default_not_nullable() {
        let mut id_col = col("id", "int4");
        id_col.hide_insert = Some(true);
        id_col.defaultvalue = "0".to_string();
        id_col.isnullable = "NO".to_string();

        let name_col = col("name", "text");
        let result =
            make_insert_query("my_table", &[id_col, name_col], DbType::Postgresql).unwrap();

        // Column is hidden, not nullable, has db default, no user default -> omit (use db default)
        assert!(result.contains("INSERT INTO \"my_table\" (\"name\") VALUES ($1::text)"));
    }

    #[test]
    fn test_insert_always_identity_column() {
        let mut id_col = col("id", "int4");
        id_col.isidentity = ColumnIdentity::Always;
        id_col.hide_insert = Some(true);

        let name_col = col("name", "text");
        let result =
            make_insert_query("my_table", &[id_col, name_col], DbType::Postgresql).unwrap();

        // Always identity should be omitted
        assert!(result.contains("INSERT INTO \"my_table\" (\"name\") VALUES ($1::text)"));
    }

    #[test]
    fn test_insert_error_not_nullable_no_default() {
        let mut id_col = col("id", "int4");
        id_col.hide_insert = Some(true);
        id_col.isnullable = "NO".to_string();

        let result = make_insert_query("my_table", &[id_col], DbType::Postgresql);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not nullable"));
    }

    #[test]
    fn test_insert_empty_table_error() {
        let cols = vec![col("id", "int4")];
        let result = make_insert_query("", &cols, DbType::Postgresql);
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_default_values_only() {
        // All columns hidden, none needs defaults -> DEFAULT VALUES
        let result = make_insert_query("my_table", &[], DbType::Postgresql).unwrap();
        assert_eq!(result, "INSERT INTO \"my_table\" DEFAULT VALUES");
    }

    // -----------------------------------------------------------------------
    // UPDATE - all DB types
    // -----------------------------------------------------------------------

    #[test]
    fn test_update_postgresql() {
        let update_col = simple_col("name", "text");
        let where_cols = vec![simple_col("id", "int4"), simple_col("email", "text")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::Postgresql);

        assert!(result.contains("-- $1 value_to_update\n-- $2 id\n-- $3 email"));
        assert!(result.contains("UPDATE \"my_table\" SET \"name\" = $1::text::text"));
        assert!(result
            .contains("($2::text::int4 IS NULL AND \"id\" IS NULL OR \"id\" = $2::text::int4)"));
        assert!(result.contains(
            "($3::text::text IS NULL AND \"email\" IS NULL OR \"email\" = $3::text::text)"
        ));
        assert!(result.contains("RETURNING 1"));
    }

    #[test]
    fn test_update_mysql() {
        let update_col = simple_col("name", "varchar");
        let where_cols = vec![simple_col("id", "int")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::Mysql);

        assert!(result.contains("UPDATE `my_table` SET `name` = :value_to_update"));
        assert!(result.contains("(:id IS NULL AND `id` IS NULL OR `id` = :id)"));
    }

    #[test]
    fn test_update_mssql() {
        let update_col = simple_col("name", "nvarchar");
        let where_cols = vec![simple_col("id", "int")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::MsSqlServer);

        assert!(result.contains("UPDATE [my_table] SET [name] = @p1"));
        assert!(result.contains("(@p2 IS NULL AND [id] IS NULL OR [id] = @p2)"));
    }

    #[test]
    fn test_update_snowflake() {
        let update_col = simple_col("name", "text");
        let where_cols = vec![simple_col("id", "int")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::Snowflake);

        // Snowflake doubles where columns
        assert!(result.contains("-- ? value_to_update (text)\n-- ? id (int)\n-- ? id (int)"));
        assert!(result.contains("UPDATE \"my_table\" SET \"name\" = ?"));
        assert!(result.contains("(? = 'null' AND \"id\" IS NULL OR \"id\" = ?)"));
    }

    #[test]
    fn test_update_bigquery() {
        let update_col = simple_col("name", "STRING");
        let where_cols = vec![simple_col("id", "INTEGER")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::Bigquery);

        assert!(result.contains("UPDATE `my_table` SET `name` = @value_to_update"));
        assert!(result.contains("(CAST(@id AS STRING) = 'null' AND `id` IS NULL OR `id` = @id)"));
    }

    #[test]
    fn test_update_duckdb() {
        let update_col = simple_col("name", "text");
        let where_cols = vec![simple_col("id", "int")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::Duckdb);

        assert!(result.contains("UPDATE \"my_table\" SET \"name\" = $value_to_update"));
        assert!(result.contains("($id IS NULL AND \"id\" IS NULL OR \"id\" = $id)"));
    }

    // -----------------------------------------------------------------------
    // Integration-style tests: exact output matching for key cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_count_mssql_no_where() {
        let cols = vec![col("id", "int")];
        let result = make_count_query(DbType::MsSqlServer, "my_table", None, &cols).unwrap();
        // MSSQL uses WHERE directly (no AND replacement)
        assert!(result.contains("SELECT COUNT(*) as count FROM [my_table] WHERE "));
    }

    #[test]
    fn test_count_mysql_no_where_uses_where_keyword() {
        let cols = vec![col("id", "int")];
        let result = make_count_query(DbType::Mysql, "my_table", None, &cols).unwrap();
        // The AND should be replaced with WHERE
        assert!(result.contains("FROM `my_table` WHERE "));
        assert!(!result.contains("FROM `my_table` AND "));
    }

    #[test]
    fn test_delete_multiple_conditions_joined_with_and() {
        let cols = vec![col("a", "int4"), col("b", "text"), col("c", "bool")];
        let result = make_delete_query("t", &cols, DbType::Postgresql);
        // Check all three conditions are present and joined
        assert!(result
            .contains("AND ($2::text::text IS NULL AND \"b\" IS NULL OR \"b\" = $2::text::text)"));
        assert!(result
            .contains("AND ($3::text::bool IS NULL AND \"c\" IS NULL OR \"c\" = $3::text::bool)"));
    }

    #[test]
    fn test_select_with_ignored_column() {
        let cols = vec![
            col("id", "int4"),
            {
                let mut c = col("hidden", "text");
                c.ignored = Some(true);
                c
            },
            col("name", "text"),
        ];
        let result =
            make_select_query("my_table", &cols, None, DbType::Postgresql, None, None).unwrap();

        // hidden column should not appear in SELECT clause but does appear in ORDER BY
        assert!(result.contains("SELECT \"id\"::text, \"name\"::text FROM"));
        assert!(!result.contains("SELECT \"id\"::text, \"hidden\"::text"));
        // It SHOULD appear in ORDER BY (order by uses all columnDefs, not just visible)
        assert!(result.contains("$4 = 'hidden'"));
    }

    #[test]
    fn test_insert_with_override_default_value() {
        let mut created_col = col("created_at", "timestamp");
        created_col.hide_insert = Some(true);
        created_col.override_default_value = Some(true);
        created_col.default_user_value = Some(serde_json::Value::String("now()".to_string()));
        // defaultvalue from DB is different
        created_col.defaultvalue = "CURRENT_TIMESTAMP".to_string();

        let name_col = col("name", "text");
        let result =
            make_insert_query("my_table", &[name_col, created_col], DbType::Postgresql).unwrap();

        // Should use the user override value, not the DB default
        assert!(result.contains("'now()'"));
        assert!(!result.contains("CURRENT_TIMESTAMP"));
    }

    #[test]
    fn test_insert_with_numeric_default_user_value() {
        let mut col1 = col("priority", "int4");
        col1.hide_insert = Some(true);
        col1.default_user_value = Some(serde_json::json!(5));

        let name_col = col("name", "text");
        let result = make_insert_query("my_table", &[name_col, col1], DbType::Postgresql).unwrap();

        // Numeric value should not be quoted
        assert!(result.contains("VALUES ($1::text, 5)"));
    }

    // -----------------------------------------------------------------------
    // WM_INTERNAL_DB expansion
    // -----------------------------------------------------------------------

    /// Helper: expand a marker and return just the SQL code string.
    fn expand_code(marker: &str, lang: &ScriptLang) -> String {
        try_expand_internal_db_query(marker, lang)
            .expect("should be recognized as a marker")
            .expect("expansion should succeed")
            .code
    }

    #[test]
    fn test_expand_select_marker() {
        let marker = r#"-- WM_INTERNAL_DB_SELECT {"table":"my_table","columnDefs":[{"field":"id","datatype":"int4"},{"field":"name","datatype":"text"}]}"#;
        let result = try_expand_internal_db_query(marker, &ScriptLang::Postgresql);
        assert!(result.is_some());
        let sql = result.unwrap().unwrap().code;
        assert!(sql.contains("SELECT \"id\"::text, \"name\"::text FROM \"my_table\""));
        assert!(sql.contains("LIMIT $1::INT OFFSET $2::INT"));
    }

    #[test]
    fn test_expand_select_with_where_and_options() {
        let marker = r#"-- WM_INTERNAL_DB_SELECT {"table":"t","columnDefs":[{"field":"id","datatype":"int4"}],"whereClause":"active = true","limit":50,"offset":10,"fixPgIntTypes":true}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("WHERE active = true AND"));
        assert!(sql.contains("pg_typeof(\"id\")"));
    }

    #[test]
    fn test_expand_count_marker() {
        let marker = r#"-- WM_INTERNAL_DB_COUNT {"table":"my_table","columnDefs":[{"field":"id","datatype":"int4"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("SELECT COUNT(*) as count FROM `my_table`"));
        assert!(sql.contains(":quicksearch"));
    }

    #[test]
    fn test_expand_delete_marker() {
        let marker = r#"-- WM_INTERNAL_DB_DELETE {"table":"my_table","columns":[{"field":"id","datatype":"int4"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("DELETE FROM \"my_table\""));
        assert!(sql.contains("RETURNING 1;"));
    }

    #[test]
    fn test_expand_insert_marker() {
        let marker = r#"-- WM_INTERNAL_DB_INSERT {"table":"my_table","columns":[{"field":"id","datatype":"int4"},{"field":"name","datatype":"text"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(
            sql.contains("INSERT INTO \"my_table\" (\"id\", \"name\") VALUES ($1::int4, $2::text)")
        );
    }

    #[test]
    fn test_expand_update_marker() {
        let marker = r#"-- WM_INTERNAL_DB_UPDATE {"table":"my_table","column":{"field":"name","datatype":"text"},"columns":[{"field":"id","datatype":"int4"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("UPDATE \"my_table\" SET \"name\" = $1::text::text"));
        assert!(sql.contains("RETURNING 1"));
    }

    #[test]
    fn test_expand_with_ducklake() {
        let marker = r#"-- WM_INTERNAL_DB_SELECT {"table":"my_table","columnDefs":[{"field":"id","datatype":"int4"}],"ducklake":"my_lake"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.contains("ATTACH 'ducklake://my_lake' AS dl;USE dl;"));
        assert!(sql.contains("SELECT \"id\" FROM \"my_table\""));
    }

    #[test]
    fn test_expand_not_a_marker() {
        let regular_sql = "SELECT * FROM my_table";
        let result = try_expand_internal_db_query(regular_sql, &ScriptLang::Postgresql);
        assert!(result.is_none());
    }

    #[test]
    fn test_expand_invalid_json() {
        let marker = "-- WM_INTERNAL_DB_SELECT {invalid json}";
        let result = try_expand_internal_db_query(marker, &ScriptLang::Postgresql);
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_expand_unknown_operation() {
        let marker = "-- WM_INTERNAL_DB_TRUNCATE {}";
        let result = try_expand_internal_db_query(marker, &ScriptLang::Postgresql);
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_expand_mysql_select() {
        let marker = r#"-- WM_INTERNAL_DB_SELECT {"table":"users","columnDefs":[{"field":"id","datatype":"int"},{"field":"name","datatype":"varchar"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("SELECT `id`, `name` FROM `users`"));
        assert!(sql.contains("LIMIT :limit OFFSET :offset"));
    }

    #[test]
    fn test_column_def_serde_roundtrip() {
        let json = r#"{"field":"id","datatype":"int4","hideInsert":true,"defaultValueNull":true,"isidentity":"No"}"#;
        let col: ColumnDef = serde_json::from_str(json).unwrap();
        assert_eq!(col.field, "id");
        assert_eq!(col.hide_insert, Some(true));
        assert_eq!(col.default_value_null, Some(true));
        assert_eq!(col.isidentity, ColumnIdentity::No);
    }

    #[test]
    fn test_column_def_with_null_fields() {
        let json = r#"{"field":"id","datatype":"int4","isprimarykey":null,"isenum":null,"isidentity":null,"isnullable":null,"defaultvalue":null}"#;
        let col: ColumnDef = serde_json::from_str(json).unwrap();
        assert_eq!(col.field, "id");
        assert!(!col.isprimarykey);
        assert!(!col.isenum);
        assert_eq!(col.isidentity, ColumnIdentity::No);
        assert_eq!(col.isnullable, "");
        assert_eq!(col.defaultvalue, "");
    }

    #[test]
    fn test_wrap_ducklake_query() {
        let query = "-- $1 limit\n-- $2 offset\nSELECT * FROM t";
        let wrapped = wrap_ducklake_query(query, "my_lake");
        assert_eq!(
            wrapped,
            "-- $1 limit\n-- $2 offset\nATTACH 'ducklake://my_lake' AS dl;USE dl;\nSELECT * FROM t"
        );
    }

    // -----------------------------------------------------------------------
    // Schema DDL helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_table_ref_with_schema() {
        assert_eq!(
            table_ref("users", Some("myschema"), DbType::Postgresql),
            "myschema.users"
        );
        assert_eq!(
            table_ref("users", Some("myschema"), DbType::Snowflake),
            "myschema.users"
        );
        assert_eq!(
            table_ref("users", Some("dataset"), DbType::Bigquery),
            "dataset.users"
        );
    }

    #[test]
    fn test_table_ref_without_schema() {
        assert_eq!(table_ref("users", None, DbType::Postgresql), "users");
        assert_eq!(table_ref("users", Some(""), DbType::Postgresql), "users");
        // Non-schema DBs ignore schema
        assert_eq!(table_ref("users", Some("myschema"), DbType::Mysql), "users");
        assert_eq!(
            table_ref("users", Some("myschema"), DbType::Duckdb),
            "users"
        );
    }

    #[test]
    fn test_datatype_has_length() {
        assert!(datatype_has_length("varchar"));
        assert!(datatype_has_length("VARCHAR"));
        assert!(datatype_has_length("nvarchar"));
        assert!(datatype_has_length("char"));
        assert!(datatype_has_length("bit"));
        assert!(datatype_has_length("varbinary"));
        assert!(!datatype_has_length("int"));
        assert!(!datatype_has_length("text"));
        assert!(!datatype_has_length("boolean"));
    }

    #[test]
    fn test_format_default_value_simple() {
        assert_eq!(
            format_default_value("hello", "text", DbType::Mysql),
            "'hello'"
        );
        assert_eq!(
            format_default_value("hello", "text", DbType::Postgresql),
            "CAST('hello' AS text)"
        );
    }

    #[test]
    fn test_format_default_value_expression() {
        // Expressions wrapped in braces are returned bare
        assert_eq!(
            format_default_value("{now()}", "timestamp", DbType::Postgresql),
            "now()"
        );
        assert_eq!(
            format_default_value("{CURRENT_TIMESTAMP}", "timestamp", DbType::Mysql),
            "CURRENT_TIMESTAMP"
        );
    }

    #[test]
    fn test_format_default_value_empty() {
        assert_eq!(format_default_value("", "text", DbType::Postgresql), "");
    }

    #[test]
    fn test_render_column_ddl_basic() {
        let c = TableEditorColumn {
            name: "email".to_string(),
            datatype: "VARCHAR".to_string(),
            primary_key: None,
            default_value: None,
            nullable: Some(false),
            datatype_length: Some(255),
            initial_name: None,
            default_constraint_name: None,
        };
        let result = render_column_ddl(&c, DbType::Postgresql, false);
        assert_eq!(result, "\"email\" VARCHAR(255) NOT NULL");
    }

    #[test]
    fn test_render_column_ddl_with_default_and_pk() {
        let c = TableEditorColumn {
            name: "id".to_string(),
            datatype: "INT".to_string(),
            primary_key: Some(true),
            default_value: Some("{nextval('id_seq')}".to_string()),
            nullable: Some(false),
            datatype_length: None,
            initial_name: None,
            default_constraint_name: None,
        };
        let result = render_column_ddl(&c, DbType::Postgresql, true);
        assert_eq!(
            result,
            "\"id\" INT NOT NULL DEFAULT nextval('id_seq') PRIMARY KEY"
        );
    }

    #[test]
    fn test_render_column_ddl_nullable_no_length() {
        let c = TableEditorColumn {
            name: "notes".to_string(),
            datatype: "TEXT".to_string(),
            primary_key: None,
            default_value: None,
            nullable: Some(true),
            datatype_length: None,
            initial_name: None,
            default_constraint_name: None,
        };
        // nullable=true means no NOT NULL appended
        let result = render_column_ddl(&c, DbType::Postgresql, false);
        assert_eq!(result, "\"notes\" TEXT");
    }

    // -----------------------------------------------------------------------
    // DROP TABLE
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_drop_table_simple() {
        let marker = r#"-- WM_INTERNAL_DB_DROP_TABLE {"table":"users"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert_eq!(sql, "DROP TABLE \"users\";");
    }

    #[test]
    fn test_expand_drop_table_with_schema() {
        let marker = r#"-- WM_INTERNAL_DB_DROP_TABLE {"table":"users","schema":"myschema"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert_eq!(sql, "DROP TABLE \"myschema\".\"users\";");
    }

    #[test]
    fn test_expand_drop_table_mysql_ignores_schema() {
        let marker = r#"-- WM_INTERNAL_DB_DROP_TABLE {"table":"users","schema":"myschema"}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert_eq!(sql, "DROP TABLE `users`;");
    }

    #[test]
    fn test_expand_drop_table_with_ducklake() {
        let marker = r#"-- WM_INTERNAL_DB_DROP_TABLE {"table":"users","ducklake":"my_lake"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.contains("ATTACH 'ducklake://my_lake' AS dl;USE dl;"));
        assert!(sql.contains("DROP TABLE \"users\";"));
    }

    // -----------------------------------------------------------------------
    // CREATE SCHEMA / DROP SCHEMA
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_create_schema() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_SCHEMA {"schema":"new_schema"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert_eq!(sql, "CREATE SCHEMA \"new_schema\";");
    }

    #[test]
    fn test_expand_drop_schema() {
        let marker = r#"-- WM_INTERNAL_DB_DROP_SCHEMA {"schema":"old_schema"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert_eq!(sql, "DROP SCHEMA \"old_schema\" CASCADE;");
    }

    #[test]
    fn test_expand_create_schema_with_ducklake() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_SCHEMA {"schema":"s","ducklake":"lake"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.starts_with("ATTACH 'ducklake://lake' AS dl;USE dl;\n"));
        assert!(sql.contains("CREATE SCHEMA \"s\";"));
    }

    // -----------------------------------------------------------------------
    // CREATE TABLE
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_create_table_basic() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_TABLE {"name":"users","columns":[{"name":"id","datatype":"INT","primaryKey":true,"nullable":false},{"name":"name","datatype":"TEXT","nullable":true}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("CREATE TABLE \"users\" ("));
        assert!(sql.contains("\"id\" INT NOT NULL PRIMARY KEY"));
        assert!(sql.contains("\"name\" TEXT"));
        assert!(sql.ends_with(");"));
    }

    #[test]
    fn test_expand_create_table_with_schema() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_TABLE {"name":"users","columns":[{"name":"id","datatype":"INT","primaryKey":true}],"schema":"myschema"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("CREATE TABLE \"myschema\".\"users\" ("));
    }

    #[test]
    fn test_expand_create_table_composite_pk() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_TABLE {"name":"t","columns":[{"name":"a","datatype":"INT","primaryKey":true},{"name":"b","datatype":"INT","primaryKey":true}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        // Composite PK should be separate constraint, not inline
        assert!(!sql.contains("\"a\" INT PRIMARY KEY"));
        assert!(sql.contains("PRIMARY KEY (\"a\", \"b\")"));
    }

    #[test]
    fn test_expand_create_table_bigquery_pk_not_enforced() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_TABLE {"name":"t","columns":[{"name":"id","datatype":"INT64","primaryKey":true}]}"#;
        let sql = expand_code(marker, &ScriptLang::Bigquery);
        assert!(sql.contains("`id` INT64 NOT NULL PRIMARY KEY NOT ENFORCED"));
    }

    #[test]
    fn test_expand_create_table_with_foreign_key() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_TABLE {"name":"orders","columns":[{"name":"id","datatype":"INT","primaryKey":true},{"name":"user_id","datatype":"INT"}],"foreignKeys":[{"targetTable":"users","columns":[{"sourceColumn":"user_id","targetColumn":"id"}],"onDelete":"CASCADE","onUpdate":"NO ACTION"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("FOREIGN KEY (\"user_id\") REFERENCES \"users\" (\"id\")"));
        assert!(sql.contains("ON DELETE CASCADE"));
        assert!(!sql.contains("ON UPDATE NO ACTION")); // NO ACTION is omitted
    }

    #[test]
    fn test_expand_create_table_with_default_value() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_TABLE {"name":"t","columns":[{"name":"status","datatype":"TEXT","defaultValue":"active","nullable":false}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("\"status\" TEXT NOT NULL DEFAULT CAST('active' AS TEXT)"));
    }

    #[test]
    fn test_expand_create_table_varchar_with_length() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_TABLE {"name":"t","columns":[{"name":"email","datatype":"VARCHAR","datatype_length":100}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("`email` VARCHAR(100)"));
    }

    #[test]
    fn test_expand_create_table_snowflake_default_schema() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_TABLE {"name":"t","columns":[{"name":"id","datatype":"INT"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Snowflake);
        // Snowflake defaults to PUBLIC schema
        assert!(sql.contains("CREATE TABLE \"PUBLIC\".\"t\" ("));
    }

    #[test]
    fn test_expand_create_table_with_ducklake() {
        let marker = r#"-- WM_INTERNAL_DB_CREATE_TABLE {"name":"t","columns":[{"name":"id","datatype":"INT"}],"ducklake":"lake"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.starts_with("ATTACH 'ducklake://lake' AS dl;USE dl;\n"));
        assert!(sql.contains("CREATE TABLE \"t\" ("));
    }

    // -----------------------------------------------------------------------
    // ALTER TABLE
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_alter_table_add_column() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"users","operations":[{"kind":"addColumn","column":{"name":"age","datatype":"INT","nullable":true}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("BEGIN;"));
        assert!(sql.contains("ALTER TABLE \"users\" ADD COLUMN \"age\" INT;"));
        assert!(sql.contains("COMMIT;"));
    }

    #[test]
    fn test_expand_alter_table_drop_column() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"users","operations":[{"kind":"dropColumn","name":"old_col"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"users\" DROP COLUMN \"old_col\";"));
    }

    #[test]
    fn test_expand_alter_table_rename_table() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"old_name","operations":[{"kind":"renameTable","to":"new_name"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"old_name\" RENAME TO \"new_name\";"));
    }

    #[test]
    fn test_expand_alter_table_rename_table_snowflake_with_schema() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"old","operations":[{"kind":"renameTable","to":"new"}],"schema":"MY_SCHEMA"}"#;
        let sql = expand_code(marker, &ScriptLang::Snowflake);
        assert!(sql.contains("ALTER TABLE \"MY_SCHEMA\".\"old\" RENAME TO \"MY_SCHEMA\".\"new\";"));
    }

    #[test]
    fn test_expand_alter_table_add_foreign_key() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"orders","operations":[{"kind":"addForeignKey","foreignKey":{"targetTable":"users","columns":[{"sourceColumn":"user_id","targetColumn":"id"}],"onDelete":"CASCADE","onUpdate":"NO ACTION"}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"orders\" ADD CONSTRAINT fk_"));
        assert!(sql.contains("FOREIGN KEY (\"user_id\") REFERENCES \"users\" (\"id\")"));
        assert!(sql.contains("ON DELETE CASCADE"));
    }

    #[test]
    fn test_expand_alter_table_drop_foreign_key_mysql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"orders","operations":[{"kind":"dropForeignKey","fk_constraint_name":"fk_orders_user"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        // MySQL uses DROP FOREIGN KEY
        assert!(sql.contains("ALTER TABLE `orders` DROP FOREIGN KEY `fk_orders_user`"));
    }

    #[test]
    fn test_expand_alter_table_drop_foreign_key_postgresql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"orders","operations":[{"kind":"dropForeignKey","fk_constraint_name":"fk_orders_user"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        // PostgreSQL uses DROP CONSTRAINT
        assert!(sql.contains("ALTER TABLE \"orders\" DROP CONSTRAINT \"fk_orders_user\""));
    }

    #[test]
    fn test_expand_alter_table_add_primary_key() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"addPrimaryKey","columns":["a","b"]}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"t\" ADD PRIMARY KEY (\"a\", \"b\");"));
    }

    #[test]
    fn test_expand_alter_table_add_primary_key_snowflake() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"addPrimaryKey","columns":["id"]}]}"#;
        let sql = expand_code(marker, &ScriptLang::Snowflake);
        // Snowflake uses named constraint
        assert!(sql.contains("ADD CONSTRAINT \"t_pk\" PRIMARY KEY (\"id\")"));
    }

    #[test]
    fn test_expand_alter_table_add_primary_key_bigquery() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"addPrimaryKey","columns":["id"]}]}"#;
        let sql = expand_code(marker, &ScriptLang::Bigquery);
        assert!(sql.contains("ADD PRIMARY KEY (`id`) NOT ENFORCED;"));
    }

    #[test]
    fn test_expand_alter_table_drop_primary_key_mysql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"dropPrimaryKey"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("ALTER TABLE `t` DROP PRIMARY KEY;"));
    }

    #[test]
    fn test_expand_alter_table_drop_primary_key_with_constraint() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"dropPrimaryKey","pk_constraint_name":"pk_t"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"t\" DROP CONSTRAINT \"pk_t\""));
    }

    #[test]
    fn test_expand_alter_table_alter_column_datatype() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"INT"},"changes":{"datatype":"BIGINT"}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"t\" ALTER COLUMN \"col\" TYPE BIGINT;"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_datatype_mysql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"INT"},"changes":{"datatype":"BIGINT"}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("ALTER TABLE `t` MODIFY COLUMN `col` BIGINT;"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_datatype_mssql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"INT"},"changes":{"datatype":"BIGINT"}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mssql);
        assert!(sql.contains("ALTER TABLE [t] ALTER COLUMN [col] BIGINT;"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_nullable() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"INT"},"changes":{"nullable":false}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"t\" ALTER COLUMN \"col\" SET NOT NULL;"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_drop_not_null() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"INT"},"changes":{"nullable":true}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"t\" ALTER COLUMN \"col\" DROP NOT NULL;"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_nullable_mssql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"INT"},"changes":{"nullable":false}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mssql);
        assert!(sql.contains("ALTER TABLE [t] ALTER COLUMN [col] INT NOT NULL;"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_nullable_mysql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"INT"},"changes":{"nullable":true}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("ALTER TABLE `t` MODIFY COLUMN `col` INT NULL;"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_rename() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"old_col","datatype":"INT"},"changes":{"name":"new_col"}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"t\" RENAME COLUMN \"old_col\" TO \"new_col\";"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_rename_mssql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"old_col","datatype":"INT"},"changes":{"name":"new_col"}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mssql);
        assert!(sql.contains("EXEC sp_rename '[t].old_col', 'new_col', 'COLUMN';"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_set_default() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"TEXT"},"changes":{"defaultValue":"hello"}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql
            .contains("ALTER TABLE \"t\" ALTER COLUMN \"col\" SET DEFAULT CAST('hello' AS TEXT);"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_drop_default() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"TEXT","defaultValue":"old"},"changes":{"defaultValue":null}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"t\" ALTER COLUMN \"col\" DROP DEFAULT;"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_drop_default_mssql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"INT","defaultValue":"0"},"defaultConstraintName":"DF_t_col","changes":{"defaultValue":null}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mssql);
        assert!(sql.contains("ALTER TABLE [t] DROP CONSTRAINT [DF_t_col]"));
    }

    #[test]
    fn test_expand_alter_table_alter_column_varchar_with_length() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"alterColumn","original":{"name":"col","datatype":"VARCHAR","datatype_length":50},"changes":{"datatype":"VARCHAR","datatype_length":100}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"t\" ALTER COLUMN \"col\" TYPE VARCHAR(100);"));
    }

    #[test]
    fn test_expand_alter_table_transactional_postgresql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"addColumn","column":{"name":"a","datatype":"INT"}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.starts_with("BEGIN;\n"));
        assert!(sql.ends_with("\nCOMMIT;"));
    }

    #[test]
    fn test_expand_alter_table_transactional_mssql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"addColumn","column":{"name":"a","datatype":"INT"}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mssql);
        assert!(sql.starts_with("BEGIN TRANSACTION;\n"));
        assert!(sql.ends_with("\nCOMMIT TRANSACTION;"));
    }

    #[test]
    fn test_expand_alter_table_no_transaction_mysql() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"addColumn","column":{"name":"a","datatype":"INT"}}]}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(!sql.contains("BEGIN"));
        assert!(!sql.contains("COMMIT"));
        assert!(sql.contains("ALTER TABLE `t` ADD COLUMN `a` INT;"));
    }

    #[test]
    fn test_expand_alter_table_with_schema() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"addColumn","column":{"name":"a","datatype":"INT"}}],"schema":"myschema"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ALTER TABLE \"myschema\".\"t\" ADD COLUMN \"a\" INT;"));
    }

    #[test]
    fn test_expand_alter_table_multiple_operations() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"addColumn","column":{"name":"a","datatype":"INT"}},{"kind":"dropColumn","name":"b"}]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("ADD COLUMN \"a\" INT;"));
        assert!(sql.contains("DROP COLUMN \"b\";"));
    }

    #[test]
    fn test_expand_alter_table_empty_operations() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[]}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert_eq!(sql, "");
    }

    #[test]
    fn test_expand_alter_table_with_ducklake() {
        let marker = r#"-- WM_INTERNAL_DB_ALTER_TABLE {"name":"t","operations":[{"kind":"addColumn","column":{"name":"a","datatype":"INT"}}],"ducklake":"lake"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.starts_with("ATTACH 'ducklake://lake' AS dl;USE dl;\n"));
        assert!(sql.contains("ALTER TABLE \"t\" ADD COLUMN \"a\" INT;"));
    }

    // -----------------------------------------------------------------------
    // LOAD_TABLE_METADATA
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_load_table_metadata_postgresql_single_table() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {"table":"users"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("a.attname as field"));
        assert!(sql.contains("pg_catalog.format_type"));
        assert!(sql.contains("relname = 'users'"));
        assert!(sql.contains("ns.nspname = 'public'"));
        // Single table should not have table_name/schema_name columns
        assert!(!sql.contains("table_name"));
    }

    #[test]
    fn test_expand_load_table_metadata_postgresql_all_tables() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("schema_name"));
        assert!(sql.contains("table_name"));
        assert!(sql.contains("c.relkind = 'r'"));
    }

    #[test]
    fn test_expand_load_table_metadata_postgresql_schema_table() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {"table":"myschema.users"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("relname = 'users'"));
        assert!(sql.contains("ns.nspname = 'myschema'"));
    }

    #[test]
    fn test_expand_load_table_metadata_mysql_single_table() {
        let marker =
            r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {"table":"users","databaseName":"mydb"}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("COLUMN_NAME as field"));
        assert!(sql.contains("TABLE_NAME = 'users'"));
        assert!(sql.contains("TABLE_SCHEMA = 'mydb'"));
    }

    #[test]
    fn test_expand_load_table_metadata_mysql_all_tables() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("TABLE_NAME as table_name"));
        assert!(sql.contains("TABLE_SCHEMA NOT IN"));
    }

    #[test]
    fn test_expand_load_table_metadata_mssql_single_table() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {"table":"users"}"#;
        let sql = expand_code(marker, &ScriptLang::Mssql);
        assert!(sql.contains("c.COLUMN_NAME as field"));
        assert!(sql.contains("c.TABLE_NAME = 'users'"));
        assert!(sql.contains("default_constraint_name"));
    }

    #[test]
    fn test_expand_load_table_metadata_mssql_all_tables() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {}"#;
        let sql = expand_code(marker, &ScriptLang::Mssql);
        assert!(sql.contains("TABLE_NAME as table_name"));
        assert!(!sql.contains("WHERE"));
    }

    #[test]
    fn test_expand_load_table_metadata_snowflake_single_table() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {"table":"MY_TABLE"}"#;
        let sql = expand_code(marker, &ScriptLang::Snowflake);
        assert!(sql.contains("COLUMN_NAME as field"));
        assert!(sql.contains("table_name = 'MY_TABLE'"));
        assert!(sql.contains("table_schema = 'PUBLIC'"));
    }

    #[test]
    fn test_expand_load_table_metadata_snowflake_all_tables() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {}"#;
        let sql = expand_code(marker, &ScriptLang::Snowflake);
        assert!(sql.contains("table_name as table_name"));
        assert!(sql.contains("table_schema as schema_name"));
        assert!(sql.contains("table_schema <> 'INFORMATION_SCHEMA'"));
    }

    #[test]
    fn test_expand_load_table_metadata_bigquery_single_table() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {"table":"dataset.my_table"}"#;
        let sql = expand_code(marker, &ScriptLang::Bigquery);
        assert!(sql.contains("dataset.INFORMATION_SCHEMA.COLUMNS"));
        assert!(sql.contains("TABLE_NAME = 'my_table'"));
    }

    #[test]
    fn test_expand_load_table_metadata_bigquery_all_tables_bun_script() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {}"#;
        let expanded = try_expand_internal_db_query(marker, &ScriptLang::Bigquery)
            .expect("should be recognized as a marker")
            .expect("expansion should succeed");
        // BigQuery all-tables returns a Bun script, not SQL
        assert_eq!(expanded.language_override, Some(ScriptLang::Bun));
        assert!(expanded.code.contains("BigQuery"));
        assert!(expanded.code.contains("INFORMATION_SCHEMA.COLUMNS"));
    }

    #[test]
    fn test_expand_load_table_metadata_duckdb() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {"table":"users"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.contains("COLUMN_NAME as field"));
        assert!(sql.contains("TABLE_NAME = 'users'"));
    }

    #[test]
    fn test_expand_load_table_metadata_with_ducklake() {
        let marker = r#"-- WM_INTERNAL_DB_LOAD_TABLE_METADATA {"table":"users","ducklake":"lake"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.starts_with("ATTACH 'ducklake://lake' AS dl;USE dl;\n"));
        assert!(sql.contains("TABLE_NAME = 'users'"));
    }

    // -----------------------------------------------------------------------
    // FOREIGN_KEYS
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_foreign_keys_postgresql() {
        let marker = r#"-- WM_INTERNAL_DB_FOREIGN_KEYS {"table":"users"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("tc.constraint_name as fk_constraint_name"));
        assert!(sql.contains("tc.table_name = 'users'"));
        assert!(sql.contains("tc.table_schema = 'public'"));
    }

    #[test]
    fn test_expand_foreign_keys_with_schema() {
        let marker = r#"-- WM_INTERNAL_DB_FOREIGN_KEYS {"table":"users","schema":"myschema"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("tc.table_schema = 'myschema'"));
    }

    #[test]
    fn test_expand_foreign_keys_schema_in_table_name() {
        let marker = r#"-- WM_INTERNAL_DB_FOREIGN_KEYS {"table":"myschema.users"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("tc.table_name = 'users'"));
        assert!(sql.contains("tc.table_schema = 'myschema'"));
    }

    #[test]
    fn test_expand_foreign_keys_mysql() {
        let marker = r#"-- WM_INTERNAL_DB_FOREIGN_KEYS {"table":"orders"}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("kcu.TABLE_NAME = 'orders'"));
        assert!(sql.contains("TABLE_SCHEMA = DATABASE()"));
    }

    #[test]
    fn test_expand_foreign_keys_mysql_with_schema() {
        let marker = r#"-- WM_INTERNAL_DB_FOREIGN_KEYS {"table":"orders","schema":"mydb"}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("TABLE_SCHEMA = 'mydb'"));
    }

    #[test]
    fn test_expand_foreign_keys_mssql() {
        let marker = r#"-- WM_INTERNAL_DB_FOREIGN_KEYS {"table":"orders"}"#;
        let sql = expand_code(marker, &ScriptLang::Mssql);
        assert!(sql.contains("t.name = 'orders'"));
        assert!(sql.contains("s.name = 'dbo'"));
    }

    #[test]
    fn test_expand_foreign_keys_snowflake() {
        let marker = r#"-- WM_INTERNAL_DB_FOREIGN_KEYS {"table":"orders","schema":"MY_SCHEMA"}"#;
        let sql = expand_code(marker, &ScriptLang::Snowflake);
        assert!(sql.contains("SHOW IMPORTED KEYS IN TABLE MY_SCHEMA.orders"));
    }

    #[test]
    fn test_expand_foreign_keys_bigquery() {
        let marker = r#"-- WM_INTERNAL_DB_FOREIGN_KEYS {"table":"dataset.orders"}"#;
        let sql = expand_code(marker, &ScriptLang::Bigquery);
        assert!(sql.contains("INFORMATION_SCHEMA.TABLE_CONSTRAINTS"));
        assert!(sql.contains("tc.table_name = 'orders'"));
    }

    #[test]
    fn test_expand_foreign_keys_duckdb() {
        let marker = r#"-- WM_INTERNAL_DB_FOREIGN_KEYS {"table":"orders"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.contains("fk_constraint.table_name = 'orders'"));
        assert!(sql.contains("fk_constraint.table_schema = 'main'"));
    }

    #[test]
    fn test_expand_foreign_keys_with_ducklake() {
        let marker = r#"-- WM_INTERNAL_DB_FOREIGN_KEYS {"table":"orders","ducklake":"lake"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.starts_with("ATTACH 'ducklake://lake' AS dl;USE dl;\n"));
    }

    // -----------------------------------------------------------------------
    // PRIMARY_KEY_CONSTRAINT
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_primary_key_constraint_postgresql() {
        let marker = r#"-- WM_INTERNAL_DB_PRIMARY_KEY_CONSTRAINT {"table":"users"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("constraint_type = 'PRIMARY KEY'"));
        assert!(sql.contains("tc.table_name = 'users'"));
        assert!(sql.contains("tc.table_schema = 'public'"));
    }

    #[test]
    fn test_expand_primary_key_constraint_with_schema() {
        let marker = r#"-- WM_INTERNAL_DB_PRIMARY_KEY_CONSTRAINT {"table":"myschema.users"}"#;
        let sql = expand_code(marker, &ScriptLang::Postgresql);
        assert!(sql.contains("tc.table_name = 'users'"));
        assert!(sql.contains("tc.table_schema = 'myschema'"));
    }

    #[test]
    fn test_expand_primary_key_constraint_mysql() {
        let marker = r#"-- WM_INTERNAL_DB_PRIMARY_KEY_CONSTRAINT {"table":"users"}"#;
        let sql = expand_code(marker, &ScriptLang::Mysql);
        assert!(sql.contains("CONSTRAINT_TYPE = 'PRIMARY KEY'"));
        assert!(sql.contains("TABLE_NAME = 'users'"));
        assert!(sql.contains("TABLE_SCHEMA = DATABASE()"));
    }

    #[test]
    fn test_expand_primary_key_constraint_mssql() {
        let marker = r#"-- WM_INTERNAL_DB_PRIMARY_KEY_CONSTRAINT {"table":"users"}"#;
        let sql = expand_code(marker, &ScriptLang::Mssql);
        assert!(sql.contains("kc.type = 'PK'"));
        assert!(sql.contains("t.name = 'users'"));
        assert!(sql.contains("s.name = 'dbo'"));
    }

    #[test]
    fn test_expand_primary_key_constraint_snowflake() {
        let marker =
            r#"-- WM_INTERNAL_DB_PRIMARY_KEY_CONSTRAINT {"table":"users","schema":"MY_SCHEMA"}"#;
        let sql = expand_code(marker, &ScriptLang::Snowflake);
        assert!(sql.contains("constraint_type = 'PRIMARY KEY'"));
        assert!(sql.contains("table_name = 'users'"));
        assert!(sql.contains("table_schema = 'MY_SCHEMA'"));
    }

    #[test]
    fn test_expand_primary_key_constraint_bigquery() {
        let marker = r#"-- WM_INTERNAL_DB_PRIMARY_KEY_CONSTRAINT {"table":"dataset.users"}"#;
        let sql = expand_code(marker, &ScriptLang::Bigquery);
        assert!(sql.contains("INFORMATION_SCHEMA.TABLE_CONSTRAINTS"));
        assert!(sql.contains("table_name = 'users'"));
    }

    #[test]
    fn test_expand_primary_key_constraint_duckdb() {
        let marker = r#"-- WM_INTERNAL_DB_PRIMARY_KEY_CONSTRAINT {"table":"users"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.contains("constraint_type = 'PRIMARY KEY'"));
        assert!(sql.contains("tc.table_name = 'users'"));
        assert!(sql.contains("tc.table_schema = 'main'"));
    }

    #[test]
    fn test_expand_primary_key_constraint_with_ducklake() {
        let marker =
            r#"-- WM_INTERNAL_DB_PRIMARY_KEY_CONSTRAINT {"table":"users","ducklake":"lake"}"#;
        let sql = expand_code(marker, &ScriptLang::DuckDb);
        assert!(sql.starts_with("ATTACH 'ducklake://lake' AS dl;USE dl;\n"));
    }

    // -----------------------------------------------------------------------
    // SNOWFLAKE_PRIMARY_KEYS
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_snowflake_primary_keys_single_table() {
        let marker = r#"-- WM_INTERNAL_DB_SNOWFLAKE_PRIMARY_KEYS {"table":"MY_SCHEMA.MY_TABLE"}"#;
        let sql = expand_code(marker, &ScriptLang::Snowflake);
        assert_eq!(sql, "SHOW PRIMARY KEYS IN TABLE MY_SCHEMA.MY_TABLE");
    }

    #[test]
    fn test_expand_snowflake_primary_keys_all() {
        let marker = r#"-- WM_INTERNAL_DB_SNOWFLAKE_PRIMARY_KEYS {}"#;
        let sql = expand_code(marker, &ScriptLang::Snowflake);
        assert_eq!(sql, "SHOW PRIMARY KEYS IN ACCOUNT");
    }
}
