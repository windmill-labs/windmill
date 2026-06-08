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
    // `// trigger all` → AND join barrier; default (`any`) = OR (current
    // behaviour). Threaded to the deploy path which persists it on the
    // subscriber's trigger rows.
    #[serde(skip_serializing_if = "JoinMode::is_any", default)]
    pub join_mode: JoinMode,
    // `// debounce <dur>` — script-level default debounce window for this
    // script's asset inputs. A per-`// on … debounce=` overrides it. Raw
    // duration string, parsed to seconds at deploy (parser-light, like
    // freshness). Absent = no debounce (fan-out, current behaviour).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub debounce_default: Option<String>,
    // `// tag <name>` — overrides the script's worker tag at deploy. Source
    // wins over any UI-set value, matching the wipe-and-reinsert convention
    // of other pipeline annotations (`// schedule`, `// on`). Absent = keep
    // whatever the caller (UI / CLI) supplied.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tag: Option<String>,
    // `// retry <count> [<delay>]` — re-run a pipeline-cascade triggered
    // script up to `count` times on failure, waiting `delay` between
    // attempts. Applies only to runs launched via the asset/schedule
    // cascade (the rows in `script_trigger`); manual UI runs are unaffected.
    // The delay is a raw duration string parsed at deploy (parser-light).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub retry: Option<RetrySpec>,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TriggerSpec {
    // Refresh when `<asset>` changes. Kind comes from parse_asset_syntax so
    // it matches the `asset` table. `debounce` is the optional per-input
    // `// on … debounce=<dur>` override (raw duration string); it takes
    // precedence over the script-level `// debounce` default, resolved at
    // deploy.
    Asset {
        asset_kind: AssetKind,
        path: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        debounce: Option<String>,
    },
    // `// on <kind>` — marker-only declaration that this script wants to be
    // triggered by a native trigger of the given kind. No path: the binding
    // is the trigger row's own `script_path` field (set when the user creates
    // the kafka/mqtt/schedule/… trigger in its dedicated UI). The graph
    // endpoint discovers attached triggers by `WHERE script_path = <this
    // script>` and surfaces a "missing" placeholder when an annotation has no
    // matching row.
    Schedule,
    Webhook,
    Email,
    Kafka,
    Mqtt,
    Nats,
    Postgres,
    Sqs,
    Gcp,
    // `// on data_upload` — UI-first entry point. Unlike the other native
    // kinds there is no external event source and no trigger row anywhere:
    // the script declares an `S3Object` input parameter and the user uploads
    // a file via the auto-generated S3 picker, which runs the pipeline. The
    // graph renders it as a clickable upload source (never a "missing"
    // placeholder, mirroring webhook).
    #[serde(rename = "data_upload")]
    DataUpload,
}

impl TriggerSpec {
    // A `// on <asset>` whose declared path contains the `{partition}`
    // token is *partition-bearing*: in an AND join its concrete partition
    // value is the join key. Non-asset triggers and assets without the
    // token are reference/presence-only inputs that never define the
    // partition (the case-3 guard).
    pub fn is_partition_bearing(&self) -> bool {
        matches!(self, TriggerSpec::Asset { path, .. } if path.contains(PARTITION_TOKEN))
    }
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

// Retry policy declared via `// retry <count> [<delay>]`. The delay is kept
// as a raw duration string (mirrors freshness/debounce_default) and resolved
// to seconds at deploy via `parse_duration_secs`; absent = back-to-back
// re-runs with no inter-attempt wait.
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct RetrySpec {
    pub count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,
}

// `// trigger any` (default) vs `// trigger all`. `Any` = OR: any trigger
// firing runs the script (current behaviour). `All` = AND: the script
// runs only once every partition-bearing input has materialized at the
// same partition (plus every reference input exists) — the join barrier.
#[derive(Serialize, Debug, PartialEq, Eq, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum JoinMode {
    #[default]
    Any,
    All,
}

impl JoinMode {
    pub fn is_any(&self) -> bool {
        matches!(self, JoinMode::Any)
    }
}

