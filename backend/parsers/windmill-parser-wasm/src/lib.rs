use serde_json::{self, json};
use wasm_bindgen::prelude::*;
use windmill_parser::MainArgSignature;
use windmill_parser_ts::parse_expr_for_ids;

fn wrap_sig(r: anyhow::Result<MainArgSignature>) -> String {
    if let Ok(r) = r {
        return serde_json::to_string(&r).unwrap();
    } else {
        return "{\"type\": \"Invalid\"}".to_string();
    }
}

#[wasm_bindgen]
pub fn parse_deno(code: &str) -> String {
    wrap_sig(windmill_parser_ts::parse_deno_signature(code, false))
}

#[wasm_bindgen]
pub fn parse_outputs(code: &str) -> String {
    let parsed = parse_expr_for_ids(code);
    let r = if let Ok(parsed) = parsed {
        json!({ "outputs": parsed })
    } else {
        json!({"error": parsed.err().unwrap().to_string()})
    };
    return serde_json::to_string(&r).unwrap();
}

#[wasm_bindgen]
pub fn parse_bash(code: &str) -> String {
    wrap_sig(windmill_parser_bash::parse_bash_sig(code))
}

#[wasm_bindgen]
pub fn parse_powershell(code: &str) -> String {
    wrap_sig(windmill_parser_bash::parse_powershell_sig(code))
}

#[wasm_bindgen]
pub fn parse_go(code: &str) -> String {
    wrap_sig(windmill_parser_go::parse_go_sig(code))
}

#[wasm_bindgen]
pub fn parse_python(code: &str) -> String {
    wrap_sig(windmill_parser_py::parse_python_signature(code))
}

#[wasm_bindgen]
pub fn parse_sql(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_pgsql_sig(code))
}

#[wasm_bindgen]
pub fn parse_mysql(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_mysql_sig(code))
}

#[wasm_bindgen]
pub fn parse_bigquery(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_bigquery_sig(code))
}

#[wasm_bindgen]
pub fn parse_snowflake(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_snowflake_sig(code))
}

#[wasm_bindgen]
pub fn parse_mssql(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_mssql_sig(code))
}

#[wasm_bindgen]
pub fn parse_graphql(code: &str) -> String {
    wrap_sig(windmill_parser_graphql::parse_graphql_sig(code))
}
