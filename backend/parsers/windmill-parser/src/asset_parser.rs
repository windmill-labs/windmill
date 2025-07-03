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
}

#[derive(Serialize)]
pub struct ParseAssetsResult<'a> {
    pub kind: AssetKind,
    pub path: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_type: Option<AssetUsageAccessType>, // None in case of ambiguity
}

pub fn merge_assets<'a>(assets: Vec<ParseAssetsResult<'a>>) -> Vec<ParseAssetsResult<'a>> {
    let mut arr: Vec<ParseAssetsResult<'a>> = vec![];
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
    arr.sort_by_key(|a| a.path);
    arr
}