// All pipeline-level annotations parsed off a script's source. Returned by
// `parse_pipeline_annotations` and forwarded into `ParseAssetsOutput`.
#[derive(Default, Debug, PartialEq, Clone)]
pub struct PipelineAnnotations {
    pub in_pipeline: bool,
    pub triggers: Vec<TriggerSpec>,
    pub partition: Option<PartitionSpec>,
    pub freshness: Option<FreshnessSpec>,
    pub join_mode: JoinMode,
    pub debounce_default: Option<String>,
    pub tag: Option<String>,
    pub retry: Option<RetrySpec>,
}

impl ParseAssetsOutput {
    /// Build from detected assets/queries plus the script's parsed
    /// pipeline annotations, so each language asset-parser does not
    /// re-list the per-annotation fields (one call site instead of six
    /// lines kept in lockstep across the parser crates).
    pub fn new(
        assets: Vec<ParseAssetsResult>,
        sql_queries: Vec<SqlQueryDetails>,
        pipeline: PipelineAnnotations,
    ) -> Self {
        ParseAssetsOutput {
            assets,
            sql_queries,
            in_pipeline: pipeline.in_pipeline,
            triggers: pipeline.triggers,
            partition: pipeline.partition,
            freshness: pipeline.freshness,
            join_mode: pipeline.join_mode,
            debounce_default: pipeline.debounce_default,
            tag: pipeline.tag,
            retry: pipeline.retry,
        }
    }
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
// Split a `// on` right-hand side into the trigger ref and any trailing
// `key=value` opts. The opts section starts at the first whitespace token
// shaped like `<ident>=…` (e.g. `debounce=60s`); everything before is the
// asset/kind ref. Asset refs aren't expected to contain a space then an
// `ident=` token — the same assumption `// partitioned` already makes.
fn split_trailing_kv_opts(s: &str) -> (&str, BTreeMap<String, String>) {
    let mut split_at: Option<usize> = None;
    let mut pos = 0usize;
    for tok in s.split_whitespace() {
        let tok_start = s[pos..].find(tok).map(|o| pos + o).unwrap_or(pos);
        pos = tok_start + tok.len();
        if let Some(eq) = tok.find('=') {
            let key = &tok[..eq];
            if !key.is_empty()
                && key.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                split_at = Some(tok_start);
                break;
            }
        }
    }
    match split_at {
        Some(i) => (s[..i].trim_end(), parse_kv_opts(&s[i..])),
        None => (s.trim_end(), BTreeMap::new()),
    }
}

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
//   - `tag <name>`               → worker-tag override (annotation wins
//                                  over UI-set value at deploy)
//   - `retry <count> [<delay>]`  → cascade-only retry policy
//
// `// pipeline` is intentionally strict — only whitespace allowed after the
// keyword. Without that constraint, casual prose like `// pipeline broken
// on staging` would false-positive (the word "pipeline" is far more common
// in normal comments than "materialize" was).
//
// `partition`, `freshness`, `tag`, and `retry` use first-write-wins; if
// multiple lines declare them, the first one is kept (last would be
// reasonable too, but first matches the file-top convention developers
// follow).
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

        if let Some(after_kw) = rest.strip_prefix("trigger") {
            if !after_kw.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            match after_kw.trim() {
                "all" => out.join_mode = JoinMode::All,
                "any" => out.join_mode = JoinMode::Any,
                // Unknown value — leave the default rather than guess.
                _ => {}
            }
            continue;
        }

        if let Some(after_kw) = rest.strip_prefix("debounce") {
            if !after_kw.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            let dur = after_kw.trim();
            if !dur.is_empty() && out.debounce_default.is_none() {
                out.debounce_default = Some(dur.to_string());
            }
            continue;
        }

        if let Some(after_kw) = rest.strip_prefix("tag") {
            if !after_kw.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            let name = after_kw.trim();
            if !name.is_empty() && out.tag.is_none() {
                out.tag = Some(name.to_string());
            }
            continue;
        }

        if let Some(after_kw) = rest.strip_prefix("retry") {
            if !after_kw.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            if out.retry.is_none() {
                if let Some(spec) = parse_retry_spec(after_kw.trim()) {
                    out.retry = Some(spec);
                }
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
            // Split off trailing `key=value` opts (e.g. `debounce=60s`)
            // from the asset/kind ref. The per-input debounce override is
            // only meaningful for asset inputs (cascade fan-out); other
            // trigger kinds ignore it.
            let (ref_part, opts) = split_trailing_kv_opts(spec_text);
            if let Some(mut trig) = parse_trigger_spec(ref_part) {
                if let TriggerSpec::Asset { debounce, .. } = &mut trig {
                    *debounce = opts
                        .get("debounce")
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());
                }
                if !out.triggers.contains(&trig) {
                    out.triggers.push(trig);
                }
            }
        }
    }

    out
}

