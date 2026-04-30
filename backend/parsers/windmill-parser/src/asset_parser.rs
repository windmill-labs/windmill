use serde::Serialize;
use std::collections::BTreeMap;

// Token recognized inside declared asset URIs that the runtime substitutes
// with the current partition value (e.g. `s3://lake/{partition}.parquet`
// becomes `s3://lake/2024-04-29.parquet` when materialized for that day).
// Substitution is the parser's job only at lineage/graph time — at runtime
// the pipeline worker performs the actual replacement before emitting
// asset signals. Kept as a single well-known token so the parser never
// has to guess which `{...}` placeholders are partition variables.
pub const PARTITION_TOKEN: &str = "{partition}";

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
    // Bare `// pipeline` (or `#` / `--`) on its own line — opt-in marker
    // that sets auto_kind='pipeline' and includes the script in its
    // folder's pipeline. Pipeline membership is broader than
    // materialization: scripts that only assert/notify/clean up are
    // members too. Outputs (when present) come from parser-detected
    // `w`/`rw` usages in `assets`, not from this marker.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub in_pipeline: bool,
    // Trigger annotations — execution DAG edges. Each is an independent OR
    // (any fires the script). Empty = script has no automatic triggers
    // (still runnable manually / via existing cron triggers). Includes both
    // top-level `// schedule "..."` and `// on <kind> ...` forms.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub triggers: Vec<TriggerSpec>,
    // `// partitioned <kind> [opts]` — declares that this pipeline script
    // produces partitioned output. The runtime resolves `{partition}` in
    // declared output URIs to the current partition value before signaling
    // downstream consumers. At most one per script.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub partition: Option<PartitionSpec>,
    // `// freshness <duration>` — SLA stating outputs must be at most
    // `duration` old. Active backstop: when no other trigger has fired the
    // script within the window, a watchdog re-runs it. Distinct from
    // schedule (which is producer cadence); freshness is consumer SLA and
    // applies regardless of which trigger last fired.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub freshness: Option<FreshnessSpec>,
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

// Partitioning declaration for a pipeline script. `daily`/`hourly`/`weekly`/
// `monthly` are time-based with the runtime supplying the current
// partition value derived from the trigger context (schedule fire time,
// freshness window, manual run arg). `dynamic` extracts the value from the
// triggering payload via JSONPath — used for per-tenant / per-shard /
// per-event-id pipelines where the partition key isn't a wall-clock value.
#[derive(Serialize, Debug, PartialEq, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PartitionKind {
    Daily,
    Hourly,
    Weekly,
    Monthly,
    Dynamic { key: String },
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct PartitionSpec {
    #[serde(flatten)]
    pub kind: PartitionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tz: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    // ISO-8601 (e.g. "2024-01-01") or kind-specific start anchor. Older
    // partitions before this anchor are not backfilled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
}

// Freshness SLA. The duration is kept as a raw string ("1h", "30m", "2d")
// and validated downstream — the parser deliberately doesn't bind to a
// specific duration crate so the annotation grammar stays parser-light.
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct FreshnessSpec {
    pub duration: String,
}

// All pipeline-level annotations parsed off a script's source. Returned by
// `parse_pipeline_annotations` and forwarded into `ParseAssetsOutput`.
#[derive(Default, Debug, PartialEq, Clone)]
pub struct PipelineAnnotations {
    pub in_pipeline: bool,
    pub triggers: Vec<TriggerSpec>,
    pub partition: Option<PartitionSpec>,
    pub freshness: Option<FreshnessSpec>,
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

// Strip an optional surrounding pair of double or single quotes. Returns
// the inner string if quoted, or `None` otherwise. Used by the annotation
// parser for cron expressions and key=value option values.
fn unquote(s: &str) -> Option<&str> {
    s.strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .or_else(|| s.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')))
}

// Tokenize a `key=value [key="quoted value"] ...` option string. Bare
// values run until the next whitespace; quoted values consume until the
// matching quote. Malformed pairs (missing `=` or empty key) are skipped
// rather than aborting the whole annotation.
fn parse_kv_opts(s: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut chars = s.chars().peekable();
    loop {
        while chars.peek().map_or(false, |c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(c);
            chars.next();
        }
        if chars.peek() != Some(&'=') || key.is_empty() {
            // Malformed — skip until next whitespace to recover.
            while chars.peek().map_or(false, |c| !c.is_whitespace()) {
                chars.next();
            }
            continue;
        }
        chars.next(); // consume '='
        let value = match chars.peek().copied() {
            Some(q @ ('"' | '\'')) => {
                chars.next();
                let mut v = String::new();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == q {
                        break;
                    }
                    v.push(c);
                }
                v
            }
            _ => {
                let mut v = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() {
                        break;
                    }
                    v.push(c);
                    chars.next();
                }
                v
            }
        };
        out.insert(key, value);
    }
    out
}

