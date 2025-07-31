#[cfg(feature = "ts-parser")]
use serde_json::json;
#[allow(unused_imports)]
use wasm_bindgen::prelude::*;
use windmill_parser::MainArgSignature;
#[cfg(feature = "ts-parser")]
use windmill_parser_ts::{parse_expr_for_ids, parse_expr_for_imports};

#[allow(dead_code)]
fn wrap_sig(r: anyhow::Result<MainArgSignature>) -> String {
    if let Ok(r) = r {
        return serde_json::to_string(&r).unwrap();
    } else {
        return "{\"type\": \"Invalid\"}".to_string();
    }
}

#[cfg(feature = "ts-parser")]
#[wasm_bindgen]
pub fn parse_deno(code: &str, main_override: Option<String>) -> String {
    wrap_sig(windmill_parser_ts::parse_deno_signature(
        code,
        false,
        false,
        main_override,
    ))
}

#[cfg(feature = "ts-parser")]
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

#[cfg(feature = "ts-parser")]
#[wasm_bindgen]
pub fn parse_ts_imports(code: &str) -> String {
    let parsed = parse_expr_for_imports(code, false);
    let r = if let Ok(parsed) = parsed {
        json!({ "imports": parsed })
    } else {
        json!({"error": parsed.err().unwrap().to_string()})
    };
    return serde_json::to_string(&r).unwrap();
}

#[cfg(feature = "bash-parser")]
#[wasm_bindgen]
pub fn parse_bash(code: &str) -> String {
    wrap_sig(windmill_parser_bash::parse_bash_sig(code))
}

#[cfg(feature = "bash-parser")]
#[wasm_bindgen]
pub fn parse_powershell(code: &str) -> String {
    wrap_sig(windmill_parser_bash::parse_powershell_sig(code))
}

#[cfg(feature = "go-parser")]
#[wasm_bindgen]
pub fn parse_go(code: &str) -> String {
    wrap_sig(windmill_parser_go::parse_go_sig(code))
}

#[cfg(feature = "py-parser")]
#[wasm_bindgen]
pub fn parse_python(code: &str, main_override: Option<String>) -> String {
    wrap_sig(windmill_parser_py::parse_python_signature(
        code,
        main_override,
        false,
    ))
}

#[cfg(feature = "sql-parser")]
#[wasm_bindgen]
pub fn parse_sql(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_pgsql_sig(code))
}

#[cfg(feature = "sql-parser")]
#[wasm_bindgen]
pub fn parse_mysql(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_mysql_sig(code))
}

#[cfg(feature = "sql-parser")]
#[wasm_bindgen]
pub fn parse_oracledb(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_oracledb_sig(code))
}

#[cfg(feature = "sql-parser")]
#[wasm_bindgen]
pub fn parse_duckdb(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_duckdb_sig(code))
}

#[cfg(feature = "sql-parser")]
#[wasm_bindgen]
pub fn parse_bigquery(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_bigquery_sig(code))
}

#[cfg(feature = "sql-parser")]
#[wasm_bindgen]
pub fn parse_snowflake(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_snowflake_sig(code))
}

#[cfg(feature = "sql-parser")]
#[wasm_bindgen]
pub fn parse_mssql(code: &str) -> String {
    wrap_sig(windmill_parser_sql::parse_mssql_sig(code))
}

#[cfg(feature = "sql-parser")]
#[wasm_bindgen]
pub fn parse_db_resource(code: &str) -> Option<String> {
    windmill_parser_sql::parse_db_resource(code)
}

#[cfg(feature = "graphql-parser")]
#[wasm_bindgen]
pub fn parse_graphql(code: &str) -> String {
    wrap_sig(windmill_parser_graphql::parse_graphql_sig(code))
}

#[cfg(feature = "php-parser")]
#[wasm_bindgen]
pub fn parse_php(code: &str) -> String {
    wrap_sig(windmill_parser_php::parse_php_signature(code, None))
}

#[cfg(feature = "rust-parser")]
#[wasm_bindgen]
pub fn parse_rust(code: &str) -> String {
    wrap_sig(windmill_parser_rust::parse_rust_signature(code))
}

#[cfg(feature = "ansible-parser")]
#[wasm_bindgen]
pub fn parse_ansible(code: &str) -> String {
    wrap_sig(windmill_parser_yaml::parse_ansible_sig(code))
}

#[cfg(feature = "csharp-parser")]
#[wasm_bindgen]
pub fn parse_csharp(code: &str) -> String {
    wrap_sig(windmill_parser_csharp::parse_csharp_signature(code))
}

#[cfg(feature = "nu-parser")]
#[wasm_bindgen]
pub fn parse_nu(code: &str) -> String {
    wrap_sig(windmill_parser_nu::parse_nu_signature(code))
}

#[cfg(feature = "java-parser")]
#[wasm_bindgen]
pub fn parse_java(code: &str) -> String {
    wrap_sig(windmill_parser_java::parse_java_signature(code))
}

#[cfg(feature = "ruby-parser")]
#[wasm_bindgen]
pub fn parse_ruby(code: &str) -> String {
    wrap_sig(windmill_parser_ruby::parse_ruby_signature(code))
}
#[cfg(feature = "sql-parser")]
#[wasm_bindgen]
pub fn parse_assets_sql(code: &str) -> String {
    if let Ok(r) = windmill_parser_sql::parse_assets(code) {
        return serde_json::to_string(&r).unwrap();
    } else {
        return "Invalid".to_string();
    }
}

#[cfg(feature = "ts-parser")]
#[wasm_bindgen]
pub fn parse_assets_ts(code: &str) -> String {
    if let Ok(r) = windmill_parser_ts::parse_assets(code) {
        return serde_json::to_string(&r).unwrap();
    } else {
        return "Invalid".to_string();
    }
}

#[cfg(feature = "py-parser")]
#[wasm_bindgen]
pub fn parse_assets_py(code: &str) -> String {
    if let Ok(r) = windmill_parser_py::parse_assets(code) {
        return serde_json::to_string(&r).unwrap();
    } else {
        return "Invalid".to_string();
    }
}

#[cfg(feature = "ruby-parser")]
#[wasm_bindgen]
pub fn parse_assets_ruby(code: &str) -> String {
    if let Ok(r) = windmill_parser_ruby::parse_assets(code) {
        return serde_json::to_string(&r).unwrap();
    } else {
        return "Invalid".to_string();
    }
}

// for related places search: ADD_NEW_LANG
