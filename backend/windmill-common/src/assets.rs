use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use windmill_parser::asset_parser::ParseAssetsResult;

use crate::{error, scripts::ScriptLang};

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetKind {
    S3Object,
    Resource,
    Variable,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_USAGE_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetUsageKind {
    Script,
    Flow,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_ACCESS_TYPE", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetUsageAccessType {
    R,
    W,
    RW,
}

pub struct Asset {
    pub path: String,
    pub kind: AssetKind,
}

pub struct AssetUsage {
    pub path: String,
    pub kind: AssetUsageKind,
    pub access_type: AssetUsageAccessType,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct AssetWithAccessType {
    pub path: String,
    pub kind: AssetKind,
    pub access_type: Option<AssetUsageAccessType>,
}

pub fn parse_assets(
    input: &str,
    lang: ScriptLang,
) -> anyhow::Result<Option<Vec<ParseAssetsResult<String>>>> {
    let r = match lang {
        ScriptLang::Python3 => windmill_parser_py::parse_assets(input),
        ScriptLang::DuckDb => windmill_parser_sql::parse_assets(input).map(|a| {
            a.iter()
                .map(|a| ParseAssetsResult {
                    path: a.path.to_string(),
                    access_type: a.access_type,
                    kind: a.kind,
                })
                .collect()
        }),
        ScriptLang::Deno | ScriptLang::Bun | ScriptLang::Nativets => {
            windmill_parser_ts::parse_assets(input)
        }
        _ => return Ok(None),
    };
    return r.map(Some);
}

impl From<windmill_parser::asset_parser::AssetKind> for AssetKind {
    fn from(kind: windmill_parser::asset_parser::AssetKind) -> Self {
        match kind {
            windmill_parser::asset_parser::AssetKind::S3Object => AssetKind::S3Object,
            windmill_parser::asset_parser::AssetKind::Resource => AssetKind::Resource,
            windmill_parser::asset_parser::AssetKind::Variable => AssetKind::Variable,
        }
    }
}
impl From<windmill_parser::asset_parser::AssetUsageAccessType> for AssetUsageAccessType {
    fn from(access_type: windmill_parser::asset_parser::AssetUsageAccessType) -> Self {
        match access_type {
            windmill_parser::asset_parser::AssetUsageAccessType::R => AssetUsageAccessType::R,
            windmill_parser::asset_parser::AssetUsageAccessType::W => AssetUsageAccessType::W,
            windmill_parser::asset_parser::AssetUsageAccessType::RW => AssetUsageAccessType::RW,
        }
    }
}
impl<S> From<windmill_parser::asset_parser::ParseAssetsResult<S>> for AssetWithAccessType
where
    S: AsRef<str> + Into<String>,
{
    fn from(asset: windmill_parser::asset_parser::ParseAssetsResult<S>) -> Self {
        AssetWithAccessType {
            access_type: asset.access_type.map(Into::into),
            kind: asset.kind.into(),
            path: asset.path.into(),
        }
    }
}

pub async fn insert_asset_usage<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    asset: &AssetWithAccessType,
    usage_path: &str,
    usage_kind: AssetUsageKind,
) -> error::Result<()> {
    sqlx::query!(
        r#"INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind)
                VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING"#,
        workspace_id,
        asset.path,
        asset.kind as AssetKind,
        asset.access_type as Option<AssetUsageAccessType>,
        usage_path,
        usage_kind as AssetUsageKind
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn clear_asset_usage<'e>(
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
