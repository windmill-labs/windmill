use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use std::collections::BTreeMap;

use crate::{error, scripts::ScriptHash};

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
    pub alt_access_type: Option<AssetUsageAccessType>,
    /// Map of column name to access type for column-level access tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<BTreeMap<String, AssetUsageAccessType>>,
}

pub async fn insert_static_asset_usage<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    asset: &AssetWithAltAccessType,
    usage_path: &str,
    usage_kind: AssetUsageKind,
) -> error::Result<()> {
    // Convert columns BTreeMap to JSONB format
    let columns_json = asset
        .columns
        .as_ref()
        .map(|cols| serde_json::to_value(cols).unwrap_or(serde_json::Value::Null));

    sqlx::query!(
        r#"INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind, columns)
                VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING"#,
        workspace_id,
        asset.path,
        asset.kind as AssetKind,
        (asset.access_type.or(asset.alt_access_type)) as Option<AssetUsageAccessType>,
        usage_path,
        usage_kind as AssetUsageKind,
        columns_json as Option<serde_json::Value>
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn clear_static_asset_usage<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    usage_path: &str,
    usage_kind: AssetUsageKind,
) -> error::Result<()> {
    sqlx::query!(
        r#"DELETE FROM asset WHERE workspace_id = $1 AND usage_path = $2 AND usage_kind = $3"#,
        workspace_id,
        usage_path,
        usage_kind as AssetUsageKind
    )
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn clear_static_asset_usage_by_script_hash<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    script_hash: ScriptHash,
) -> error::Result<()> {
    sqlx::query!(
        "DELETE FROM asset WHERE workspace_id = $1 AND usage_kind = 'script' AND usage_path = (SELECT path FROM script WHERE hash = $2 AND workspace_id = $1)",
        workspace_id,
        script_hash.0
    )
    .execute(executor)
    .await?;
    Ok(())
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

impl From<windmill_parser::asset_parser::AssetKind> for AssetKind {
    fn from(parser_kind: windmill_parser::asset_parser::AssetKind) -> Self {
        match parser_kind {
            windmill_parser::asset_parser::AssetKind::S3Object => AssetKind::S3Object,
            windmill_parser::asset_parser::AssetKind::Resource => AssetKind::Resource,
            windmill_parser::asset_parser::AssetKind::Ducklake => AssetKind::Ducklake,
            windmill_parser::asset_parser::AssetKind::DataTable => AssetKind::DataTable,
        }
    }
}

impl From<windmill_parser::asset_parser::AssetUsageAccessType> for AssetUsageAccessType {
    fn from(parser_kind: windmill_parser::asset_parser::AssetUsageAccessType) -> Self {
        match parser_kind {
            windmill_parser::asset_parser::AssetUsageAccessType::R => AssetUsageAccessType::R,
            windmill_parser::asset_parser::AssetUsageAccessType::W => AssetUsageAccessType::W,
            windmill_parser::asset_parser::AssetUsageAccessType::RW => AssetUsageAccessType::RW,
        }
    }
}
