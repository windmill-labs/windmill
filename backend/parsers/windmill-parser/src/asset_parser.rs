use serde::Serialize;

#[derive(Serialize, PartialEq, Clone, Copy)]
#[serde(rename_all(serialize = "lowercase"))]
pub enum AssetUsageAccessType {
    R,
    W,
    RW,
}

use AssetUsageAccessType::*;

#[derive(Serialize, PartialEq, Clone, Copy)]
#[serde(rename_all(serialize = "lowercase"))]
pub enum AssetKind {
    S3Object,
    Resource,
    Variable,
}

#[derive(Serialize)]
pub struct ParseAssetsResult<S: AsRef<str>> {
    pub kind: AssetKind,
    pub path: S,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_type: Option<AssetUsageAccessType>, // None in case of ambiguity
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
    } else if s.starts_with("var://") {
        Some((AssetKind::Variable, &s[6..]))
    } else {
        None
    }
}
