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
pub struct ParseAssetsResult {
    pub kind: AssetKind,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_type: Option<AssetUsageAccessType>, // None in case of ambiguity
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
        } else {
            arr.push(asset);
        }
    }
    arr.sort_by(|a, b| a.path.cmp(&b.path));
    arr
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
