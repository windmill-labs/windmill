use serde::Serialize;
use std::collections::BTreeMap;

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
    Volume,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct ParseAssetsResult {
    pub kind: AssetKind,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_type: Option<AssetUsageAccessType>, // None in case of ambiguity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<BTreeMap<String, AssetUsageAccessType>>, // Map column name to access type, "*" represents wildcard
}

#[derive(Serialize, Debug, PartialEq)]
pub struct SqlQueryDetails {
    pub query_string: String, // SQL query with $1 placeholders for interpolations
    pub span: (u32, u32),     // (start, end) byte positions in source code
    pub source_kind: AssetKind, // DataTable or Ducklake
    pub source_name: String,  // e.g., "main", "dt"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_schema: Option<String>, // e.g., Some("public"), None
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub has_raw_interpolation: bool, // true if any ${sql.raw(...)} was used
}

#[derive(Serialize, Debug, Default)]
pub struct ParseAssetsOutput {
    pub assets: Vec<ParseAssetsResult>,
    pub sql_queries: Vec<SqlQueryDetails>,
    // Bare `// materialize` (or `#` / `--`) anywhere in the source — opt-in
    // marker that sets auto_kind='materializer' and includes the script in
    // its folder's pipeline. Does NOT declare what is materialized: the
    // parser-detected `w`/`rw` usages in `assets` are the outputs.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub is_materializer: bool,
    // `// on <trigger-spec>` annotations — execution DAG edges. Each is an
    // independent OR (any fires the script). Empty = script has no
    // automatic triggers (still runnable manually / via existing cron
    // triggers).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub triggers: Vec<TriggerSpec>,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TriggerSpec {
    // Refresh when `<asset>` changes. Kind comes from parse_asset_syntax so
    // it matches the `asset` table.
    Asset { asset_kind: AssetKind, path: String },
    // Refresh on cron. The raw expression is passed through as-is so the
    // existing schedule subsystem can validate it.
    Schedule { cron: String },
    // `// on <kind> <path>` style. The `path` is a workspace-relative
    // reference to a trigger row already configured in the corresponding
    // trigger table (http_trigger, email_trigger, kafka_trigger, …). Keeps
    // the annotation terse; auth/broker/topic details live in the trigger's
    // own UI.
    Webhook { path: String },
    Email { path: String },
    Kafka { path: String },
    Mqtt { path: String },
    Nats { path: String },
    Postgres { path: String },
    Sqs { path: String },
    Gcp { path: String },
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
    existing: Option<BTreeMap<String, AssetUsageAccessType>>,
    new: Option<BTreeMap<String, AssetUsageAccessType>>,
) -> Option<BTreeMap<String, AssetUsageAccessType>> {
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
// This function takes care of the fact that assets can be suffixed (e.g. "main/users" or "u/user/resource?table=table1")
pub fn asset_was_used(assets: &Vec<ParseAssetsResult>, (kind, path): (AssetKind, &String)) -> bool {
    assets.iter().any(|a| {
        let a_path = a.path.as_str();
        // Check for /table suffix (Ducklake, DataTable) or ?table= suffix (Resource)
        let has_same_path_base = a_path
            .strip_prefix(path)
            .map(|p| p.starts_with('/') || p.starts_with('?'))
            .unwrap_or(false);
        (has_same_path_base || a_path == path) && a.kind == kind
    })
}

pub fn parse_asset_syntax(s: &str, enable_default_syntax: bool) -> Option<(AssetKind, &str)> {
    if enable_default_syntax && s == "datatable" {
        return Some((AssetKind::DataTable, "main"));
    } else if enable_default_syntax && s == "ducklake" {
        return Some((AssetKind::Ducklake, "main"));
    }
    for (prefix, kind) in ASSET_KINDS.iter() {
        if s.starts_with(prefix) {
            let path = &s[prefix.len()..];
            return Some((*kind, path));
        }
    }
    None
}

pub const ASSET_KINDS: &[(&str, AssetKind)] = &[
    ("s3://", AssetKind::S3Object),
    ("res://", AssetKind::Resource),
    ("$res:", AssetKind::Resource),
    ("ducklake://", AssetKind::Ducklake),
    ("datatable://", AssetKind::DataTable),
    ("volume://", AssetKind::Volume),
];

// Scan raw source for pipeline annotations. Language-agnostic: any line
// whose first non-whitespace tokens are a comment prefix (`//`, `#`, or
// `--`) followed by either:
//   - bare `materialize` (no arguments)  → is_materializer = true
//   - `on <trigger-spec>`                 → one TriggerSpec entry
//
// Trailing content on a `// materialize` line (beyond whitespace) is ignored
// so the old `// materialize s3://foo` style is forward-compatible: the
// bare marker still fires even if users have stale per-asset arguments.
pub fn parse_pipeline_annotations(code: &str) -> (bool, Vec<TriggerSpec>) {
    let mut is_materializer = false;
    let mut triggers: Vec<TriggerSpec> = vec![];

    for raw_line in code.lines() {
        let line = raw_line.trim_start();
        let rest = if let Some(r) = line.strip_prefix("//") {
            r
        } else if let Some(r) = line.strip_prefix("--") {
            r
        } else if let Some(r) = line.strip_prefix('#') {
            r
        } else {
            continue;
        };
        let rest = rest.trim_start();

        if let Some(after_kw) = rest.strip_prefix("materialize") {
            // Bare marker: must be end-of-line, whitespace, or nothing.
            // Rejects `materializer`, `materialized`, etc.
            if after_kw.is_empty() || after_kw.starts_with(|c: char| c.is_whitespace()) {
                is_materializer = true;
            }
            continue;
        }

        if let Some(after_kw) = rest.strip_prefix("on") {
            if !after_kw.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            let spec_text = after_kw.trim();
            if spec_text.is_empty() {
                continue;
            }
            if let Some(trig) = parse_trigger_spec(spec_text) {
                if !triggers.contains(&trig) {
                    triggers.push(trig);
                }
            }
        }
    }

    (is_materializer, triggers)
}

// Parse a single `on <spec>` right-hand side. Accepted forms:
//   schedule "<cron expr>"
//   <kind> <trigger-path>           — where <kind> is one of
//     webhook | email | kafka | mqtt | nats | postgres | sqs | gcp
//   <asset-path-with-prefix>        (e.g. s3://bucket/key, $res:f/foo)
fn parse_trigger_spec(s: &str) -> Option<TriggerSpec> {
    if let Some(rest) = s.strip_prefix("schedule") {
        let rest = rest.trim_start();
        // Accept either double or single quotes around the cron expression.
        let cron = rest
            .strip_prefix('"')
            .and_then(|r| r.strip_suffix('"'))
            .or_else(|| rest.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')))?;
        if cron.trim().is_empty() {
            return None;
        }
        return Some(TriggerSpec::Schedule { cron: cron.to_string() });
    }

    // `<kind> <path>` — delegate to a tiny table so the annotation set
    // stays in lockstep with `TriggerSpec`.
    type Ctor = fn(String) -> TriggerSpec;
    const KINDS: &[(&str, Ctor)] = &[
        ("webhook", |p| TriggerSpec::Webhook { path: p }),
        ("email", |p| TriggerSpec::Email { path: p }),
        ("kafka", |p| TriggerSpec::Kafka { path: p }),
        ("mqtt", |p| TriggerSpec::Mqtt { path: p }),
        ("nats", |p| TriggerSpec::Nats { path: p }),
        ("postgres", |p| TriggerSpec::Postgres { path: p }),
        ("sqs", |p| TriggerSpec::Sqs { path: p }),
        ("gcp", |p| TriggerSpec::Gcp { path: p }),
    ];
    for (kw, ctor) in KINDS {
        if let Some(rest) = s.strip_prefix(kw) {
            if !rest.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            let path = rest.trim();
            if path.is_empty() {
                return None;
            }
            return Some(ctor(path.to_string()));
        }
    }

    let (kind, path) = parse_asset_syntax(s, false)?;
    Some(TriggerSpec::Asset { asset_kind: kind, path: path.to_string() })
}

#[cfg(test)]
mod pipeline_annotation_tests {
    use super::*;

    #[test]
    fn bare_materialize_marker() {
        let (is_m, triggers) = parse_pipeline_annotations("// materialize\nconsole.log('hi')");
        assert!(is_m);
        assert!(triggers.is_empty());
    }

    #[test]
    fn bare_materialize_with_trailing_noise_still_fires() {
        // Forward-compat with old `// materialize s3://foo` style.
        let (is_m, _) = parse_pipeline_annotations("// materialize s3://legacy");
        assert!(is_m);
    }

    #[test]
    fn rejects_materializer_keyword_variants() {
        let (is_m, _) = parse_pipeline_annotations("// materializer\n# materialized");
        assert!(!is_m);
    }

    #[test]
    fn on_schedule() {
        let (_, triggers) = parse_pipeline_annotations("// on schedule \"0 */6 * * *\"");
        assert_eq!(triggers.len(), 1);
        assert_eq!(
            triggers[0],
            TriggerSpec::Schedule { cron: "0 */6 * * *".to_string() }
        );
    }

    #[test]
    fn on_asset_ts_py_sql() {
        let code = "// on s3://a/b\n# on datatable://main\n-- on $res:f/foo";
        let (_, triggers) = parse_pipeline_annotations(code);
        assert_eq!(triggers.len(), 3);
        assert!(matches!(
            triggers[0],
            TriggerSpec::Asset { asset_kind: AssetKind::S3Object, .. }
        ));
        assert!(matches!(
            triggers[1],
            TriggerSpec::Asset { asset_kind: AssetKind::DataTable, .. }
        ));
        assert!(matches!(
            triggers[2],
            TriggerSpec::Asset { asset_kind: AssetKind::Resource, .. }
        ));
    }

    #[test]
    fn on_deduplicates() {
        let code = "// on s3://a/b\n# on s3://a/b";
        let (_, triggers) = parse_pipeline_annotations(code);
        assert_eq!(triggers.len(), 1);
    }

    #[test]
    fn rejects_unknown_trigger_spec() {
        let (_, triggers) = parse_pipeline_annotations("// on unknown://nope\n# on schedule");
        assert!(triggers.is_empty());
    }

    #[test]
    fn combined() {
        let code = "// materialize\n// on s3://in.csv\n// on schedule \"0 0 * * *\"";
        let (is_m, triggers) = parse_pipeline_annotations(code);
        assert!(is_m);
        assert_eq!(triggers.len(), 2);
    }
}
