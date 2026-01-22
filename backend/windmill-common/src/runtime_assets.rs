use std::collections::HashMap;

use serde_json::{value::RawValue, Value};
use windmill_parser::asset_parser::{parse_asset_syntax, ASSET_KINDS};

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
        extract_assets_from_raw_value(value, &mut assets).unwrap_or(());
    }

    assets
}

fn extract_assets_from_raw_value(
    value: &Box<RawValue>,
    assets: &mut Vec<RuntimeAsset>,
) -> Option<()> {
    let json = value.get().trim_start();
    if json.len() < 256 && json.len() > 2 && json.starts_with('"') {
        // Ensure the string starts with an asset scheme before parsing
        let prefix = ASSET_KINDS
            .iter()
            .any(|(prefix, _)| json[1..].starts_with(prefix));
        if prefix {
            let s = serde_json::from_str::<String>(value.get()).ok()?;
            let (kind, path) = parse_asset_syntax(&s, false)?;
            let kind = convert_asset_kind(kind);
            assets.push(RuntimeAsset { path: path.to_string(), kind });
        }
    } else if json.len() < 256 && json.starts_with('{') && json.contains("\"s3\"") {
        let s = serde_json::from_str::<Value>(value.get()).ok()?;
        let (kind, path) = parse_s3_json_object(&s)?;
        assets.push(RuntimeAsset { path, kind });
    }
    None
}

fn parse_s3_json_object(value: &Value) -> Option<(AssetKind, String)> {
    match value.get("s3") {
        Some(Value::String(s3_path)) => {
            let storage = value.get("storage").and_then(|v| v.as_str()).unwrap_or("");
            let asset_path = format!("{}/{}", storage, s3_path);
            Some((AssetKind::S3Object, asset_path))
        }
        _ => None,
    }
}