// Scan raw source for pipeline annotations. Language-agnostic: any line
// whose first non-whitespace tokens are a comment prefix (`//`, `#`, or
// `--`) followed by one of the recognized keywords:
//   - `pipeline`                 → opt-in marker (must be alone on the line)
//   - `schedule "<cron>"`        → top-level inline cron schedule
//   - `on <trigger-spec>`        → asset / native trigger edge
//   - `partitioned <kind> [opts]` → partition declaration
//   - `freshness <duration>`     → SLA / active backstop
//
// `// pipeline` is intentionally strict — only whitespace allowed after the
// keyword. Without that constraint, casual prose like `// pipeline broken
// on staging` would false-positive (the word "pipeline" is far more common
// in normal comments than "materialize" was).
//
// `partition` and `freshness` use first-write-wins; if multiple lines
// declare them, the first one is kept (last would be reasonable too, but
// first matches the file-top convention developers follow).
pub fn parse_pipeline_annotations(code: &str) -> PipelineAnnotations {
    let mut out = PipelineAnnotations::default();

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

        if let Some(after_kw) = rest.strip_prefix("pipeline") {
            // Strict: keyword must be the only content on the line. Rejects
            // `pipeline broken`, `pipelines`, `pipeline-related`, etc.
            if after_kw.trim().is_empty() {
                out.in_pipeline = true;
            }
            continue;
        }

        if let Some(after_kw) = rest.strip_prefix("schedule") {
            if !after_kw.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            let after = after_kw.trim();
            if let Some(cron) = unquote(after) {
                if !cron.trim().is_empty() {
                    let trig = TriggerSpec::Schedule { cron: cron.to_string() };
                    if !out.triggers.contains(&trig) {
                        out.triggers.push(trig);
                    }
                }
            }
            continue;
        }

        if let Some(after_kw) = rest.strip_prefix("partitioned") {
            if !after_kw.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            if out.partition.is_none() {
                if let Some(spec) = parse_partitioned_spec(after_kw.trim()) {
                    out.partition = Some(spec);
                }
            }
            continue;
        }

        if let Some(after_kw) = rest.strip_prefix("freshness") {
            if !after_kw.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            let dur = after_kw.trim();
            if !dur.is_empty() && out.freshness.is_none() {
                out.freshness = Some(FreshnessSpec { duration: dur.to_string() });
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
                if !out.triggers.contains(&trig) {
                    out.triggers.push(trig);
                }
            }
        }
    }

    out
}

// Parse a `// partitioned <kind> [opts]` right-hand side. Recognized kinds:
// `daily`, `hourly`, `weekly`, `monthly` (with optional tz/format/start),
// and `dynamic key="<jsonpath>"` (plus optional format).
fn parse_partitioned_spec(s: &str) -> Option<PartitionSpec> {
    let mut split = s.splitn(2, char::is_whitespace);
    let kind_word = split.next()?;
    let opts_str = split.next().unwrap_or("");
    let opts = parse_kv_opts(opts_str);
    let kind = match kind_word {
        "daily" => PartitionKind::Daily,
        "hourly" => PartitionKind::Hourly,
        "weekly" => PartitionKind::Weekly,
        "monthly" => PartitionKind::Monthly,
        "dynamic" => {
            let key = opts.get("key")?.clone();
            if key.is_empty() {
                return None;
            }
            PartitionKind::Dynamic { key }
        }
        _ => return None,
    };
    Some(PartitionSpec {
        kind,
        tz: opts.get("tz").cloned(),
        format: opts.get("format").cloned(),
        start: opts.get("start").cloned(),
    })
}

