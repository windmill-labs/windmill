use std::collections::HashMap;

use serde_json::{value::RawValue, Value};
use windmill_parser::asset_parser::parse_asset_syntax;

use crate::assets::AssetKind;

/// Represents a runtime-detected asset from job arguments
#[derive(Debug, Clone)]
pub struct RuntimeAsset {
    pub path: String,
    pub kind: AssetKind,
}

/// Convert from parser AssetKind to common AssetKind
fn convert_asset_kind(parser_kind: windmill_parser::asset_parser::AssetKind) -> AssetKind {
    match parser_kind {
        windmill_parser::asset_parser::AssetKind::S3Object => AssetKind::S3Object,
        windmill_parser::asset_parser::AssetKind::Resource => AssetKind::Resource,
        windmill_parser::asset_parser::AssetKind::Ducklake => AssetKind::Ducklake,
        windmill_parser::asset_parser::AssetKind::DataTable => AssetKind::DataTable,
    }
}

/// Extract assets from job arguments by analyzing the JSON values
///
/// This function detects assets in two ways:
/// 1. String values matching asset syntax (e.g., "s3:///path/to/file")
/// 2. Objects with an "s3" field containing a string path
pub fn extract_runtime_assets_from_args(
    args: &HashMap<String, Box<RawValue>>,
) -> Vec<RuntimeAsset> {
    let mut assets = Vec::new();

    for (_key, value) in args.iter() {
        extract_assets_from_value(value, &mut assets);
    }

    assets
}

fn extract_assets_from_value(value: &Box<RawValue>, assets: &mut Vec<RuntimeAsset>) {
    match get_asset_heuristic(value) {
        Some((kind, path)) => {
            assets.push(RuntimeAsset { path, kind });
        }
        None => {}
    }
}

// Try to guess if a raw value is an asset before parsing it in case it's big
fn get_asset_heuristic(value: &Box<RawValue>) -> Option<(AssetKind, String)> {
    let json = value.get();
    if json.len() < 256 && json.trim_start().starts_with('"') {
        // parse_asset_syntax is super cheap, ensure the string starts with an asset scheme before parsing
        let _ = parse_asset_syntax(json.trim_matches('"'), false)?;
        let s = serde_json::from_str::<String>(value.get()).ok()?;
        let (kind, path) = parse_asset_syntax(&s, false)?;
        return Some((convert_asset_kind(kind), path.to_string()));
    }

    if json.len() < 256 && json.trim_start().starts_with('{') && json.contains("\"s3\"") {
        let s = serde_json::from_str::<Value>(value.get()).ok()?;
        match s.get("s3") {
            Some(Value::String(s3_path)) => {
                let storage = s.get("storage").and_then(|v| v.as_str()).unwrap_or("");
                let asset_path = format!("{}/{}", storage, s3_path);
                return Some((AssetKind::S3Object, asset_path));
            }
            _ => {}
        }
    }

    None
}