// Parse a `// retry <count> [<delay>]` right-hand side. `<count>` is a
// non-negative decimal; `<delay>` is an optional raw duration string left
// for `parse_duration_secs` to validate at deploy. A bare zero count (or
// non-numeric token) is rejected — that is, the annotation must encode a
// real retry policy or be omitted, so a typo (`// retry retry 3`) fails
// safe rather than silently disabling cascade retries.
fn parse_retry_spec(s: &str) -> Option<RetrySpec> {
    let mut split = s.splitn(2, char::is_whitespace);
    let count_word = split.next()?.trim();
    let count: u32 = count_word.parse().ok()?;
    if count == 0 {
        return None;
    }
    let delay = split
        .next()
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty());
    Some(RetrySpec { count, delay })
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
// Native trigger keywords are *marker-only* — no trailing path. The actual
// binding lives on the native trigger row (`script_path` column). Anything
// trailing the keyword is rejected so the form stays unambiguous.
fn parse_trigger_spec(s: &str) -> Option<TriggerSpec> {
    // Marker-only native trigger keywords. The match table keeps the
    // annotation set in lockstep with `TriggerSpec`. `schedule` is in here
    // too — the cron lives on the schedule row the user creates separately;
    // the annotation is just the binding declaration.
    const NATIVE_KINDS: &[(&str, TriggerSpec)] = &[
        ("schedule", TriggerSpec::Schedule),
        ("webhook", TriggerSpec::Webhook),
        ("email", TriggerSpec::Email),
        ("kafka", TriggerSpec::Kafka),
        ("mqtt", TriggerSpec::Mqtt),
        ("nats", TriggerSpec::Nats),
        ("postgres", TriggerSpec::Postgres),
        ("sqs", TriggerSpec::Sqs),
        ("gcp", TriggerSpec::Gcp),
        ("data_upload", TriggerSpec::DataUpload),
    ];
    for (kw, spec) in NATIVE_KINDS {
        if let Some(rest) = s.strip_prefix(kw) {
            // Must be a complete word — `kafkalike` doesn't match `kafka`.
            // Trailing whitespace alone is fine; any non-empty trailing
            // content is treated as malformed (the annotation is marker-only).
            if !rest.is_empty() && !rest.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            if !rest.trim().is_empty() {
                return None;
            }
            return Some(spec.clone());
        }
    }

    let (kind, path) = parse_asset_syntax(s, false)?;
    // `debounce` is attached by the caller from the `// on` line's opts.
    Some(TriggerSpec::Asset { asset_kind: kind, path: path.to_string(), debounce: None })
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
    fn on_schedule_marker() {
        // `// on schedule` is marker-only — the binding is the schedule row's
        // own `script_path` field, just like kafka/mqtt/etc.
        let out = parse_pipeline_annotations("// on schedule");
        assert_eq!(out.triggers.len(), 1);
        assert_eq!(out.triggers[0], TriggerSpec::Schedule);
    }

    #[test]
    fn rejects_schedule_with_trailing_content() {
        // Marker-only — the old `// schedule "<cron>"` form is gone, and a
        // trailing path/cron on the `on schedule` form is malformed.
        let out = parse_pipeline_annotations("// on schedule \"0 0 * * *\"");
        assert!(out.triggers.is_empty());
        let out = parse_pipeline_annotations("// schedule \"0 0 * * *\"");
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
    fn join_mode_defaults_to_any() {
        let out = parse_pipeline_annotations("// on s3://a/b");
        assert_eq!(out.join_mode, JoinMode::Any);
        assert!(out.join_mode.is_any());
    }

    #[test]
    fn trigger_all_sets_and_mode() {
        let out = parse_pipeline_annotations(
            "// pipeline\n// on s3://lake/{partition}/x\n// trigger all",
        );
        assert_eq!(out.join_mode, JoinMode::All);
        assert!(!out.join_mode.is_any());
    }

    #[test]
    fn trigger_any_explicit_and_other_prefixes() {
        // explicit `any`, plus `#` / `--` comment prefixes are accepted.
        assert_eq!(
            parse_pipeline_annotations("# trigger all\n-- trigger any").join_mode,
            JoinMode::Any
        );
        assert_eq!(
            parse_pipeline_annotations("-- trigger all").join_mode,
            JoinMode::All
        );
    }

    #[test]
    fn trigger_unknown_or_glued_keeps_default() {
        // unknown value, and `triggerall` (no whitespace) must not match.
        assert_eq!(
            parse_pipeline_annotations("// trigger bogus").join_mode,
            JoinMode::Any
        );
        assert_eq!(
            parse_pipeline_annotations("// triggerall").join_mode,
            JoinMode::Any
        );
    }

    #[test]
    fn is_partition_bearing_only_for_tokened_assets() {
        let out = parse_pipeline_annotations(
            "// on s3://lake/raw/{partition}/events.parquet\n\
             // on s3://lake/dim/customers.parquet\n\
             // schedule \"@daily\"",
        );
        assert_eq!(out.triggers.len(), 3);
        assert!(out.triggers[0].is_partition_bearing());
        assert!(!out.triggers[1].is_partition_bearing());
        assert!(!out.triggers[2].is_partition_bearing());
    }

    fn asset_debounce(t: &TriggerSpec) -> Option<&str> {
        match t {
            TriggerSpec::Asset { debounce, .. } => debounce.as_deref(),
            _ => None,
        }
    }

    #[test]
    fn debounce_none_by_default() {
        let out = parse_pipeline_annotations("// on s3://a/b");
        assert_eq!(out.debounce_default, None);
        assert_eq!(asset_debounce(&out.triggers[0]), None);
    }

    #[test]
    fn script_level_debounce_default() {
        let out = parse_pipeline_annotations("// debounce 30s\n// on s3://a/b");
        assert_eq!(out.debounce_default.as_deref(), Some("30s"));
        // Per-edge override absent — precedence (edge ?? default) is
        // resolved at deploy, so the Asset itself stays None here.
        assert_eq!(asset_debounce(&out.triggers[0]), None);
    }

    #[test]
    fn on_level_debounce_override() {
        let out = parse_pipeline_annotations(
            "// debounce 30s\n\
             // on s3://lake/{partition}/raw.parquet debounce=60s\n\
             // on $res:f/cfg",
        );
        assert_eq!(out.debounce_default.as_deref(), Some("30s"));
        assert_eq!(out.triggers.len(), 2);
        assert_eq!(asset_debounce(&out.triggers[0]), Some("60s"));
        // ref still parses correctly with the trailing opt stripped.
        assert!(out.triggers[0].is_partition_bearing());
        assert_eq!(asset_debounce(&out.triggers[1]), None);
    }

    #[test]
    fn debounce_keyword_strictness_and_first_wins() {
        // `debounced` (no space) must not match; empty ignored; first wins.
        let out = parse_pipeline_annotations(
            "// debounced nope\n// debounce\n// debounce 1m\n// debounce 5m",
        );
        assert_eq!(out.debounce_default.as_deref(), Some("1m"));
    }

    #[test]
    fn native_trigger_keywords_are_marker_only() {
        // Marker form: `// on kafka` parses to the unit variant.
        let out = parse_pipeline_annotations("// on kafka");
        assert_eq!(out.triggers.len(), 1);
        assert!(matches!(out.triggers[0], TriggerSpec::Kafka));

        // Old path-bearing form is rejected (no path on native markers).
        let out = parse_pipeline_annotations("// on webhook f/foo");
        assert!(out.triggers.is_empty());

        // Trailing key=value opts are silently dropped by the line-level
        // KV splitter before parse_trigger_spec sees them — same behaviour
        // for both asset and native kinds. The opts have no meaning for a
        // marker, but the marker still parses.
        let out = parse_pipeline_annotations("// on mqtt debounce=30s");
        assert_eq!(out.triggers.len(), 1);
        assert!(matches!(out.triggers[0], TriggerSpec::Mqtt));

        // `kafkalike` mustn't match `kafka`.
        let out = parse_pipeline_annotations("// on kafkalike");
        assert!(out.triggers.is_empty());
    }

    #[test]
    fn all_native_marker_keywords_parse() {
        let code = "// on webhook\n// on email\n// on kafka\n// on mqtt\n\
                    // on nats\n// on postgres\n// on sqs\n// on gcp\n// on data_upload";
        let out = parse_pipeline_annotations(code);
        assert_eq!(out.triggers.len(), 9);
        assert!(matches!(out.triggers[0], TriggerSpec::Webhook));
        assert!(matches!(out.triggers[1], TriggerSpec::Email));
        assert!(matches!(out.triggers[2], TriggerSpec::Kafka));
        assert!(matches!(out.triggers[3], TriggerSpec::Mqtt));
        assert!(matches!(out.triggers[4], TriggerSpec::Nats));
        assert!(matches!(out.triggers[5], TriggerSpec::Postgres));
        assert!(matches!(out.triggers[6], TriggerSpec::Sqs));
        assert!(matches!(out.triggers[7], TriggerSpec::Gcp));
        assert!(matches!(out.triggers[8], TriggerSpec::DataUpload));
    }

    #[test]
    fn data_upload_marker_is_marker_only() {
        // `// on data_upload` parses to the unit variant — no path.
        let out = parse_pipeline_annotations("// on data_upload");
        assert_eq!(out.triggers.len(), 1);
        assert!(matches!(out.triggers[0], TriggerSpec::DataUpload));

        // Trailing content makes it malformed (marker-only).
        let out = parse_pipeline_annotations("// on data_upload f/foo");
        assert!(out.triggers.is_empty());

        // `data_uploadish` mustn't match `data_upload`.
        let out = parse_pipeline_annotations("// on data_uploadish");
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
    fn tag_basic() {
        let out = parse_pipeline_annotations("// tag heavy");
        assert_eq!(out.tag.as_deref(), Some("heavy"));
    }

    #[test]
    fn tag_first_wins() {
        let out = parse_pipeline_annotations("// tag heavy\n# tag light");
        assert_eq!(out.tag.as_deref(), Some("heavy"));
    }

    #[test]
    fn tag_empty_is_skipped() {
        let out = parse_pipeline_annotations("// tag   ");
        assert!(out.tag.is_none());
    }

    #[test]
    fn retry_count_only() {
        let out = parse_pipeline_annotations("// retry 3");
        let r = out.retry.expect("retry");
        assert_eq!(r.count, 3);
        assert_eq!(r.delay, None);
    }

    #[test]
    fn retry_with_delay() {
        let out = parse_pipeline_annotations("// retry 3 5s");
        let r = out.retry.expect("retry");
        assert_eq!(r.count, 3);
        assert_eq!(r.delay.as_deref(), Some("5s"));
    }

    #[test]
    fn retry_first_wins() {
        let out = parse_pipeline_annotations("// retry 3 5s\n# retry 1");
        let r = out.retry.expect("retry");
        assert_eq!(r.count, 3);
        assert_eq!(r.delay.as_deref(), Some("5s"));
    }

    #[test]
    fn retry_zero_is_skipped() {
        let out = parse_pipeline_annotations("// retry 0 5s");
        assert!(out.retry.is_none());
    }

    #[test]
    fn retry_non_numeric_count_is_skipped() {
        let out = parse_pipeline_annotations("// retry many");
        assert!(out.retry.is_none());
    }

    #[test]
    fn combined() {
        let code = concat!(
            "// pipeline\n",
            "// schedule \"0 0 * * *\"\n",
            "// on s3://in.csv\n",
            "// partitioned daily tz=\"UTC\"\n",
            "// freshness 2h\n",
            "// tag heavy\n",
            "// retry 3 5s\n"
        );
        let out = parse_pipeline_annotations(code);
        assert!(out.in_pipeline);
        assert_eq!(out.triggers.len(), 2);
        assert!(out.partition.is_some());
        assert_eq!(out.freshness.unwrap().duration, "2h");
        assert_eq!(out.tag.as_deref(), Some("heavy"));
        let r = out.retry.expect("retry");
        assert_eq!(r.count, 3);
        assert_eq!(r.delay.as_deref(), Some("5s"));
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
