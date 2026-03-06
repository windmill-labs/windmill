use serde::Deserialize;

pub const WM_INTERNAL_PREFIX: &str = "-- WM_INTERNAL_DB_";

#[derive(Deserialize)]
pub struct ColumnDef {
    pub field: String,
    pub datatype: String,
    #[serde(default)]
    pub isprimarykey: bool,
    #[serde(default)]
    pub ignored: bool,
    #[serde(default)]
    pub editable: bool,
    #[serde(default = "default_isnullable")]
    pub isnullable: String,
    #[serde(default = "default_isidentity")]
    pub isidentity: String,
    #[serde(default)]
    pub defaultvalue: Option<String>,
    #[serde(rename = "hideInsert")]
    #[serde(default)]
    pub hide_insert: bool,
    #[serde(rename = "overrideDefaultValue")]
    #[serde(default)]
    pub override_default_value: bool,
    #[serde(rename = "defaultUserValue")]
    #[serde(default)]
    pub default_user_value: Option<serde_json::Value>,
    #[serde(rename = "defaultValueNull")]
    #[serde(default)]
    pub default_value_null: bool,
}

fn default_isnullable() -> String {
    "YES".to_string()
}

fn default_isidentity() -> String {
    "No".to_string()
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DbType {
    Mysql,
    #[serde(alias = "ms_sql_server")]
    MsSqlServer,
    Postgresql,
    Snowflake,
    Bigquery,
    Duckdb,
}

#[derive(Deserialize)]
pub struct SelectParams {
    pub table: String,
    pub column_defs: Vec<ColumnDef>,
    pub where_clause: Option<String>,
    pub db_type: DbType,
    pub ducklake: Option<String>,
}

#[derive(Deserialize)]
pub struct CountParams {
    pub table: String,
    pub column_defs: Vec<ColumnDef>,
    pub where_clause: Option<String>,
    pub db_type: DbType,
    pub ducklake: Option<String>,
}

#[derive(Deserialize)]
pub struct InsertParams {
    pub table: String,
    pub columns: Vec<ColumnDef>,
    pub db_type: DbType,
    pub ducklake: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateParams {
    pub table: String,
    pub column: SimpleColumn,
    pub columns: Vec<SimpleColumn>,
    pub db_type: DbType,
    pub ducklake: Option<String>,
}

#[derive(Deserialize)]
pub struct DeleteParams {
    pub table: String,
    pub columns: Vec<SimpleColumn>,
    pub db_type: DbType,
    pub ducklake: Option<String>,
}

#[derive(Deserialize)]
pub struct SimpleColumn {
    pub field: String,
    pub datatype: String,
}

// --- Quoting ---

fn quote_ident(field: &str, db_type: &DbType) -> String {
    match db_type {
        DbType::Postgresql | DbType::Snowflake | DbType::Duckdb => format!("\"{}\"", field),
        DbType::MsSqlServer => format!("[{}]", field),
        DbType::Mysql | DbType::Bigquery => format!("`{}`", field),
    }
}

// --- Build visible field list (filter ignored columns) ---

fn build_visible_field_list(column_defs: &[ColumnDef], db_type: &DbType) -> Vec<String> {
    column_defs
        .iter()
        .filter(|c| !c.ignored)
        .map(|c| quote_ident(&c.field, db_type))
        .collect()
}

// --- Build parameters comment header ---

struct ParamEntry {
    field: String,
    datatype: String,
}

fn build_parameters(columns: &[ParamEntry], db_type: &DbType) -> String {
    columns
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let base_type = c.datatype.split('(').next().unwrap_or(&c.datatype);
            match db_type {
                DbType::Postgresql => format!("-- ${} {}", i + 1, c.field),
                DbType::Mysql => format!("-- :{} ({})", c.field, base_type),
                DbType::MsSqlServer => format!("-- @p{} {} ({})", i + 1, c.field, base_type),
                DbType::Snowflake => format!("-- ? {} ({})", c.field, base_type),
                DbType::Bigquery => format!("-- @{} ({})", c.field, base_type),
                DbType::Duckdb => format!("-- ${} ({})", c.field, base_type),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn col_to_param(c: &ColumnDef) -> ParamEntry {
    ParamEntry { field: c.field.clone(), datatype: c.datatype.clone() }
}

fn simple_to_param(c: &SimpleColumn) -> ParamEntry {
    ParamEntry { field: c.field.clone(), datatype: c.datatype.clone() }
}

// ===================== SELECT =====================

pub fn make_select_query(p: &SelectParams) -> String {
    match p.db_type {
        DbType::Snowflake => make_snowflake_select(p),
        _ => make_generic_select(p),
    }
}

fn make_snowflake_select(p: &SelectParams) -> String {
    let filtered = build_visible_field_list(&p.column_defs, &p.db_type);
    let select_clause = filtered.join(", ");

    let mut headers: Vec<ParamEntry> =
        vec![ParamEntry { field: "quicksearch".into(), datatype: "text".into() }];

    let qs_parts: Vec<String> = std::iter::once("LENGTH(?) = 0".to_string())
        .chain(filtered.iter().map(|col| {
            headers.push(ParamEntry { field: "quicksearch".into(), datatype: "text".into() });
            format!("CONCAT({}) ILIKE CONCAT('%', ?, '%')", col)
        }))
        .collect();
    let quicksearch_condition = qs_parts.join(" OR ");

    let order_parts: Vec<String> = p
        .column_defs
        .iter()
        .map(|c| {
            for _ in 0..4 {
                headers.push(ParamEntry { field: "order_by".into(), datatype: "text".into() });
            }
            // Actually snowflake uses order_by, is_desc alternating
            // Let me re-read: it pushes order_by, is_desc, order_by, is_desc for each column
            format!(
                "CASE WHEN ? = '{}' AND ? = FALSE THEN \"{}\" END ASC,\n\t\tCASE WHEN ? = '{}' AND ? = TRUE THEN \"{}\" END DESC",
                c.field, c.field, c.field, c.field
            )
        })
        .collect();

    // Fix headers: snowflake actually pushes order_by, is_desc pairs (not 4x order_by)
    // Re-read the TS: it pushes { field: 'order_by' }, { field: 'is_desc' }, { field: 'order_by' }, { field: 'is_desc' }
    // But the datatypes are text and boolean. Let me redo:
    headers.clear();
    headers.push(ParamEntry { field: "quicksearch".into(), datatype: "text".into() });
    for col in &filtered {
        headers.push(ParamEntry { field: "quicksearch".into(), datatype: "text".into() });
        let _ = col; // just for the count
    }
    for _ in &p.column_defs {
        headers.push(ParamEntry { field: "order_by".into(), datatype: "text".into() });
        headers.push(ParamEntry { field: "is_desc".into(), datatype: "boolean".into() });
        headers.push(ParamEntry { field: "order_by".into(), datatype: "text".into() });
        headers.push(ParamEntry { field: "is_desc".into(), datatype: "boolean".into() });
    }

    let params = build_parameters(&headers, &DbType::Snowflake);

    let where_clause = if let Some(wc) = &p.where_clause {
        format!(" WHERE {} AND ({})", wc, quicksearch_condition)
    } else {
        format!(" WHERE {}", quicksearch_condition)
    };

    format!(
        "{}\n\nSELECT {} FROM {}{} ORDER BY {} LIMIT 100 OFFSET 0",
        params,
        select_clause,
        p.table,
        where_clause,
        order_parts.join(",\n")
    )
}

fn make_generic_select(p: &SelectParams) -> String {
    let filtered = build_visible_field_list(&p.column_defs, &p.db_type);
    let is_bq = matches!(p.db_type, DbType::Bigquery);

    let params = build_parameters(
        &[
            ParamEntry {
                field: "limit".into(),
                datatype: if is_bq { "integer" } else { "int" }.into(),
            },
            ParamEntry {
                field: "offset".into(),
                datatype: if is_bq { "integer" } else { "int" }.into(),
            },
            ParamEntry {
                field: "quicksearch".into(),
                datatype: if is_bq { "string" } else { "text" }.into(),
            },
            ParamEntry {
                field: "order_by".into(),
                datatype: if is_bq { "string" } else { "text" }.into(),
            },
            ParamEntry {
                field: "is_desc".into(),
                datatype: if is_bq { "bool" } else { "boolean" }.into(),
            },
        ],
        &p.db_type,
    );

    let mut query = params;
    query.push('\n');

    match p.db_type {
        DbType::Mysql => {
            let order_by: Vec<String> = p
                .column_defs
                .iter()
                .map(|c| {
                    format!(
                        "\nCASE WHEN :order_by = '{}' AND :is_desc IS false THEN `{}` END,\nCASE WHEN :order_by = '{}' AND :is_desc IS true THEN `{}` END DESC",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect();

            let qs = format!(
                " (:quicksearch = '' OR CONCAT_WS(' ', {}) LIKE CONCAT('%', :quicksearch, '%'))",
                filtered.join(", ")
            );

            query += &format!("SELECT {} FROM {}", filtered.join(", "), p.table);
            query += &format!(
                " WHERE {}{}",
                p.where_clause
                    .as_ref()
                    .map(|wc| format!("{} AND", wc))
                    .unwrap_or_default(),
                qs
            );
            query += &format!(" ORDER BY {}", order_by.join(",\n"));
            query += " LIMIT :limit OFFSET :offset";
        }
        DbType::Postgresql => {
            let order_parts: Vec<String> = p
                .column_defs
                .iter()
                .map(|c| {
                    // Always use text cast (no fixPgIntTypes breaking feature for new internal scripts)
                    format!(
                        "\n      (CASE WHEN $4 = '{}' AND $5 IS false THEN \"{}\"::text END),\n      (CASE WHEN $4 = '{}' AND $5 IS true THEN \"{}\"::text END) DESC",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect();

            let qs = format!(
                "($3 = '' OR CONCAT({}) ILIKE '%' || $3 || '%')",
                filtered.join(", ")
            );

            let select_cols = filtered
                .iter()
                .map(|c| format!("{}::text", c))
                .collect::<Vec<_>>()
                .join(", ");

            query += &format!("SELECT {} FROM {}\n", select_cols, p.table);
            query += &format!(
                " WHERE {}{}\n",
                p.where_clause
                    .as_ref()
                    .map(|wc| format!("{} AND ", wc))
                    .unwrap_or_default(),
                qs
            );
            query += &format!(" ORDER BY {}\n", order_parts.join(",\n"));
            query += " LIMIT $1::INT OFFSET $2::INT";
        }
        DbType::MsSqlServer => {
            let unsortable = ["text", "ntext", "image"];

            let order_parts: Vec<String> = p
                .column_defs
                .iter()
                .filter(|c| !unsortable.contains(&c.datatype.to_lowercase().as_str()))
                .map(|c| {
                    format!(
                        "\n(CASE WHEN @p4 = '{}' AND @p5 = 0 THEN {} END) ASC,\n(CASE WHEN @p4 = '{}' AND @p5 = 1 THEN {} END) DESC",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect();

            let search_cols: Vec<&String> = filtered
                .iter()
                .filter(|col| {
                    let field_name = &col[1..col.len() - 1]; // Remove brackets
                    let def = p.column_defs.iter().find(|c| c.field == field_name);
                    !unsortable.contains(
                        &def.map(|d| d.datatype.to_lowercase())
                            .unwrap_or_default()
                            .as_str(),
                    )
                })
                .collect();

            let qs = if !search_cols.is_empty() {
                format!(
                    " (@p3 = '' OR CONCAT({}) LIKE '%' + @p3 + '%')",
                    search_cols
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                " (@p3 = '')".to_string()
            };

            query += &format!("SELECT {} FROM {}", filtered.join(", "), p.table);
            query += &format!(
                " WHERE {}{}",
                p.where_clause
                    .as_ref()
                    .map(|wc| format!("{} AND", wc))
                    .unwrap_or_default(),
                qs
            );
            query += &format!(" ORDER BY {}", order_parts.join(",\n"));
            query += " OFFSET @p2 ROWS FETCH NEXT @p1 ROWS ONLY";
        }
        DbType::Bigquery => {
            let order_parts: Vec<String> = p
                .column_defs
                .iter()
                .map(|c| {
                    let needs_json = c.datatype == "JSON"
                        || c.datatype.starts_with("STRUCT")
                        || c.datatype.starts_with("ARRAY")
                        || c.datatype == "GEOGRAPHY";
                    if needs_json {
                        format!(
                            "\n(CASE WHEN @order_by = '{}' AND @is_desc = false THEN TO_JSON_STRING({}) END) ASC,\n(CASE WHEN @order_by = '{}' AND @is_desc = true THEN TO_JSON_STRING({}) END) DESC",
                            c.field, c.field, c.field, c.field
                        )
                    } else {
                        format!(
                            "\n(CASE WHEN @order_by = '{}' AND @is_desc = false THEN {} END) ASC,\n(CASE WHEN @order_by = '{}' AND @is_desc = true THEN {} END) DESC",
                            c.field, c.field, c.field, c.field
                        )
                    }
                })
                .collect();

            let search_parts: Vec<String> = filtered
                .iter()
                .map(|col| {
                    let field_name = &col[1..col.len() - 1];
                    let def = p.column_defs.iter().find(|c| c.field == field_name);
                    let needs_json = def.map_or(false, |d| {
                        d.datatype == "JSON"
                            || d.datatype.starts_with("STRUCT")
                            || d.datatype.starts_with("ARRAY")
                            || d.datatype == "GEOGRAPHY"
                    });
                    if needs_json {
                        format!("TO_JSON_STRING({})", col)
                    } else {
                        format!("CAST({} AS STRING)", col)
                    }
                })
                .collect();
            let qs = format!(
                " (@quicksearch = '' OR REGEXP_CONTAINS(CONCAT({}), '(?i)' || @quicksearch))",
                search_parts.join(",")
            );

            query += &format!("SELECT {} FROM {}", filtered.join(", "), p.table);
            query += &format!(
                " WHERE {}{}",
                p.where_clause
                    .as_ref()
                    .map(|wc| format!("{} AND", wc))
                    .unwrap_or_default(),
                qs
            );
            query += &format!(" ORDER BY {}", order_parts.join(",\n"));
            query += " LIMIT @limit OFFSET @offset";
        }
        DbType::Duckdb => {
            let order_parts: Vec<String> = p
                .column_defs
                .iter()
                .map(|c| {
                    format!(
                        "\n      (CASE WHEN $order_by = '{}' AND $is_desc IS false THEN \"{}\"::text END),\n      (CASE WHEN $order_by = '{}' AND $is_desc IS true THEN \"{}\"::text END) DESC",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect();

            let qs = format!(
                "($quicksearch = '' OR CONCAT({}) ILIKE '%' || $quicksearch || '%')",
                filtered.join(", ")
            );

            query += &format!("SELECT {} FROM {}\n", filtered.join(", "), p.table);
            query += &format!(
                " WHERE {}{}\n",
                p.where_clause
                    .as_ref()
                    .map(|wc| format!("{} AND ", wc))
                    .unwrap_or_default(),
                qs
            );
            query += &format!(" ORDER BY {}\n", order_parts.join(",\n"));
            query += " LIMIT $limit::INT OFFSET $offset::INT";
        }
        DbType::Snowflake => unreachable!(),
    }

    query
}

// ===================== COUNT =====================

pub fn make_count_query(p: &CountParams) -> String {
    let filtered = build_visible_field_list(&p.column_defs, &p.db_type);
    let is_bq = matches!(p.db_type, DbType::Bigquery);

    let mut query;
    let mut quicksearch_condition = String::new();

    if let Some(wc) = &p.where_clause {
        quicksearch_condition = format!(" {} AND ", wc);
    }

    match p.db_type {
        DbType::Snowflake => {
            if !filtered.is_empty() {
                let params = build_parameters(
                    &[
                        ParamEntry { field: "quicksearch".into(), datatype: "text".into() },
                        ParamEntry { field: "quicksearch".into(), datatype: "text".into() },
                    ],
                    &p.db_type,
                );
                query = params;
                query.push('\n');
                quicksearch_condition += &format!(
                    "(? = '' OR CONCAT({}) ILIKE '%' || ? || '%')",
                    filtered.join(", ")
                );
            } else {
                let params = build_parameters(
                    &[ParamEntry { field: "quicksearch".into(), datatype: "text".into() }],
                    &p.db_type,
                );
                query = params;
                query.push('\n');
                quicksearch_condition += "(? = '' OR 1 = 1)";
            }
            query += &format!("SELECT COUNT(*) as count FROM {}", p.table);
        }
        _ => {
            let params = build_parameters(
                &[ParamEntry {
                    field: "quicksearch".into(),
                    datatype: if is_bq { "string" } else { "text" }.into(),
                }],
                &p.db_type,
            );
            query = params;
            query.push('\n');

            match p.db_type {
                DbType::Mysql => {
                    if !filtered.is_empty() {
                        quicksearch_condition += &format!(
                            " (:quicksearch = '' OR CONCAT_WS(' ', {}) LIKE CONCAT('%', :quicksearch, '%'))",
                            filtered.join(", ")
                        );
                    } else {
                        quicksearch_condition += " (:quicksearch = '' OR 1 = 1)";
                    }
                    query += &format!("SELECT COUNT(*) as count FROM {}", p.table);
                }
                DbType::Postgresql => {
                    if !filtered.is_empty() {
                        quicksearch_condition += &format!(
                            "($1 = '' OR CONCAT({}) ILIKE '%' || $1 || '%')",
                            filtered.join(", ")
                        );
                    } else {
                        quicksearch_condition += "($1 = '' OR 1 = 1)";
                    }
                    query += &format!("SELECT COUNT(*) as count FROM {}", p.table);
                }
                DbType::MsSqlServer => {
                    if !filtered.is_empty() {
                        quicksearch_condition += &format!(
                            "(@p1 = '' OR CONCAT({}) LIKE '%' + @p1 + '%')",
                            filtered.join(", +")
                        );
                    } else {
                        quicksearch_condition += "(@p1 = '' OR 1 = 1)";
                    }
                    query += &format!("SELECT COUNT(*) as count FROM [{}]", p.table);
                }
                DbType::Bigquery => {
                    if !filtered.is_empty() {
                        let search_parts: Vec<String> = filtered
                            .iter()
                            .map(|col| {
                                let field_name = &col[1..col.len() - 1];
                                let def = p.column_defs.iter().find(|c| c.field == field_name);
                                let needs_json = def.map_or(false, |d| {
                                    d.datatype == "JSON"
                                        || d.datatype.starts_with("STRUCT")
                                        || d.datatype.starts_with("ARRAY")
                                });
                                if needs_json {
                                    format!("TO_JSON_STRING({})", col)
                                } else {
                                    col.clone()
                                }
                            })
                            .collect();
                        quicksearch_condition += &format!(
                            "(@quicksearch = '' OR REGEXP_CONTAINS(CONCAT({}), '(?i)' || @quicksearch))",
                            search_parts.join(",")
                        );
                    } else {
                        quicksearch_condition += "(@quicksearch = '' OR 1 = 1)";
                    }
                    query += &format!("SELECT COUNT(*) as count FROM `{}`", p.table);
                }
                DbType::Duckdb => {
                    if !filtered.is_empty() {
                        quicksearch_condition += &format!(
                            " ($quicksearch = '' OR CONCAT(' ', {}) LIKE CONCAT('%', $quicksearch, '%'))",
                            filtered.join(", ")
                        );
                    } else {
                        quicksearch_condition += " ($quicksearch = '' OR 1 = 1)";
                    }
                    query += &format!("SELECT COUNT(*) as count FROM {}", p.table);
                }
                DbType::Snowflake => unreachable!(),
            }
        }
    }

    // Apply WHERE logic matching the TS
    if p.where_clause.is_some() {
        query += &format!(" WHERE {}", quicksearch_condition);
    } else {
        match p.db_type {
            DbType::MsSqlServer => {
                query += &format!(" WHERE {}", quicksearch_condition);
            }
            _ => {
                query += &format!(" WHERE {}", quicksearch_condition);
            }
        }
    }

    query
}

// ===================== INSERT =====================

pub fn make_insert_query(p: &InsertParams) -> String {
    let columns_insert: Vec<&ColumnDef> = p
        .columns
        .iter()
        .filter(|c| {
            !c.hide_insert
                && !(matches!(p.db_type, DbType::Postgresql)
                    && c.defaultvalue
                        .as_ref()
                        .map_or(false, |d| d.starts_with("nextval(")))
        })
        .collect();

    let columns_default: Vec<&ColumnDef> = p
        .columns
        .iter()
        .filter(|c| !should_omit_column_in_insert(c))
        .collect();

    let all_insert_columns: Vec<&ColumnDef> = columns_insert
        .iter()
        .chain(columns_default.iter())
        .copied()
        .collect();

    let params: Vec<ParamEntry> = columns_insert.iter().map(|c| col_to_param(c)).collect();
    let mut query = build_parameters(&params, &p.db_type);
    query.push('\n');

    let column_names = all_insert_columns
        .iter()
        .map(|c| c.field.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let insert_values = format_insert_values(&columns_insert, &p.db_type, 1);
    let default_values = format_default_values(&columns_default);
    let should_comma = !columns_default.is_empty();
    let comma = if should_comma { ", " } else { "" };
    let values_str = format!("{}{}{}", insert_values, comma, default_values);

    if values_str.trim().is_empty() {
        return format!("INSERT INTO {} DEFAULT VALUES", p.table);
    }

    query += &format!(
        "INSERT INTO {} ({}) VALUES ({})",
        p.table, column_names, values_str
    );
    query
}

fn should_omit_column_in_insert(column: &ColumnDef) -> bool {
    if !column.hide_insert || column.isidentity == "Always" {
        return true;
    }

    let user_default_value = (column.default_user_value.is_some()
        && column.default_user_value.as_ref().map_or(
            true,
            |v| !matches!(v, serde_json::Value::String(s) if s.is_empty()),
        ))
        || column.default_value_null;
    let db_default_value = column
        .defaultvalue
        .as_ref()
        .map_or(false, |d| !d.is_empty());

    if column.isnullable == "NO" {
        if !user_default_value && !db_default_value && column.isidentity == "No" {
            // This would be an error in TS, but we just omit
            return true;
        }
        if !user_default_value && !db_default_value {
            return column.isidentity != "No";
        }
        return !user_default_value && db_default_value;
    } else if column.isnullable == "YES" {
        return !user_default_value;
    }

    false
}

fn format_insert_values(columns: &[&ColumnDef], db_type: &DbType, start_index: usize) -> String {
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

fn get_user_default_value(column: &ColumnDef) -> Option<String> {
    if column.default_value_null {
        return Some("NULL".to_string());
    }
    if let Some(val) = &column.default_user_value {
        match val {
            serde_json::Value::String(s) if !s.is_empty() => return Some(format!("'{}'", s)),
            serde_json::Value::Number(n) => return Some(n.to_string()),
            serde_json::Value::Bool(b) => return Some(b.to_string()),
            _ => return None,
        }
    }
    None
}

fn format_default_values(columns: &[&ColumnDef]) -> String {
    columns
        .iter()
        .map(|c| {
            let user_val = get_user_default_value(c);
            if c.override_default_value {
                user_val.unwrap_or_default()
            } else {
                user_val
                    .or_else(|| c.defaultvalue.clone())
                    .unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

// ===================== UPDATE =====================

pub fn make_update_query(p: &UpdateParams) -> String {
    let mut param_cols: Vec<ParamEntry> =
        vec![ParamEntry { field: "value_to_update".into(), datatype: p.column.datatype.clone() }];

    match p.db_type {
        DbType::Snowflake => {
            for c in &p.columns {
                param_cols.push(simple_to_param(c));
                param_cols.push(simple_to_param(c));
            }
        }
        _ => {
            for c in &p.columns {
                param_cols.push(simple_to_param(c));
            }
        }
    }

    let mut query = build_parameters(&param_cols, &p.db_type);

    match p.db_type {
        DbType::Postgresql => {
            let conditions: Vec<String> = p
                .columns
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
                .collect();
            query += &format!(
                "\n\nUPDATE {} SET {} = $1::text::{} \nWHERE {}\tRETURNING 1",
                p.table,
                p.column.field,
                p.column.datatype,
                conditions.join("\n    AND ")
            );
        }
        DbType::Mysql => {
            let conditions: Vec<String> = p
                .columns
                .iter()
                .map(|c| {
                    format!(
                        "(:{} IS NULL AND {} IS NULL OR {} = :{})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect();
            query += &format!(
                "\n\nUPDATE {} SET {} = :value_to_update \nWHERE {}",
                p.table,
                p.column.field,
                conditions.join("\n    AND ")
            );
        }
        DbType::MsSqlServer => {
            let conditions: Vec<String> = p
                .columns
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
                .collect();
            query += &format!(
                "\n\nUPDATE {} SET {} = @p1 \nWHERE {}",
                p.table,
                p.column.field,
                conditions.join("\n    AND ")
            );
        }
        DbType::Snowflake => {
            let conditions: Vec<String> = p
                .columns
                .iter()
                .map(|c| format!("(? = 'null' AND {} IS NULL OR {} = ?)", c.field, c.field))
                .collect();
            query += &format!(
                "\n\nUPDATE {} SET {} = ? \nWHERE {}",
                p.table,
                p.column.field,
                conditions.join("\n    AND ")
            );
        }
        DbType::Bigquery => {
            let conditions: Vec<String> = p
                .columns
                .iter()
                .map(|c| {
                    format!(
                        "(CAST(@{} AS STRING) = 'null' AND {} IS NULL OR {} = @{})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect();
            query += &format!(
                "\n\nUPDATE {} SET {} = @value_to_update \nWHERE {}",
                p.table,
                p.column.field,
                conditions.join("\n    AND ")
            );
        }
        DbType::Duckdb => {
            let conditions: Vec<String> = p
                .columns
                .iter()
                .map(|c| {
                    format!(
                        "(${} IS NULL AND {} IS NULL OR {} = ${})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect();
            query += &format!(
                "\n\nUPDATE {} SET {} = $value_to_update \nWHERE {}",
                p.table,
                p.column.field,
                conditions.join("\n    AND ")
            );
        }
    }

    query
}

// ===================== DELETE =====================

pub fn make_delete_query(p: &DeleteParams) -> String {
    let param_cols: Vec<ParamEntry> = match p.db_type {
        DbType::Snowflake => p
            .columns
            .iter()
            .flat_map(|c| vec![simple_to_param(c), simple_to_param(c)])
            .collect(),
        _ => p.columns.iter().map(simple_to_param).collect(),
    };

    let mut query = build_parameters(&param_cols, &p.db_type);

    match p.db_type {
        DbType::Postgresql => {
            let conditions: Vec<String> = p
                .columns
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
                .collect();
            query += &format!(
                "\nDELETE FROM {} \nWHERE {} RETURNING 1;",
                p.table,
                conditions.join("\n    AND ")
            );
        }
        DbType::Mysql => {
            let conditions: Vec<String> = p
                .columns
                .iter()
                .map(|c| {
                    format!(
                        "(:{} IS NULL AND {} IS NULL OR {} = :{})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect();
            query += &format!(
                "\nDELETE FROM {} \nWHERE {}",
                p.table,
                conditions.join("\n    AND ")
            );
        }
        DbType::MsSqlServer => {
            let conditions: Vec<String> = p
                .columns
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
                .collect();
            query += &format!(
                "\nDELETE FROM {} \nWHERE {}",
                p.table,
                conditions.join("\n    AND ")
            );
        }
        DbType::Snowflake => {
            let conditions: Vec<String> = p
                .columns
                .iter()
                .map(|c| format!("(? = 'null' AND {} IS NULL OR {} = ?)", c.field, c.field))
                .collect();
            query += &format!(
                "\nDELETE FROM {} \nWHERE {}",
                p.table,
                conditions.join("\n    AND ")
            );
        }
        DbType::Bigquery => {
            let conditions: Vec<String> = p
                .columns
                .iter()
                .map(|c| {
                    format!(
                        "(CAST(@{} AS STRING) = 'null' AND {} IS NULL OR {} = @{})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect();
            query += &format!(
                "\nDELETE FROM {} \nWHERE {}",
                p.table,
                conditions.join("\n    AND ")
            );
        }
        DbType::Duckdb => {
            let conditions: Vec<String> = p
                .columns
                .iter()
                .map(|c| {
                    format!(
                        "(${} IS NULL AND {} IS NULL OR {} = ${})",
                        c.field, c.field, c.field, c.field
                    )
                })
                .collect();
            query += &format!(
                "\nDELETE FROM {} \nWHERE {}",
                p.table,
                conditions.join("\n    AND ")
            );
        }
    }

    query
}

// ===================== Main entry point =====================

fn wrap_ducklake(sql: &str, ducklake: &str) -> String {
    // Insert ATTACH statement after leading comment lines, matching frontend behavior
    let attach = format!("ATTACH 'ducklake://{}' AS dl;USE dl;\n", ducklake);
    // Find end of leading comment block
    let mut insert_pos = 0;
    for line in sql.lines() {
        if line.starts_with("--") {
            insert_pos += line.len() + 1; // +1 for newline
        } else {
            break;
        }
    }
    let mut result = String::with_capacity(sql.len() + attach.len());
    result.push_str(&sql[..insert_pos]);
    result.push_str(&attach);
    result.push_str(&sql[insert_pos..]);
    result
}

/// Checks if a script content is a WM internal DB marker and replaces it with the actual SQL.
/// Returns None if the content is not an internal marker (pass-through).
pub fn maybe_replace_internal_script(content: &str) -> Option<String> {
    let content = content.trim();
    if !content.starts_with(WM_INTERNAL_PREFIX) {
        return None;
    }

    // Find the end of the marker line
    let (marker_line, json_part) = match content.find('\n') {
        Some(pos) => (content[..pos].trim(), content[pos + 1..].trim()),
        None => (content, ""),
    };

    match marker_line {
        "-- WM_INTERNAL_DB_SELECT_SCRIPT" => {
            let params: SelectParams = serde_json::from_str(json_part).ok()?;
            let ducklake = params.ducklake.clone();
            let mut sql = make_select_query(&params);
            if let Some(dl) = &ducklake {
                sql = wrap_ducklake(&sql, dl);
            }
            Some(sql)
        }
        "-- WM_INTERNAL_DB_COUNT_SCRIPT" => {
            let params: CountParams = serde_json::from_str(json_part).ok()?;
            let ducklake = params.ducklake.clone();
            let mut sql = make_count_query(&params);
            if let Some(dl) = &ducklake {
                sql = wrap_ducklake(&sql, dl);
            }
            Some(sql)
        }
        "-- WM_INTERNAL_DB_INSERT_SCRIPT" => {
            let params: InsertParams = serde_json::from_str(json_part).ok()?;
            let ducklake = params.ducklake.clone();
            let mut sql = make_insert_query(&params);
            if let Some(dl) = &ducklake {
                sql = wrap_ducklake(&sql, dl);
            }
            Some(sql)
        }
        "-- WM_INTERNAL_DB_UPDATE_SCRIPT" => {
            let params: UpdateParams = serde_json::from_str(json_part).ok()?;
            let ducklake = params.ducklake.clone();
            let mut sql = make_update_query(&params);
            if let Some(dl) = &ducklake {
                sql = wrap_ducklake(&sql, dl);
            }
            Some(sql)
        }
        "-- WM_INTERNAL_DB_DELETE_SCRIPT" => {
            let params: DeleteParams = serde_json::from_str(json_part).ok()?;
            let ducklake = params.ducklake.clone();
            let mut sql = make_delete_query(&params);
            if let Some(dl) = &ducklake {
                sql = wrap_ducklake(&sql, dl);
            }
            Some(sql)
        }
        _ => None,
    }
}
