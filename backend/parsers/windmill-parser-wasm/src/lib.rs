use serde_json;
use wasm_bindgen::prelude::*;
use windmill_parser::MainArgSignature;

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
pub fn parse_bash(code: &str) -> String {
    wrap_sig(windmill_parser_bash::parse_bash_sig(code))
}

#[wasm_bindgen]
pub fn parse_go(code: &str) -> String {
    wrap_sig(windmill_parser_go::parse_go_sig(code))
}

#[wasm_bindgen]
pub fn parse_python(code: &str) -> String {
    wrap_sig(windmill_parser_py::parse_python_signature(code))
}
