//! Server-side asset inference at script deploy.
//!
//! The `asset` rows written at deploy drive the asset-trigger cascade
//! (`fetch_producer_writes` in windmill-queue). Historically they came solely
//! from the client-supplied `NewScript.assets`, so a client with broken
//! inference (e.g. a failed wasm load) deploying `assets: null` silently
//! killed the producer side of the cascade while the subscriber side (parsed
//! server-side from `// on` annotations) kept looking wired. This module makes
//! asset *presence* server-authoritative by re-parsing the deployed content
//! with the same parsers the frontend wasm builds wrap.
//!
//! Merge semantics are a union: server-parsed assets are always present;
//! client entries are kept too (they may carry `alt_access_type` — the user's
//! manual access-type override — or come from client-side detection paths the
//! server has no parser for). For duplicate `(kind, path)` keys the server's
//! parser-derived fields win and the client's `alt_access_type` is preserved.
//! Languages without a server parser (or whose parse fails) fall back to the
//! client list unchanged, matching the previous behavior.

use std::collections::BTreeMap;

use windmill_common::{
    assets::{
        asset_access_type_from_parser, asset_kind_from_parser, AssetKind, AssetUsageAccessType,
        AssetWithAltAccessType,
    },
    scripts::ScriptLang,
};

/// Mirror of the frontend `inferAssets` language dispatch (infer.ts): only
/// these languages have a body-asset parser. Returns `None` for unsupported
/// languages or on parse failure — callers then keep the client-supplied list.
fn parse_assets_for_lang(
    lang: &ScriptLang,
    content: &str,
) -> Option<Vec<windmill_parser::asset_parser::ParseAssetsResult>> {
    let parsed = match lang {
        ScriptLang::DuckDb => windmill_parser_sql_asset::parse_assets(content),
        ScriptLang::Bun | ScriptLang::Deno | ScriptLang::Nativets => {
            windmill_parser_ts_asset::parse_assets(content)
        }
        #[cfg(feature = "python")]
        ScriptLang::Python3 => windmill_parser_py_asset::parse_assets(content),
        ScriptLang::Ansible => windmill_parser_yaml::parse_assets(content),
        _ => return None,
    };
    match parsed {
        Ok(out) => Some(out.assets),
        Err(e) => {
            tracing::warn!(
                "server-side asset inference failed for a {} script; falling back to \
                 client-supplied assets: {e:#}",
                lang.as_str()
            );
            None
        }
    }
}

/// Mirror of the frontend `getCommentPrefix` (infer.ts) — the languages whose
/// leading comment block is scanned for `volume: <path>` annotations.
fn comment_prefix(lang: &ScriptLang) -> Option<&'static str> {
    match lang {
        ScriptLang::Python3
        | ScriptLang::Bash
        | ScriptLang::Powershell
        | ScriptLang::Ansible
        | ScriptLang::Ruby
        | ScriptLang::Rlang => Some("#"),
        ScriptLang::Deno
        | ScriptLang::Bun
        | ScriptLang::Bunnative
        | ScriptLang::Nativets
        | ScriptLang::Go => Some("//"),
        _ => None,
    }
}

/// Mirror of the frontend `parseVolumeAnnotations` (infer.ts): `<prefix>
/// volume: <path>` lines in the leading comment block, each an `rw` volume
/// asset. Scanning stops at the first non-comment line (blank lines are
/// skipped), exactly like the frontend.
fn parse_volume_annotations(content: &str, prefix: &str) -> Vec<AssetWithAltAccessType> {
    let mut volumes = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(after) = trimmed.strip_prefix(prefix) else {
            break;
        };
        let after = after.trim();
        if let Some(rest) = after.strip_prefix("volume:") {
            if let Some(path) = rest.trim().split_whitespace().next() {
                volumes.push(AssetWithAltAccessType {
                    path: path.to_string(),
                    kind: AssetKind::Volume,
                    access_type: Some(AssetUsageAccessType::RW),
                    alt_access_type: None,
                    columns: None,
                });
            }
        }
    }
    volumes
}

/// Parse the deployed content into asset usages, mirroring the frontend's
/// `inferAssets` (body parser per language + volume annotations). `None`
/// means "no server parser produced anything for this language" — distinct
/// from `Some(vec![])`, which is an authoritative "this script uses no
/// assets".
fn infer_script_assets(lang: &ScriptLang, content: &str) -> Option<Vec<AssetWithAltAccessType>> {
    let body_assets = parse_assets_for_lang(lang, content);
    let volume_assets = comment_prefix(lang)
        .map(|p| parse_volume_annotations(content, p))
        .unwrap_or_default();
    if body_assets.is_none() && volume_assets.is_empty() {
        return None;
    }
    let mut out: Vec<AssetWithAltAccessType> = body_assets
        .unwrap_or_default()
        .into_iter()
        .map(|a| AssetWithAltAccessType {
            path: a.path,
            kind: asset_kind_from_parser(a.kind),
            access_type: a.access_type.map(asset_access_type_from_parser),
            alt_access_type: None,
            columns: a.columns.map(|cols| {
                cols.into_iter()
                    .map(|(k, v)| (k, asset_access_type_from_parser(v)))
                    .collect::<BTreeMap<_, _>>()
            }),
        })
        .collect();
    out.extend(volume_assets);
    Some(out)
}

