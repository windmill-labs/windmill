use serde::{Deserialize, Serialize};
use windmill_parser::asset_parser::ParseAssetsResult;

use crate::scripts::ScriptLang;

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetKind {
    S3Object,
    Resource,
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
