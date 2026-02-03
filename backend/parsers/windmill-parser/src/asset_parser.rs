use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, PartialEq, Clone, Copy, Debug)]
#[serde(rename_all(serialize = "lowercase"))]
pub enum AssetUsageAccessType {
    R,
    W,
    RW,
}

use AssetUsageAccessType::*;

#[derive(Serialize, PartialEq, Clone, Copy, Debug)]
#[serde(rename_all(serialize = "lowercase"))]
pub enum AssetKind {
    S3Object,
    Resource,
    Ducklake,
    DataTable,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct ParseAssetsResult {
    pub kind: AssetKind,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_type: Option<AssetUsageAccessType>, // None in case of ambiguity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<HashMap<String, AssetUsageAccessType>>, // Map column name to access type, "*" represents wildcard
}

#[derive(Serialize, Debug, PartialEq)]
pub struct SqlQueryDetails {
    pub query_string: String, // SQL query with $1 placeholders for interpolations
    pub span: (u32, u32),     // (start, end) byte positions in source code
    pub source_kind: AssetKind, // DataTable or Ducklake
    pub source_name: String,  // e.g., "main", "dt"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_schema: Option<String>, // e.g., Some("public"), None
}

#[derive(Serialize, Debug, Default)]
pub struct ParseAssetsOutput {
    pub assets: Vec<ParseAssetsResult>,
    pub sql_queries: Vec<SqlQueryDetails>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DelegateToGitRepoDetails {
    pub resource: String,
    pub playbook: Option<String>,
    pub commit: Option<String>,
}

pub fn merge_assets(assets: Vec<ParseAssetsResult>) -> Vec<ParseAssetsResult> {
    let mut arr: Vec<ParseAssetsResult> = vec![];
    for asset in assets {
        // Remove duplicates
        if let Some(existing) = arr
            .iter_mut()
            .find(|x| x.path == asset.path && x.kind == asset.kind)
        {
            // merge access types
            existing.access_type = match (asset.access_type, existing.access_type) {
                (None, _) | (_, None) => None,
                (Some(R), Some(W)) | (Some(W), Some(R)) => Some(RW),
                (Some(RW), _) | (_, Some(RW)) => Some(RW),
                (Some(R), Some(R)) => Some(R),
                (Some(W), Some(W)) => Some(W),
            };
            // merge columns: union the column sets and merge access types per column
            existing.columns = merge_column_maps(existing.columns.take(), asset.columns);
        } else {
            arr.push(asset);
        }
    }
    arr.sort_by(|a, b| a.path.cmp(&b.path));
    arr
}

fn merge_column_maps(
    existing: Option<HashMap<String, AssetUsageAccessType>>,
    new: Option<HashMap<String, AssetUsageAccessType>>,
) -> Option<HashMap<String, AssetUsageAccessType>> {
    match (existing, new) {
        (None, None) => None,
        (Some(map), None) | (None, Some(map)) => Some(map),
        (Some(mut existing_map), Some(new_map)) => {
            for (col_name, new_access) in new_map {
                existing_map
                    .entry(col_name)
                    .and_modify(|existing_access| {
                        *existing_access = merge_access_types(*existing_access, new_access);
                    })
                    .or_insert(new_access);
            }
            Some(existing_map)
        }
    }
}

fn merge_access_types(a: AssetUsageAccessType, b: AssetUsageAccessType) -> AssetUsageAccessType {
    match (a, b) {
        (R, W) | (W, R) => RW,
        (RW, _) | (_, RW) => RW,
        (R, R) => R,
        (W, W) => W,
    }
}

// Will return false if the user assigned an asset to a variable like:
//   let sql = wmill.datatable('main')
// But never used it. In that case we don't know which table is being used,
// but we still want to add the main datatable as an asset with unknown access type.
//
// This function takes care of the fact that assets can be suffixed (e.g. "main/users")
pub fn asset_was_used(assets: &Vec<ParseAssetsResult>, (kind, path): (AssetKind, &String)) -> bool {
    assets.iter().any(|a| {
        let a_path = a.path.as_str();
        let has_same_path_base = a_path
            .strip_prefix(path)
            .map(|p| p.starts_with('/'))
            .unwrap_or(false);
        (has_same_path_base || a_path == path) && a.kind == kind
    })
}

pub fn parse_asset_syntax(s: &str, enable_default_syntax: bool) -> Option<(AssetKind, &str)> {
    if enable_default_syntax && s == "datatable" {
        Some((AssetKind::DataTable, "main"))
    } else if enable_default_syntax && s == "ducklake" {
        Some((AssetKind::Ducklake, "main"))
    } else if s.starts_with("s3://") {
        Some((AssetKind::S3Object, &s[5..]))
    } else if s.starts_with("res://") {
        Some((AssetKind::Resource, &s[6..]))
    } else if s.starts_with("$res:") {
        Some((AssetKind::Resource, &s[5..]))
    } else if s.starts_with("ducklake://") {
        Some((AssetKind::Ducklake, &s[11..]))
    } else if s.starts_with("datatable://") {
        Some((AssetKind::DataTable, &s[12..]))
    } else {
        None
    }
}
