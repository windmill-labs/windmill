use serde::{Deserialize, Serialize};
use windmill_types::scripts::ScriptLang;

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
    #[serde(default)]
    pub defaultvalue: String,
    #[serde(default)]
    pub isprimarykey: bool,
    #[serde(default)]
    pub isidentity: ColumnIdentity,
    #[serde(default)]
    pub isnullable: String,
    #[serde(default)]
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
    let attach = format!("ATTACH 'ducklake://{}' AS dl;USE dl;\n", ducklake);
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

/// Checks if a SQL script is a WM_INTERNAL_DB marker and expands it into real SQL.
/// Returns `None` if the script is not a marker (regular SQL passthrough).
/// Returns `Some(Ok(sql))` on successful expansion.
/// Returns `Some(Err(msg))` if the marker is detected but malformed.
pub fn try_expand_internal_db_query(
    code: &str,
    lang: &ScriptLang,
) -> Option<Result<String, String>> {
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
        "SELECT" => expand_select(json_str, db_type),
        "COUNT" => expand_count(json_str, db_type),
        "DELETE" => expand_delete(json_str, db_type),
        "INSERT" => expand_insert(json_str, db_type),
        "UPDATE" => expand_update(json_str, db_type),
        _ => Err(format!("Unknown WM_INTERNAL_DB operation: {}", op)),
    };

    Some(result)
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
            format!("\"{}\"", identifier)
        }
        DbType::MsSqlServer => format!("[{}]", identifier),
        DbType::Mysql | DbType::Bigquery => format!("`{}`", identifier),
    }
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
    query.push_str(&format!("SELECT {} FROM {}", select_clause, table));

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
            format!(
                "CASE WHEN ? = '{}' AND ? = FALSE THEN \"{}\" END ASC,\n\t\tCASE WHEN ? = '{}' AND ? = TRUE THEN \"{}\" END DESC",
                col.field, col.field, col.field, col.field
            )
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
    let number_check_expr = match check_is_number {
        Some(true) => format!(
            " pg_typeof(\"{}\")::text IN ('integer', 'bigint', 'smallint', 'numeric', 'real', 'double precision') AND",
            field
        ),
        Some(false) => format!(
            " pg_typeof(\"{}\")::text NOT IN ('integer', 'bigint', 'smallint', 'numeric', 'real', 'double precision') AND",
            field
        ),
        None => String::new(),
    };
    let cast = if text_cast { "::text" } else { "" };
    let desc_suffix = if is_desc { " DESC" } else { "" };
    format!(
        "(CASE WHEN{} $4 = '{}' AND $5 IS {} THEN \"{}\"{} END){}",
        number_check_expr, field, is_desc, field, cast, desc_suffix
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
                    format!(
                        "\nCASE WHEN :order_by = '{}' AND :is_desc IS false THEN `{}` END,\nCASE WHEN :order_by = '{}' AND :is_desc IS true THEN `{}` END DESC",
                        col.field, col.field, col.field, col.field
                    )
                })
                .collect::<Vec<_>>()
                .join(",\n");

            let quicksearch = format!(
                " (:quicksearch = '' OR CONCAT_WS(' ', {}) LIKE CONCAT('%', :quicksearch, '%'))",
                filtered_columns.join(", ")
            );

            query.push_str(&format!("SELECT {} FROM {}", select_clause, table));
            query.push_str(&format!(
                " WHERE {}{}",
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
                table
            ));
            query.push_str(&format!(
                " WHERE {}{}\n",
                where_clause
                    .map(|wc| format!("{} AND ", wc))
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
                    format!(
                        "\n(CASE WHEN @p4 = '{}' AND @p5 = 0 THEN {} END) ASC,\n(CASE WHEN @p4 = '{}' AND @p5 = 1 THEN {} END) DESC",
                        col.field, col.field, col.field, col.field
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

            query.push_str(&format!("SELECT {} FROM {}", select_clause, table));
            query.push_str(&format!(
                " WHERE {}{}",
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
                    let is_complex = col.datatype == "JSON"
                        || col.datatype.starts_with("STRUCT")
                        || col.datatype.starts_with("ARRAY")
                        || col.datatype == "GEOGRAPHY";
                    if is_complex {
                        format!(
                            "\n(CASE WHEN @order_by = '{}' AND @is_desc = false THEN TO_JSON_STRING({}) END) ASC,\n(CASE WHEN @order_by = '{}' AND @is_desc = true THEN TO_JSON_STRING({}) END) DESC",
                            col.field, col.field, col.field, col.field
                        )
                    } else {
                        format!(
                            "\n(CASE WHEN @order_by = '{}' AND @is_desc = false THEN {} END) ASC,\n(CASE WHEN @order_by = '{}' AND @is_desc = true THEN {} END) DESC",
                            col.field, col.field, col.field, col.field
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

            query.push_str(&format!("SELECT {} FROM {}", select_clause, table));
            query.push_str(&format!(
                " WHERE {}{}",
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
                    format!(
                        "\n      (CASE WHEN $order_by = '{}' AND $is_desc IS false THEN \"{}\"::text END),\n      (CASE WHEN $order_by = '{}' AND $is_desc IS true THEN \"{}\"::text END) DESC",
                        col.field, col.field, col.field, col.field
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
                table
            ));
            query.push_str(&format!(
                " WHERE {}{}\n",
                where_clause
                    .map(|wc| format!("{} AND ", wc))
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
            query.push_str(&format!("SELECT COUNT(*) as count FROM {}", table));
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
            query.push_str(&format!("SELECT COUNT(*) as count FROM {}", table));
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
            query.push_str(&format!("SELECT COUNT(*) as count FROM [{}]", table));
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
            query.push_str(&format!("SELECT COUNT(*) as count FROM {}", table));
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
            query.push_str(&format!("SELECT COUNT(*) as count FROM `{}`", table));
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
            query.push_str(&format!("SELECT COUNT(*) as count FROM {}", table));
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

    match db_type {
        DbType::Postgresql => {
            let conditions: String = columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    format!(
                        "(${}::text::{} IS NULL AND {} IS NULL OR {} = ${}::text::{})",
                        i + 1,
                        c.datatype,
                        c.field,
                        c.field,
                        i + 1,
                        c.datatype
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nDELETE FROM {} \nWHERE {} RETURNING 1;",
                table, conditions
            ));
        }
        DbType::Mysql => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    format!(
                        "(:{} IS NULL AND {} IS NULL OR {} = :{})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!("\nDELETE FROM {} \nWHERE {}", table, conditions));
        }
        DbType::MsSqlServer => {
            let conditions: String = columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    format!(
                        "(@p{} IS NULL AND {} IS NULL OR {} = @p{})",
                        i + 1,
                        c.field,
                        c.field,
                        i + 1
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!("\nDELETE FROM {} \nWHERE {}", table, conditions));
        }
        DbType::Snowflake => {
            let conditions: String = columns
                .iter()
                .map(|_c| format!("(? = 'null' AND {} IS NULL OR {} = ?)", _c.field, _c.field))
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!("\nDELETE FROM {} \nWHERE {}", table, conditions));
        }
        DbType::Bigquery => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    format!(
                        "(CAST(@{} AS STRING) = 'null' AND {} IS NULL OR {} = @{})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!("\nDELETE FROM {} \nWHERE {}", table, conditions));
        }
        DbType::Duckdb => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    format!(
                        "(${} IS NULL AND {} IS NULL OR {} = ${})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!("\nDELETE FROM {} \nWHERE {}", table, conditions));
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

fn format_column_names(columns: &[ColumnDef]) -> String {
    columns
        .iter()
        .map(|c| c.field.as_str())
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
    );
    let insert_values = format_insert_values(&insert_cols_owned, db_type, 1);
    let default_cols_owned: Vec<ColumnDef> = columns_default.iter().map(|c| (*c).clone()).collect();
    let default_values = format_default_values(&default_cols_owned);
    let comma_or_empty = if should_insert_comma { ", " } else { "" };
    let values_str = format!("{}{}{}", insert_values, comma_or_empty, default_values);

    if values_str.trim().is_empty() {
        return Ok(format!("INSERT INTO {} DEFAULT VALUES", table));
    }

    query.push_str(&format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table, column_names, values_str
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

    match db_type {
        DbType::Postgresql => {
            let conditions: String = columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    format!(
                        "(${}::text::{} IS NULL AND {} IS NULL OR {} = ${}::text::{})",
                        i + 2,
                        c.datatype,
                        c.field,
                        c.field,
                        i + 2,
                        c.datatype
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");

            query.push_str(&format!(
                "\nUPDATE {} SET {} = $1::text::{} \nWHERE {}\tRETURNING 1",
                table, column.field, column.datatype, conditions
            ));
        }
        DbType::Mysql => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    format!(
                        "(:{} IS NULL AND {} IS NULL OR {} = :{})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nUPDATE {} SET {} = :value_to_update \nWHERE {}",
                table, column.field, conditions
            ));
        }
        DbType::MsSqlServer => {
            let conditions: String = columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    format!(
                        "(@p{} IS NULL AND {} IS NULL OR {} = @p{})",
                        i + 2,
                        c.field,
                        c.field,
                        i + 2
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nUPDATE {} SET {} = @p1 \nWHERE {}",
                table, column.field, conditions
            ));
        }
        DbType::Snowflake => {
            let conditions: String = columns
                .iter()
                .map(|c| format!("(? = 'null' AND {} IS NULL OR {} = ?)", c.field, c.field))
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nUPDATE {} SET {} = ? \nWHERE {}",
                table, column.field, conditions
            ));
        }
        DbType::Bigquery => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    format!(
                        "(CAST(@{} AS STRING) = 'null' AND {} IS NULL OR {} = @{})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nUPDATE {} SET {} = @value_to_update \nWHERE {}",
                table, column.field, conditions
            ));
        }
        DbType::Duckdb => {
            let conditions: String = columns
                .iter()
                .map(|c| {
                    format!(
                        "(${} IS NULL AND {} IS NULL OR {} = ${})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect::<Vec<_>>()
                .join("\n    AND ");
            query.push_str(&format!(
                "\nUPDATE {} SET {} = $value_to_update \nWHERE {}",
                table, column.field, conditions
            ));
        }
    }

    query
}

// ---------------------------------------------------------------------------
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
        assert!(result.contains("SELECT \"id\"::text, \"name\"::text FROM my_table\n"));
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
        assert!(result.contains("SELECT `id`, `name` FROM my_table"));
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

        assert!(result.contains("SELECT [id], [name] FROM my_table"));
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
        assert!(result.contains("SELECT \"id\", \"name\" FROM my_table"));
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
        assert!(result.contains("SELECT `id`, `name` FROM my_table"));
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
        assert!(result.contains("SELECT \"id\", \"name\" FROM my_table\n"));
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
        assert!(result.contains("SELECT COUNT(*) as count FROM my_table"));
        assert!(result.contains("($1 = '' OR CONCAT(\"id\", \"name\") ILIKE '%' || $1 || '%')"));
        // Should use WHERE not AND
        assert!(result.contains(" WHERE "));
        assert!(!result.contains("my_table AND "));
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
        assert!(result.contains("SELECT COUNT(*) as count FROM my_table"));
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
        assert!(result.contains("SELECT COUNT(*) as count FROM my_table"));
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
        assert!(result.contains("SELECT COUNT(*) as count FROM my_table"));
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
        assert!(result.contains("DELETE FROM my_table"));
        assert!(result.contains("($1::text::int4 IS NULL AND id IS NULL OR id = $1::text::int4)"));
        assert!(
            result.contains("($2::text::text IS NULL AND name IS NULL OR name = $2::text::text)")
        );
        assert!(result.contains("RETURNING 1;"));
    }

    #[test]
    fn test_delete_mysql() {
        let cols = vec![col("id", "int"), col("name", "varchar")];
        let result = make_delete_query("my_table", &cols, DbType::Mysql);

        assert!(result.contains("-- :id (int)\n-- :name (varchar)"));
        assert!(result.contains("DELETE FROM my_table"));
        assert!(result.contains("(:id IS NULL AND id IS NULL OR id = :id)"));
        assert!(result.contains("(:name IS NULL AND name IS NULL OR name = :name)"));
    }

    #[test]
    fn test_delete_mssql() {
        let cols = vec![col("id", "int"), col("name", "nvarchar")];
        let result = make_delete_query("my_table", &cols, DbType::MsSqlServer);

        assert!(result.contains("-- @p1 id (int)\n-- @p2 name (nvarchar)"));
        assert!(result.contains("(@p1 IS NULL AND id IS NULL OR id = @p1)"));
        assert!(result.contains("(@p2 IS NULL AND name IS NULL OR name = @p2)"));
    }

    #[test]
    fn test_delete_snowflake() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result = make_delete_query("my_table", &cols, DbType::Snowflake);

        // Snowflake doubles columns
        assert!(result.contains("-- ? id (int)\n-- ? id (int)\n-- ? name (text)\n-- ? name (text)"));
        assert!(result.contains("(? = 'null' AND id IS NULL OR id = ?)"));
        assert!(result.contains("(? = 'null' AND name IS NULL OR name = ?)"));
    }

    #[test]
    fn test_delete_bigquery() {
        let cols = vec![col("id", "INTEGER"), col("name", "STRING")];
        let result = make_delete_query("my_table", &cols, DbType::Bigquery);

        assert!(result.contains("-- @id (INTEGER)\n-- @name (STRING)"));
        assert!(result.contains("(CAST(@id AS STRING) = 'null' AND id IS NULL OR id = @id)"));
    }

    #[test]
    fn test_delete_duckdb() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result = make_delete_query("my_table", &cols, DbType::Duckdb);

        assert!(result.contains("-- $id (int)\n-- $name (text)"));
        assert!(result.contains("($id IS NULL AND id IS NULL OR id = $id)"));
    }

    // -----------------------------------------------------------------------
    // INSERT - basic cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_insert_postgresql_basic() {
        let cols = vec![col("id", "int4"), col("name", "text")];
        let result = make_insert_query("my_table", &cols, DbType::Postgresql).unwrap();

        assert!(result.contains("-- $1 id\n-- $2 name"));
        assert!(result.contains("INSERT INTO my_table (id, name) VALUES ($1::int4, $2::text)"));
    }

    #[test]
    fn test_insert_mysql_basic() {
        let cols = vec![col("id", "int"), col("name", "varchar")];
        let result = make_insert_query("my_table", &cols, DbType::Mysql).unwrap();

        assert!(result.contains("INSERT INTO my_table (id, name) VALUES (:id, :name)"));
    }

    #[test]
    fn test_insert_mssql_basic() {
        let cols = vec![col("id", "int"), col("name", "nvarchar")];
        let result = make_insert_query("my_table", &cols, DbType::MsSqlServer).unwrap();

        assert!(result.contains("INSERT INTO my_table (id, name) VALUES (@p1, @p2)"));
    }

    #[test]
    fn test_insert_snowflake_basic() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result = make_insert_query("my_table", &cols, DbType::Snowflake).unwrap();

        assert!(result.contains("INSERT INTO my_table (id, name) VALUES (?, ?)"));
    }

    #[test]
    fn test_insert_bigquery_basic() {
        let cols = vec![col("id", "INTEGER"), col("name", "STRING")];
        let result = make_insert_query("my_table", &cols, DbType::Bigquery).unwrap();

        assert!(result.contains("INSERT INTO my_table (id, name) VALUES (@id, @name)"));
    }

    #[test]
    fn test_insert_duckdb_basic() {
        let cols = vec![col("id", "int"), col("name", "text")];
        let result = make_insert_query("my_table", &cols, DbType::Duckdb).unwrap();

        assert!(result.contains("INSERT INTO my_table (id, name) VALUES ($id, $name)"));
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
        assert!(result.contains("INSERT INTO my_table (name) VALUES ($1::text)"));
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
        assert!(result.contains("INSERT INTO my_table (name, id) VALUES ($1::text, '42')"));
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
        assert!(result.contains("INSERT INTO my_table (name) VALUES ($1::text)"));
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
        assert!(result.contains("INSERT INTO my_table (name) VALUES ($1::text)"));
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
        assert_eq!(result, "INSERT INTO my_table DEFAULT VALUES");
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
        assert!(result.contains("UPDATE my_table SET name = $1::text::text"));
        assert!(result.contains("($2::text::int4 IS NULL AND id IS NULL OR id = $2::text::int4)"));
        assert!(
            result.contains("($3::text::text IS NULL AND email IS NULL OR email = $3::text::text)")
        );
        assert!(result.contains("RETURNING 1"));
    }

    #[test]
    fn test_update_mysql() {
        let update_col = simple_col("name", "varchar");
        let where_cols = vec![simple_col("id", "int")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::Mysql);

        assert!(result.contains("UPDATE my_table SET name = :value_to_update"));
        assert!(result.contains("(:id IS NULL AND id IS NULL OR id = :id)"));
    }

    #[test]
    fn test_update_mssql() {
        let update_col = simple_col("name", "nvarchar");
        let where_cols = vec![simple_col("id", "int")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::MsSqlServer);

        assert!(result.contains("UPDATE my_table SET name = @p1"));
        assert!(result.contains("(@p2 IS NULL AND id IS NULL OR id = @p2)"));
    }

    #[test]
    fn test_update_snowflake() {
        let update_col = simple_col("name", "text");
        let where_cols = vec![simple_col("id", "int")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::Snowflake);

        // Snowflake doubles where columns
        assert!(result.contains("-- ? value_to_update (text)\n-- ? id (int)\n-- ? id (int)"));
        assert!(result.contains("UPDATE my_table SET name = ?"));
        assert!(result.contains("(? = 'null' AND id IS NULL OR id = ?)"));
    }

    #[test]
    fn test_update_bigquery() {
        let update_col = simple_col("name", "STRING");
        let where_cols = vec![simple_col("id", "INTEGER")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::Bigquery);

        assert!(result.contains("UPDATE my_table SET name = @value_to_update"));
        assert!(result.contains("(CAST(@id AS STRING) = 'null' AND id IS NULL OR id = @id)"));
    }

    #[test]
    fn test_update_duckdb() {
        let update_col = simple_col("name", "text");
        let where_cols = vec![simple_col("id", "int")];
        let result = make_update_query("my_table", &update_col, &where_cols, DbType::Duckdb);

        assert!(result.contains("UPDATE my_table SET name = $value_to_update"));
        assert!(result.contains("($id IS NULL AND id IS NULL OR id = $id)"));
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
        assert!(result.contains("FROM my_table WHERE "));
        assert!(!result.contains("FROM my_table AND "));
    }

    #[test]
    fn test_delete_multiple_conditions_joined_with_and() {
        let cols = vec![col("a", "int4"), col("b", "text"), col("c", "bool")];
        let result = make_delete_query("t", &cols, DbType::Postgresql);
        // Check all three conditions are present and joined
        assert!(result.contains("AND ($2::text::text IS NULL AND b IS NULL OR b = $2::text::text)"));
        assert!(result.contains("AND ($3::text::bool IS NULL AND c IS NULL OR c = $3::text::bool)"));
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

    #[test]
    fn test_expand_select_marker() {
        let marker = r#"-- WM_INTERNAL_DB_SELECT {"table":"my_table","columnDefs":[{"field":"id","datatype":"int4"},{"field":"name","datatype":"text"}]}"#;
        let result = try_expand_internal_db_query(marker, &ScriptLang::Postgresql);
        assert!(result.is_some());
        let sql = result.unwrap().unwrap();
        assert!(sql.contains("SELECT \"id\"::text, \"name\"::text FROM my_table"));
        assert!(sql.contains("LIMIT $1::INT OFFSET $2::INT"));
    }

    #[test]
    fn test_expand_select_with_where_and_options() {
        let marker = r#"-- WM_INTERNAL_DB_SELECT {"table":"t","columnDefs":[{"field":"id","datatype":"int4"}],"whereClause":"active = true","limit":50,"offset":10,"fixPgIntTypes":true}"#;
        let result = try_expand_internal_db_query(marker, &ScriptLang::Postgresql);
        let sql = result.unwrap().unwrap();
        assert!(sql.contains("WHERE active = true AND"));
        assert!(sql.contains("pg_typeof(\"id\")"));
    }

    #[test]
    fn test_expand_count_marker() {
        let marker = r#"-- WM_INTERNAL_DB_COUNT {"table":"my_table","columnDefs":[{"field":"id","datatype":"int4"}]}"#;
        let result = try_expand_internal_db_query(marker, &ScriptLang::Mysql);
        let sql = result.unwrap().unwrap();
        assert!(sql.contains("SELECT COUNT(*) as count FROM my_table"));
        assert!(sql.contains(":quicksearch"));
    }

    #[test]
    fn test_expand_delete_marker() {
        let marker = r#"-- WM_INTERNAL_DB_DELETE {"table":"my_table","columns":[{"field":"id","datatype":"int4"}]}"#;
        let result = try_expand_internal_db_query(marker, &ScriptLang::Postgresql);
        let sql = result.unwrap().unwrap();
        assert!(sql.contains("DELETE FROM my_table"));
        assert!(sql.contains("RETURNING 1;"));
    }

    #[test]
    fn test_expand_insert_marker() {
        let marker = r#"-- WM_INTERNAL_DB_INSERT {"table":"my_table","columns":[{"field":"id","datatype":"int4"},{"field":"name","datatype":"text"}]}"#;
        let result = try_expand_internal_db_query(marker, &ScriptLang::Postgresql);
        let sql = result.unwrap().unwrap();
        assert!(sql.contains("INSERT INTO my_table (id, name) VALUES ($1::int4, $2::text)"));
    }

    #[test]
    fn test_expand_update_marker() {
        let marker = r#"-- WM_INTERNAL_DB_UPDATE {"table":"my_table","column":{"field":"name","datatype":"text"},"columns":[{"field":"id","datatype":"int4"}]}"#;
        let result = try_expand_internal_db_query(marker, &ScriptLang::Postgresql);
        let sql = result.unwrap().unwrap();
        assert!(sql.contains("UPDATE my_table SET name = $1::text::text"));
        assert!(sql.contains("RETURNING 1"));
    }

    #[test]
    fn test_expand_with_ducklake() {
        let marker = r#"-- WM_INTERNAL_DB_SELECT {"table":"my_table","columnDefs":[{"field":"id","datatype":"int4"}],"ducklake":"my_lake"}"#;
        let result = try_expand_internal_db_query(marker, &ScriptLang::DuckDb);
        let sql = result.unwrap().unwrap();
        assert!(sql.contains("ATTACH 'ducklake://my_lake' AS dl;USE dl;"));
        assert!(sql.contains("SELECT \"id\" FROM my_table"));
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
        let result = try_expand_internal_db_query(marker, &ScriptLang::Mysql);
        let sql = result.unwrap().unwrap();
        assert!(sql.contains("SELECT `id`, `name` FROM users"));
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
    fn test_wrap_ducklake_query() {
        let query = "-- $1 limit\n-- $2 offset\nSELECT * FROM t";
        let wrapped = wrap_ducklake_query(query, "my_lake");
        assert_eq!(
            wrapped,
            "-- $1 limit\n-- $2 offset\nATTACH 'ducklake://my_lake' AS dl;USE dl;\nSELECT * FROM t"
        );
    }
}
