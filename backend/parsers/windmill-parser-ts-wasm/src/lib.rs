use serde_json;
use wasm_bindgen::prelude::*;
use windmill_parser_ts::parse_deno_signature;

#[wasm_bindgen]
pub fn parse_deno_wasm(code: &str) -> String {
    let r = parse_deno_signature(code, false);
    if let Ok(r) = r {
        return serde_json::to_string(&r).unwrap();
    } else {
        return "{\"type\": \"Invalid\"}".to_string();
    }
}