// Parse a single `on <spec>` right-hand side. Accepted forms:
//   <kind> <trigger-path>           — where <kind> is one of
//     webhook | email | kafka | mqtt | nats | postgres | sqs | gcp
//   <asset-path-with-prefix>        (e.g. s3://bucket/key, $res:f/foo)
//
// Note: `on schedule "..."` is no longer accepted — schedule moved to a
// top-level `// schedule "..."` annotation. The `Schedule` TriggerSpec
// variant is still produced, just from a different keyword.
fn parse_trigger_spec(s: &str) -> Option<TriggerSpec> {
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
    fn bare_pipeline_marker() {
        let out = parse_pipeline_annotations("// pipeline\nconsole.log('hi')");
        assert!(out.in_pipeline);
        assert!(out.triggers.is_empty());
    }

    #[test]
    fn pipeline_marker_strict_grammar_rejects_trailing_words() {
        // Strict — the word "pipeline" is common in casual prose, so trailing
        // content disqualifies the line. Trailing whitespace alone is fine.
        let out = parse_pipeline_annotations(
            "// pipeline broken on staging\n# pipeline this through grep\n-- pipeline_v2",
        );
        assert!(!out.in_pipeline);

        let out = parse_pipeline_annotations("// pipeline   \n");
        assert!(out.in_pipeline);
    }

    #[test]
    fn rejects_pipeline_keyword_variants() {
        let out = parse_pipeline_annotations("// pipelines\n# pipelined\n-- pipeline-foo");
        assert!(!out.in_pipeline);
    }

    #[test]
    fn top_level_schedule() {
        let out = parse_pipeline_annotations("// schedule \"0 */6 * * *\"");
        assert_eq!(out.triggers.len(), 1);
        assert_eq!(
            out.triggers[0],
            TriggerSpec::Schedule { cron: "0 */6 * * *".to_string() }
        );
    }

    #[test]
    fn top_level_schedule_single_quotes_and_other_prefixes() {
        let out = parse_pipeline_annotations("# schedule '0 0 * * *'\n-- schedule \"@daily\"");
        assert_eq!(out.triggers.len(), 2);
    }

    #[test]
    fn rejects_on_schedule_form() {
        // The old `on schedule "..."` form is no longer recognized — the
        // `on` branch tries `schedule` as a native trigger kind and falls
        // through.
        let out = parse_pipeline_annotations("// on schedule \"0 0 * * *\"");
        assert!(out.triggers.is_empty());
    }

    #[test]
    fn on_asset_ts_py_sql() {
        let code = "// on s3://a/b\n# on datatable://main\n-- on $res:f/foo";
        let out = parse_pipeline_annotations(code);
        assert_eq!(out.triggers.len(), 3);
        assert!(matches!(
            out.triggers[0],
            TriggerSpec::Asset { asset_kind: AssetKind::S3Object, .. }
        ));
        assert!(matches!(
            out.triggers[1],
            TriggerSpec::Asset { asset_kind: AssetKind::DataTable, .. }
        ));
        assert!(matches!(
            out.triggers[2],
            TriggerSpec::Asset { asset_kind: AssetKind::Resource, .. }
        ));
    }

    #[test]
    fn on_deduplicates() {
        let code = "// on s3://a/b\n# on s3://a/b";
        let out = parse_pipeline_annotations(code);
        assert_eq!(out.triggers.len(), 1);
    }

    #[test]
    fn rejects_unknown_trigger_spec() {
        let out = parse_pipeline_annotations("// on unknown://nope\n# schedule");
        assert!(out.triggers.is_empty());
    }

    #[test]
    fn partitioned_daily() {
        let code = "// partitioned daily tz=\"UTC\" format=\"YYYY-MM-DD\" start=\"2024-01-01\"";
        let out = parse_pipeline_annotations(code);
        let p = out.partition.expect("partition");
        assert_eq!(p.kind, PartitionKind::Daily);
        assert_eq!(p.tz.as_deref(), Some("UTC"));
        assert_eq!(p.format.as_deref(), Some("YYYY-MM-DD"));
        assert_eq!(p.start.as_deref(), Some("2024-01-01"));
    }

    #[test]
    fn partitioned_hourly_minimal() {
        let out = parse_pipeline_annotations("// partitioned hourly");
        let p = out.partition.expect("partition");
        assert_eq!(p.kind, PartitionKind::Hourly);
        assert!(p.tz.is_none());
    }

    #[test]
    fn partitioned_dynamic_requires_key() {
        let out = parse_pipeline_annotations("// partitioned dynamic");
        assert!(out.partition.is_none());
        let out = parse_pipeline_annotations("// partitioned dynamic key=\"$.tenant_id\"");
        let p = out.partition.expect("partition");
        assert_eq!(
            p.kind,
            PartitionKind::Dynamic { key: "$.tenant_id".to_string() }
        );
    }

    #[test]
    fn partitioned_first_wins() {
        let code = "// partitioned daily tz=\"UTC\"\n// partitioned hourly";
        let out = parse_pipeline_annotations(code);
        let p = out.partition.expect("partition");
        assert_eq!(p.kind, PartitionKind::Daily);
    }

    #[test]
    fn partitioned_unknown_kind_is_skipped() {
        let out = parse_pipeline_annotations("// partitioned bogus");
        assert!(out.partition.is_none());
    }

    #[test]
    fn freshness_basic() {
        let out = parse_pipeline_annotations("// freshness 1h");
        assert_eq!(out.freshness.unwrap().duration, "1h");
    }

    #[test]
    fn freshness_first_wins() {
        let out = parse_pipeline_annotations("// freshness 1h\n# freshness 30m");
        assert_eq!(out.freshness.unwrap().duration, "1h");
    }

    #[test]
    fn combined() {
        let code = concat!(
            "// pipeline\n",
            "// schedule \"0 0 * * *\"\n",
            "// on s3://in.csv\n",
            "// partitioned daily tz=\"UTC\"\n",
            "// freshness 2h\n"
        );
        let out = parse_pipeline_annotations(code);
        assert!(out.in_pipeline);
        assert_eq!(out.triggers.len(), 2);
        assert!(out.partition.is_some());
        assert_eq!(out.freshness.unwrap().duration, "2h");
    }

    #[test]
    fn kv_opts_quoted_with_spaces() {
        let m = parse_kv_opts("a=\"hello world\" b=plain c='single quoted'");
        assert_eq!(m.get("a").unwrap(), "hello world");
        assert_eq!(m.get("b").unwrap(), "plain");
        assert_eq!(m.get("c").unwrap(), "single quoted");
    }

    #[test]
    fn kv_opts_malformed_recovers() {
        let m = parse_kv_opts("garbage a=ok =alone b=fine");
        assert_eq!(m.get("a").unwrap(), "ok");
        assert_eq!(m.get("b").unwrap(), "fine");
        assert!(m.get("garbage").is_none());
    }
}
