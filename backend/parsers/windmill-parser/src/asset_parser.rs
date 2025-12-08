use serde::Serialize;

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

#[derive(Serialize, Debug, PartialEq)]
pub struct ParseAssetsResult<S: AsRef<str>> {
    pub kind: AssetKind,
    pub path: S,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_type: Option<AssetUsageAccessType>, // None in case of ambiguity
}

#[derive(Debug, Clone, Serialize)]
pub struct DelegateToGitRepoDetails {
    pub resource: String,
    pub playbook: Option<String>,
    pub commit: Option<String>,
}

pub fn merge_assets<S: AsRef<str>>(assets: Vec<ParseAssetsResult<S>>) -> Vec<ParseAssetsResult<S>> {
    let mut arr: Vec<ParseAssetsResult<S>> = vec![];
    for asset in assets {
        // Remove duplicates
        if let Some(existing) = arr
            .iter_mut()
            .find(|x| x.path.as_ref() == asset.path.as_ref() && x.kind == asset.kind)
        {
            // merge access types
            existing.access_type = match (asset.access_type, existing.access_type) {
                (None, _) | (_, None) => None,
                (Some(R), Some(W)) | (Some(W), Some(R)) => Some(RW),
                (Some(RW), _) | (_, Some(RW)) => Some(RW),
                (Some(R), Some(R)) => Some(R),
                (Some(W), Some(W)) => Some(W),
            };
        } else {
            arr.push(asset);
        }
    }
    arr.sort_by(|a, b| a.path.as_ref().cmp(b.path.as_ref()));
    arr
}

pub fn parse_asset_syntax(s: &str) -> Option<(AssetKind, &str)> {
    if s.starts_with("s3://") {
        Some((AssetKind::S3Object, &s[5..]))
    } else if s.starts_with("res://") {
        Some((AssetKind::Resource, &s[6..]))
    } else if s.starts_with("$res:") {
        Some((AssetKind::Resource, &s[5..]))
    } else if s.starts_with("ducklake://") {
        Some((AssetKind::Ducklake, &s[11..]))
    } else if s.starts_with("datatable://") {
        Some((AssetKind::DataTable, &s[11..]))
    } else {
        None
    }
}

pub fn detect_sql_access_type(sql: &str) -> Option<AssetUsageAccessType> {
    let first_kw = sql
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();

    // Check for write operations
    let has_write = first_kw.starts_with("insert")
        || first_kw.starts_with("update")
        || first_kw.starts_with("delete")
        || first_kw.starts_with("drop")
        || first_kw.starts_with("create")
        || first_kw.starts_with("alter")
        || first_kw.starts_with("truncate")
        || first_kw.starts_with("merge");

    // Check for read operations
    let has_read = first_kw.starts_with("select")
        || first_kw.starts_with("with")  // CTEs, usually for reads
        || first_kw.starts_with("show")
        || first_kw.starts_with("describe")
        || first_kw.starts_with("explain");

    match (has_read, has_write) {
        (true, true) => Some(RW),
        (true, false) => Some(R),
        (false, true) => Some(W),
        (false, false) => None, // Unknown - couldn't determine
    }
}