/// The asset list to persist at deploy: server-parsed assets unioned with the
/// client-supplied ones. See the module docs for the exact semantics.
pub fn effective_script_assets(
    lang: &ScriptLang,
    content: &str,
    client_assets: Option<Vec<AssetWithAltAccessType>>,
) -> Option<Vec<AssetWithAltAccessType>> {
    let Some(inferred) = infer_script_assets(lang, content) else {
        return client_assets;
    };
    let mut merged: Vec<AssetWithAltAccessType> = inferred;
    for client in client_assets.into_iter().flatten() {
        if let Some(existing) = merged
            .iter_mut()
            .find(|a| a.kind == client.kind && a.path == client.path)
        {
            // Server parse wins for parser-derived fields; the client's
            // alt_access_type is the user's manual override — keep it.
            if existing.alt_access_type.is_none() {
                existing.alt_access_type = client.alt_access_type;
            }
        } else {
            merged.push(client);
        }
    }
    Some(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(
        kind: AssetKind,
        path: &str,
        access: Option<AssetUsageAccessType>,
    ) -> AssetWithAltAccessType {
        AssetWithAltAccessType {
            path: path.to_string(),
            kind,
            access_type: access,
            alt_access_type: None,
            columns: None,
        }
    }

    // The bug class this module exists for: client deploys with no assets,
    // but the content demonstrably writes one — the server parse must
    // produce the producer row anyway.
    #[test]
    fn duckdb_write_survives_empty_client_assets() {
        let content = "ATTACH 'datatable://main' AS pg;\n\
                       CREATE TABLE IF NOT EXISTS pg.out_table AS\n\
                       SELECT * FROM (SELECT 1 AS placeholder);";
        let got = effective_script_assets(&ScriptLang::DuckDb, content, None).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].kind, AssetKind::DataTable);
        assert_eq!(got[0].path, "main/out_table");
        assert_eq!(got[0].access_type, Some(AssetUsageAccessType::W));
    }

    #[test]
    fn client_alt_access_type_is_preserved_on_match() {
        let content = "ATTACH 'datatable://main' AS pg;\n\
                       CREATE TABLE IF NOT EXISTS pg.out_table AS SELECT 1;";
        let mut client = asset(AssetKind::DataTable, "main/out_table", None);
        client.alt_access_type = Some(AssetUsageAccessType::RW);
        let got =
            effective_script_assets(&ScriptLang::DuckDb, content, Some(vec![client])).unwrap();
        assert_eq!(got.len(), 1);
        // Parser-derived access wins; the user's alt override rides along.
        assert_eq!(got[0].access_type, Some(AssetUsageAccessType::W));
        assert_eq!(got[0].alt_access_type, Some(AssetUsageAccessType::RW));
    }

    #[test]
    fn client_only_entries_are_kept() {
        let content = "ATTACH 'datatable://main' AS pg;\n\
                       CREATE TABLE IF NOT EXISTS pg.out_table AS SELECT 1;";
        let extra = asset(
            AssetKind::S3Object,
            "bucket/file.parquet",
            Some(AssetUsageAccessType::R),
        );
        let got = effective_script_assets(&ScriptLang::DuckDb, content, Some(vec![extra])).unwrap();
        assert_eq!(got.len(), 2);
        assert!(got.iter().any(|a| a.kind == AssetKind::S3Object));
    }

    #[test]
    fn unsupported_language_falls_back_to_client() {
        let client = vec![asset(
            AssetKind::S3Object,
            "b/f.json",
            Some(AssetUsageAccessType::W),
        )];
        let got = effective_script_assets(&ScriptLang::Go, "package main", Some(client.clone()));
        assert_eq!(got.map(|v| v.len()), Some(1));
        assert_eq!(
            effective_script_assets(&ScriptLang::Go, "package main", None).is_none(),
            true
        );
    }

    #[test]
    fn volume_annotations_parsed_from_leading_comment_block() {
        let content = "// volume: my_vol\n// some other comment\nconsole.log(1)\n// volume: ignored_after_code\n";
        let got = effective_script_assets(&ScriptLang::Bun, content, None).unwrap();
        let vols: Vec<_> = got.iter().filter(|a| a.kind == AssetKind::Volume).collect();
        assert_eq!(vols.len(), 1);
        assert_eq!(vols[0].path, "my_vol");
        assert_eq!(vols[0].access_type, Some(AssetUsageAccessType::RW));
    }
}
