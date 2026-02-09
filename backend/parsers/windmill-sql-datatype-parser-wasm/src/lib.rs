use wasm_bindgen::prelude::*;
use windmill_parser_sql::Typ;

fn to_str(typ: Typ) -> String {
    match typ {
        Typ::Str(_) => "str".to_string(),
        Typ::Int => "int".to_string(),
        Typ::Float => "float".to_string(),
        Typ::Bool => "bool".to_string(),
        Typ::List(t) => format!("list-{}", to_str(*t)),
        Typ::Bytes => "bytes".to_string(),
        Typ::Datetime => "datetime".to_string(),
        Typ::Date => "date".to_string(),
        _ => "unknown".to_string(),
    }
}

#[wasm_bindgen]
pub fn parse_sql(typ: &str) -> String {
    to_str(windmill_parser_sql::parse_pg_typ(typ))
}

#[wasm_bindgen]
pub fn parse_mysql(typ: &str) -> String {
    to_str(windmill_parser_sql::parse_mysql_typ(typ))
}

#[wasm_bindgen]
pub fn parse_bigquery(typ: &str) -> String {
    to_str(windmill_parser_sql::parse_bigquery_typ(typ))
}

#[wasm_bindgen]
pub fn parse_snowflake(typ: &str) -> String {
    to_str(windmill_parser_sql::parse_snowflake_typ(typ))
}

#[wasm_bindgen]
pub fn parse_mssql(typ: &str) -> String {
    to_str(windmill_parser_sql::parse_mssql_typ(typ))
}

#[wasm_bindgen]
pub fn parse_duckdb(typ: &str) -> String {
    to_str(windmill_parser_sql::parse_duckdb_typ(typ))
}
