use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(
    Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type, PartialOrd, Ord,
)]
#[sqlx(type_name = "ASSET_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetKind {
    S3Object,
    Resource,
    // Avoid unnexpected crashes when deserializing old assets
    Variable, // Deprecated
    Ducklake,
    DataTable,
}

#[derive(
    Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type, PartialOrd, Ord,
)]
#[sqlx(type_name = "ASSET_USAGE_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetUsageKind {
    Script,
    Flow,
    Job,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_ACCESS_TYPE", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetUsageAccessType {
    R,
    W,
    RW,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct AssetWithAltAccessType {
    pub path: String,
    pub kind: AssetKind,
    pub access_type: Option<AssetUsageAccessType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_access_type: Option<AssetUsageAccessType>,
    /// Map of column name to access type for column-level access tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<BTreeMap<String, AssetUsageAccessType>>,
}

pub fn merge_asset_usage_access_types(
    a: Option<AssetUsageAccessType>,
    b: Option<AssetUsageAccessType>,
) -> Option<AssetUsageAccessType> {
    use AssetUsageAccessType::*;
    match (a, b) {
        (None, _) | (_, None) => None,
        (Some(R), Some(W)) | (Some(W), Some(R)) => Some(RW),
        (Some(RW), _) | (_, Some(RW)) => Some(RW),
        (Some(R), Some(R)) => Some(R),
        (Some(W), Some(W)) => Some(W),
    }
}

pub fn merge_asset_columns(
    a: &Option<BTreeMap<String, AssetUsageAccessType>>,
    b: &Option<BTreeMap<String, AssetUsageAccessType>>,
) -> Option<BTreeMap<String, AssetUsageAccessType>> {
    match (a, b) {
        (None, None) => None,
        (Some(cols), None) | (None, Some(cols)) => Some(cols.clone()),
        (Some(cols_a), Some(cols_b)) => {
            let mut merged = cols_a.clone();
            for (col, access_b) in cols_b {
                let access_a = merged.get(col);
                let merged_access =
                    merge_asset_usage_access_types(access_a.cloned(), Some(*access_b));
                if let Some(access) = merged_access {
                    merged.insert(col.clone(), access);
                }
            }
            Some(merged)
        }
    }
}
