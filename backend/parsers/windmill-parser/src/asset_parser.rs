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
    // `duration` old. Drives passive monitoring in CE (the asset graph
    // colors the node's badge fresh/stale against its last successful run)
    // and the enterprise watchdog (windmill-queue `freshness_watchdog`),
    // which re-runs a stale unpartitioned producer. Distinct from schedule
    // (which is producer cadence); freshness is consumer SLA and applies
    // regardless of which trigger last fired.
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
    // `// materialize [manual] <asset> [append] [key=<col>]` —
    // managed-materialization target + its strategy. At most one per script.
    // Drives the worker's write-strategy + snapshot capture.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub materialize: Option<MaterializeSpec>,
    // `// data_test <kind> …` — data-quality assertions run against the
    // materialized asset after the write commits. Accumulating (multiple
    // lines allowed). Drives the worker's post-materialize verifier probes.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub data_tests: Vec<DataTest>,
    // `// column <out> <- <src>.<col>[, …]` — declared column-level lineage,
    // one entry per output column. Accumulating. Pure metadata: drives the
    // column-lineage graph view, executes nothing.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub column_lineage: Vec<ColumnLineage>,
    // Bare `// macros` (must be alone on the line, like `// pipeline`) —
    // marks this DuckDB script as a workspace *macro library*: its body is
    // CREATE [OR REPLACE] MACRO statements (plus plain setup) registered at
    // deploy and injected as TEMP macros into consumer jobs at run time.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub macros: bool,
    // `// use <lib_script_path>` — force-inject the whole named macro
    // library (definitions + its setup statements) into this script's jobs,
    // for dynamic SQL that call-detection can't see. Accumulating.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub use_libs: Vec<String>,
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

// `// materialize [manual] <asset> [append] [key=<col>] [history] [track=<c1,c2>]`
// — declares that this script produces a *managed* materialization of `<asset>`
// (a `ducklake://` table). By default the runtime generates the write DDL around
// the script's single trailing `SELECT` and owns idempotency, partition-state
// and snapshot capture. `manual` is the escape hatch: the script writes its own
// DDL and the runtime only records state (track-only). The reconciliation
// strategy options apply to managed mode: none → DELETE-by-partition + INSERT
// (replace); `key=<col>` → MERGE (dedup within slice, SCD type 1); `append` →
// INSERT-only. `append` wins if both are given (deploy-time warning).
// `key=<col> history` upgrades the merge to SCD type 2: the SELECT is the current
// snapshot (one row per key), and a change to any tracked column (`track=`,
// default all non-key) closes the prior version and opens a new one, keeping full
// history (`valid_from`/`valid_to`/`is_current`). The leading keyword `scd2` is a
// recognized alias for `history`. `deletes=close` (scd2 only) also closes a key
// that disappears from the snapshot; default leaves absent keys current.
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct MaterializeSpec {
    pub target_kind: AssetKind,
    pub target_path: String,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub manual: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub append: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unique_key: Option<String>,
    // `scd2` managed history mode: the SELECT is the current snapshot (one row
    // per `unique_key`), and the runtime maintains a Slowly-Changing-Dimension
    // type-2 history (`valid_from`/`valid_to`/`is_current`). `unique_key` (the
    // `key=` opt) is the natural key; `track` lists the columns whose change
    // opens a new version (empty ⇒ all non-key columns). Managed mode only.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub scd2: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub track: Vec<String>,
    // scd2 only: `deletes=close` opts into hard-delete-close — a key that
    // disappears from the snapshot has its current version closed (dbt's
    // `hard_deletes=close`). Default (false) leaves absent keys current.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub close_deleted: bool,
}

// `// data_test <kind> …` — a data-quality assertion run against the
// freshly-materialized asset (post DELETE+INSERT), failing the run on
// violation. The first extensible annotation family: the parser turns a
// `data_test` line into one of a known *vocabulary* of checks, and the
// runtime turns each check into a SQL "verifier" probe. A sibling annotation
// family (e.g. column-lineage) follows the same shape — a keyword head
// selecting a variant, the rest parsed per-variant — rather than growing a
// new closed list. See `docs/ducklake-materialization.md` §"Extensible
// annotations". Multiple `// data_test` lines accumulate (unlike the
// single-value annotations above, which are first-write-wins).
//
// Built-ins mirror dbt's generic data tests; `Custom` is the escape hatch
// (dbt's singular test): a DuckDB script path whose SELECT returns the
// violating rows. The keyword is `data_test` — NOT `test` — to stay clear
// of the unrelated `// test:` CI-test annotation (see
// `windmill_common::schema::parse_ci_test_annotation`), matching dbt 1.8's
// own `tests:` → `data_tests:` rename.
#[derive(Serialize, Debug, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DataTest {
    // `// data_test unique <col>` — no two non-NULL rows share `column`.
    Unique { column: String },
    // `// data_test not_null <col>` — `column` is never NULL.
    NotNull { column: String },
    // `// data_test accepted_values <col> = a,b,c` — every non-NULL value of
    // `column` is one of `values` (comma-separated; surrounding quotes stripped).
    AcceptedValues { column: String, values: Vec<String> },
    // `// data_test relationships <col> -> <asset>.<refcol>` — referential
    // integrity: every non-NULL `column` value exists in `to_path`'s `to_column`.
    Relationships { column: String, to_kind: AssetKind, to_path: String, to_column: String },
    // `// data_test <script_path>` — escape hatch: a deployed DuckDB script
    // whose trailing SELECT returns the violating rows (non-empty ⇒ fail).
    Custom { path: String },
}

// `// column <out_col> <- <asset-uri>.<col>[, …]` — declared column-level
// lineage: one output column of this script's produced asset and the upstream
// source columns it derives from. A sibling of `DataTest` in the extensible
// annotation family (`docs/pipelines-vs-dbt.md` §3): same parse shape — a head
// token (the output column) then a per-variant tail — but accumulating, one
// line per output column. Unlike `data_test` these are pure metadata: they
// drive the column-lineage graph view, never a runtime probe.
//
// dbt derives column lineage from SQL-AST parsing; Windmill is polyglot
// (Python/TS/Bash/SQL in one DAG), so a uniform AST is not available. The
// annotation is the explicit, language-agnostic declaration — the same
// "annotations are real comments parsed strictly" stance as the rest of the
// pipeline grammar. Body-inferred per-asset column *sets* (`columns` on
// `ParseAssetsResult`) complement it but cannot express column→column edges.
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct ColumnLineage {
    // The produced asset's output column this line describes.
    pub column: String,
    // Upstream source columns it derives from (≥1; malformed refs dropped).
    pub inputs: Vec<ColumnRef>,
}

// One `<asset-uri>.<col>` upstream reference inside a `// column` line. The
// asset URI accepts the default-syntax shorthands (like `// materialize` /
// `// data_test relationships`); the column is the segment after the final
// `.` (so a schema-qualified `warehouse/main.orders.amount` keeps `amount`).
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct ColumnRef {
    pub from_kind: AssetKind,
    pub from_path: String,
    pub from_column: String,
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
    pub materialize: Option<MaterializeSpec>,
    pub data_tests: Vec<DataTest>,
    pub column_lineage: Vec<ColumnLineage>,
    pub macros: bool,
    pub use_libs: Vec<String>,
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
            materialize: pipeline.materialize,
            data_tests: pipeline.data_tests,
            column_lineage: pipeline.column_lineage,
            macros: pipeline.macros,
            use_libs: pipeline.use_libs,
        }
    }
}

// Combine column lineage inferred from the body (SQL AST) with lineage declared
// via `// column` annotations. The annotation is the *override*: where both
// describe the same output column, the explicit declaration wins and the
// inferred entry is dropped. Inferred entries are also deduped by output column
// among themselves (first wins). Used by the language asset-parsers so a
// `// column` line can correct a mis-inferred edge without disabling inference
// for the rest of the columns.
pub fn merge_column_lineage(
    inferred: Vec<ColumnLineage>,
    annotated: Vec<ColumnLineage>,
) -> Vec<ColumnLineage> {
    let mut seen: std::collections::HashSet<String> =
        annotated.iter().map(|c| c.column.clone()).collect();
    let mut out = annotated;
    for c in inferred {
        if seen.insert(c.column.clone()) {
            out.push(c);
        }
    }
    out
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
            // merge access types — a None on either side means ambiguous
            // usage (unknown access), which poisons the merge to None;
            // otherwise delegate to the shared truth table.
            existing.access_type = match (asset.access_type, existing.access_type) {
                (None, _) | (_, None) => None,
                (Some(a), Some(b)) => Some(merge_access_types(a, b)),
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
    for tok in s.split_whitespace() {
        // `split_whitespace` yields slices borrowed from `s`, so the exact
        // byte offset is the pointer delta — substring search (`find`) would
        // misfire when an earlier token also appears inside a later one.
        let tok_start = tok.as_ptr() as usize - s.as_ptr() as usize;
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

// Scan the leading comment header for pipeline annotations. Only the
// contiguous block of comment lines at the top of the file is considered
// (blank lines tolerated, scan stops at the first line of actual code) so
// that ordinary comments in the body can't false-positive as annotations.
// Language-agnostic: any header line whose first non-whitespace tokens are
// a comment prefix (`//`, `#`, or `--`) followed by one of the recognized
// keywords:
//   - `pipeline`                 → opt-in marker (must be alone on the line)
//   - `on <trigger-spec>`        → asset / native trigger edge (including
//                                  the marker-only `on schedule` form)
//   - `partitioned <kind> [opts]` → partition declaration
//   - `freshness <duration>`     → SLA window (badge + EE watchdog)
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
// Try to consume `<kw>` as a complete word from `rest`. Returns the trailing
// text after the keyword if it matched (empty or whitespace-bounded),
// `None` otherwise. Prevents `partitioned` matching `partition`, `pipelines`
// matching `pipeline`, etc. Mirrors `consumeKeyword` in
// parsePipelineAnnotations.ts.
fn consume_keyword<'a>(rest: &'a str, kw: &str) -> Option<&'a str> {
    let after = rest.strip_prefix(kw)?;
    if after.is_empty() || after.starts_with(|c: char| c.is_whitespace()) {
        Some(after)
    } else {
        None
    }
}

pub fn parse_pipeline_annotations(code: &str) -> PipelineAnnotations {
    let mut out = PipelineAnnotations::default();

    for raw_line in code.lines() {
        let line = raw_line.trim_start();
        if line.is_empty() {
            continue;
        }
        let rest = if let Some(r) = line.strip_prefix("//") {
            r
        } else if let Some(r) = line.strip_prefix("--") {
            r
        } else if let Some(r) = line.strip_prefix('#') {
            r
        } else {
            // Annotations live in the leading comment header. Stop at the first
            // line of actual code so comments inside the body (e.g. a regular
            // `# tag ...` prose comment) can't false-positive as annotations.
            // Mirrors BashAnnotations::sandbox_image / ssh_target.
            break;
        };
        let rest = rest.trim_start();

        if let Some(after_kw) = consume_keyword(rest, "pipeline") {
            // Strict: keyword must be the only content on the line. Rejects
            // `pipeline broken`, `pipelines`, `pipeline-related`, etc.
            if after_kw.trim().is_empty() {
                out.in_pipeline = true;
            }
            continue;
        }

        if let Some(after_kw) = consume_keyword(rest, "macros") {
            // Strict like `pipeline`: keyword alone on the line, so prose
            // such as `// macros are defined below` never false-positives.
            if after_kw.trim().is_empty() {
                out.macros = true;
            }
            continue;
        }

        // `// use <lib_script_path>` — accumulating. The argument must be a
        // single whitespace-free token containing `/` (all script paths do),
        // so prose like `// use this script to …` is dropped fail-safe.
        if let Some(after_kw) = consume_keyword(rest, "use") {
            let path = after_kw.trim();
            if !path.is_empty()
                && !path.contains(char::is_whitespace)
                && path.contains('/')
                && !out.use_libs.iter().any(|p| p == path)
            {
                out.use_libs.push(path.to_string());
            }
            continue;
        }

        if let Some(after_kw) = consume_keyword(rest, "partitioned") {
            if out.partition.is_none() {
                if let Some(spec) = parse_partitioned_spec(after_kw.trim()) {
                    out.partition = Some(spec);
                }
            }
            continue;
        }

        if let Some(after_kw) = consume_keyword(rest, "freshness") {
            let dur = after_kw.trim();
            if !dur.is_empty() && out.freshness.is_none() {
                out.freshness = Some(FreshnessSpec { duration: dur.to_string() });
            }
            continue;
        }

        if let Some(after_kw) = consume_keyword(rest, "trigger") {
            match after_kw.trim() {
                "all" => out.join_mode = JoinMode::All,
                "any" => out.join_mode = JoinMode::Any,
                // Unknown value — leave the default rather than guess.
                _ => {}
            }
            continue;
        }

        if let Some(after_kw) = consume_keyword(rest, "debounce") {
            let dur = after_kw.trim();
            if !dur.is_empty() && out.debounce_default.is_none() {
                out.debounce_default = Some(dur.to_string());
            }
            continue;
        }

        if let Some(after_kw) = consume_keyword(rest, "tag") {
            let name = after_kw.trim();
            // Worker tags are single-word identifiers (e.g. `heavy`, `gpu`).
            // A value with whitespace or beyond the `script.tag` column width
            // is almost certainly a regular comment starting with "# tag ...".
            if !name.is_empty()
                && !name.contains(char::is_whitespace)
                && name.len() <= 50
                && out.tag.is_none()
            {
                out.tag = Some(name.to_string());
            }
            continue;
        }

        if let Some(after_kw) = consume_keyword(rest, "retry") {
            if out.retry.is_none() {
                if let Some(spec) = parse_retry_spec(after_kw.trim()) {
                    out.retry = Some(spec);
                }
            }
            continue;
        }

        if let Some(after_kw) = consume_keyword(rest, "materialize") {
            if out.materialize.is_none() {
                if let Some(spec) = parse_materialize_spec(after_kw.trim()) {
                    out.materialize = Some(spec);
                }
            }
            continue;
        }

        // `data_test` is checked before `on`/asset shorthands and is a complete
        // word (so it never collides with the `// test:` CI annotation, which
        // has no whitespace after `test`). Accumulates — every well-formed line
        // adds a check; malformed lines are dropped (fail-safe, the missing
        // check is then simply absent from the graph + run).
        if let Some(after_kw) = consume_keyword(rest, "data_test") {
            if let Some(spec) = parse_data_test_spec(after_kw.trim()) {
                out.data_tests.push(spec);
            }
            continue;
        }

        // `// column <out> <- <src>.<col>[, …]` — accumulating column lineage.
        // A complete word, so it never swallows a body comment that happens to
        // start with `column` followed by non-lineage prose (that has no `<-`
        // and is dropped fail-safe). Checked before `on`/asset shorthands.
        if let Some(after_kw) = consume_keyword(rest, "column") {
            if let Some(spec) = parse_column_lineage_spec(after_kw.trim()) {
                out.column_lineage.push(spec);
            }
            continue;
        }

        if let Some(after_kw) = consume_keyword(rest, "on") {
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

// Parse a `// materialize [manual] <asset> [append] [key=<col>] [history]
// [track=<c1,c2>]` right-hand side. An optional leading `manual` token opts out
// of managed mode (track-only); a leading `scd2` token is an alias for the
// `history` flag. The next whitespace token is the target asset URI
// (default-syntax shorthands enabled, so `ducklake` → `ducklake://main`); the
// remainder are strategy options — bare `append`, bare `history` (SCD type-2 on
// a keyed merge), `key=<col>` (merge/scd2 key), `track=<c1,c2,…>` (scd2 tracked
// columns), and `deletes=close` (scd2 hard-delete-close) — which apply to managed
// mode only. A missing/empty target yields `None` (the annotation is dropped,
// fail-safe).
fn parse_materialize_spec(s: &str) -> Option<MaterializeSpec> {
    // One optional leading mode keyword: `manual` (escape hatch, track-only) or
    // `scd2` (alias for the `history` flag below). A missing keyword is the
    // default managed mode.
    fn strip_mode<'a>(s: &'a str, kw: &str) -> Option<&'a str> {
        s.strip_prefix(kw)
            .filter(|after| after.is_empty() || after.starts_with(char::is_whitespace))
            .map(|after| after.trim_start())
    }
    let (manual, scd2_kw, rest) = if let Some(after) = strip_mode(s, "manual") {
        (true, false, after)
    } else if let Some(after) = strip_mode(s, "scd2") {
        (false, true, after)
    } else {
        (false, false, s)
    };
    let mut it = rest.trim().splitn(2, char::is_whitespace);
    let asset_tok = it.next()?;
    let opts_str = it.next().unwrap_or("");
    let (target_kind, path) = parse_asset_syntax(asset_tok.trim(), true)?;
    if path.is_empty() {
        return None;
    }
    let append = opts_str.split_whitespace().any(|t| t == "append");
    // SCD type-2 history mode. The primary spelling is the bare `history` flag on
    // a keyed merge (`key=<col> history`) — it reads as "a keyed upsert that keeps
    // history"; the leading `scd2` keyword is a recognized alias for the same.
    let scd2 = scd2_kw || opts_str.split_whitespace().any(|t| t == "history");
    let opts = parse_kv_opts(opts_str);
    let unique_key = opts.get("key").filter(|k| !k.is_empty()).cloned();
    // `track=<c1,c2,…>` (scd2 only): comma-separated columns whose change opens a
    // new version. Empty entries dropped; an empty list ⇒ track all non-key cols.
    // Like every `=`-option here the value is whitespace-terminated, so the list
    // must contain no spaces (`track=a,b`, not `track=a, b` — the rest is dropped).
    let track = opts
        .get("track")
        .map(|v| {
            v.split(',')
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    // `deletes=close` (scd2 only) opts into hard-delete-close; any other value
    // (or absence) keeps the soft-delete default.
    let close_deleted = opts.get("deletes").map(|v| v == "close").unwrap_or(false);
    Some(MaterializeSpec {
        target_kind,
        target_path: path.to_string(),
        manual,
        append,
        unique_key,
        scd2,
        track,
        close_deleted,
    })
}

// Parse a `// data_test <kind> …` right-hand side into one `DataTest`. The
// leading token selects the variant; the remainder is parsed per-variant.
// Anything not matching a built-in keyword is the `Custom` escape hatch — a
// single script-path token. Returns `None` for malformed input so a typo
// fails safe (the check is dropped, never silently mis-parsed).
//
// This is the extension seam: a new built-in is one match arm + its parser;
// a sibling annotation family (column-lineage) reuses the same head-keyword
// dispatch shape rather than adding a parallel closed list.
fn parse_data_test_spec(s: &str) -> Option<DataTest> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut it = s.splitn(2, char::is_whitespace);
    let head = it.next()?;
    let rest = it.next().unwrap_or("").trim();
    match head {
        "unique" => Some(DataTest::Unique { column: single_ident(rest)? }),
        "not_null" => Some(DataTest::NotNull { column: single_ident(rest)? }),
        "accepted_values" => parse_accepted_values(rest),
        "relationships" => parse_relationships(rest),
        // Custom escape hatch: the whole right-hand side must be one path token
        // (`head` with no trailing content). Trailing content after a
        // non-built-in head is a malformed built-in (e.g. `uniq order_id`) and
        // is rejected rather than misread as a path.
        _ if rest.is_empty() => Some(DataTest::Custom { path: head.to_string() }),
        _ => None,
    }
}

// A single bare identifier token (column name). Rejects empty / multi-token
// input. The identifier is double-quoted + escaped at codegen, so any
// character is safe here; we only enforce "exactly one token".
fn single_ident(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s.split_whitespace().count() != 1 {
        return None;
    }
    Some(s.to_string())
}

// Strip one layer of matching surrounding single or double quotes.
fn unquote(s: &str) -> &str {
    let b = s.as_bytes();
    if b.len() >= 2 && (b[0] == b'"' || b[0] == b'\'') && b[b.len() - 1] == b[0] {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// `<col> = a,b,c` — column, then `=`, then a comma-separated value list.
// Surrounding quotes are stripped per value; empty values are dropped; a
// value may not itself contain a comma (v1 limitation).
fn parse_accepted_values(s: &str) -> Option<DataTest> {
    let (col, vals) = s.split_once('=')?;
    let column = single_ident(col)?;
    let values: Vec<String> = vals
        .split(',')
        .map(|v| unquote(v.trim()).to_string())
        .filter(|v| !v.is_empty())
        .collect();
    if values.is_empty() {
        return None;
    }
    Some(DataTest::AcceptedValues { column, values })
}

// `<col> -> <asset-uri>.<refcol>` — referential integrity. The referenced
// column is the segment after the final `.`; everything before it is the
// asset URI (default-syntax shorthands enabled, like `// materialize`).
fn parse_relationships(s: &str) -> Option<DataTest> {
    let (col, target) = s.split_once("->")?;
    let column = single_ident(col)?;
    let target = target.trim();
    let (asset_uri, ref_col) = target.rsplit_once('.')?;
    let to_column = single_ident(ref_col)?;
    let (to_kind, to_path) = parse_asset_syntax(asset_uri.trim(), true)?;
    if to_path.is_empty() {
        return None;
    }
    Some(DataTest::Relationships { column, to_kind, to_path: to_path.to_string(), to_column })
}

// Parse a `// column <out_col> <- <ref>[, <ref> …]` right-hand side. The head
// (before `<-`) is the output column; the tail is a comma-separated list of
// `<asset-uri>.<col>` upstream references. Mirrors `parse_accepted_values`'
// "drop empties, require ≥1" stance: individually malformed refs are dropped
// and the line is kept iff at least one ref parses; a missing `<-`, a non-ident
// output column, or zero valid refs drops the whole line (fail-safe).
fn parse_column_lineage_spec(s: &str) -> Option<ColumnLineage> {
    let (out_col, refs) = s.split_once("<-")?;
    let column = single_ident(out_col)?;
    let inputs: Vec<ColumnRef> = refs
        .split(',')
        .filter_map(|r| parse_column_ref(r.trim()))
        .collect();
    if inputs.is_empty() {
        return None;
    }
    Some(ColumnLineage { column, inputs })
}

// `<asset-uri>.<col>` — the referenced column is the segment after the final
// `.`; everything before it is the asset URI (default-syntax shorthands
// enabled, like `// materialize`). Same shape as `parse_relationships`' target.
fn parse_column_ref(s: &str) -> Option<ColumnRef> {
    let (asset_uri, ref_col) = s.rsplit_once('.')?;
    let from_column = single_ident(ref_col)?;
    let (from_kind, from_path) = parse_asset_syntax(asset_uri.trim(), true)?;
    if from_path.is_empty() {
        return None;
    }
    Some(ColumnRef { from_kind, from_path: from_path.to_string(), from_column })
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
    fn macros_marker_strict_like_pipeline() {
        assert!(parse_pipeline_annotations("// macros\nCREATE MACRO m(a) AS a;").macros);
        assert!(parse_pipeline_annotations("-- macros   \nSELECT 1;").macros);
        // Trailing prose / keyword variants disqualify the line.
        assert!(!parse_pipeline_annotations("// macros are defined below\n").macros);
        assert!(!parse_pipeline_annotations("// macros_v2\n").macros);
    }

    #[test]
    fn use_accumulates_dedups_and_rejects_prose() {
        let out = parse_pipeline_annotations(
            "// use f/lib/stats\n// use f/lib/dates\n// use f/lib/stats\nSELECT 1;",
        );
        assert_eq!(out.use_libs, vec!["f/lib/stats", "f/lib/dates"]);

        // Prose, slashless tokens, and multi-token lines are dropped fail-safe.
        let out = parse_pipeline_annotations(
            "// use this script to compute\n// use standalone\n// use f/lib/ok extra\n",
        );
        assert!(out.use_libs.is_empty());

        // Only the leading comment header is scanned.
        let out = parse_pipeline_annotations("SELECT 1;\n-- use f/lib/late\n");
        assert!(out.use_libs.is_empty());
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
             // on schedule",
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
    fn tag_with_whitespace_is_skipped() {
        // A regular English comment starting with "# tag " must not be
        // mistaken for a worker-tag annotation (worker tags are single words).
        let out =
            parse_pipeline_annotations("# tag this function so we remember to refactor it later");
        assert!(out.tag.is_none());
    }

    #[test]
    fn tag_too_long_is_skipped() {
        let long = "x".repeat(51);
        let out = parse_pipeline_annotations(&format!("// tag {long}"));
        assert!(out.tag.is_none());
    }

    #[test]
    fn annotations_in_body_are_ignored() {
        // Only the leading comment header is scanned. A regular `# tag ...`
        // prose comment buried in the body — the WIN-2090 false-positive that
        // crashed the `script.tag` INSERT — must not be treated as an
        // annotation once real code has started.
        let code = concat!(
            "import pandas as pd\n",
            "\n",
            "def main():\n",
            "    # tag each row with its source so downstream steps can filter\n",
            "    # on s3://should/not/parse\n",
            "    return pd.DataFrame()\n",
        );
        let out = parse_pipeline_annotations(code);
        assert!(out.tag.is_none());
        assert!(out.triggers.is_empty());
    }

    #[test]
    fn header_allows_blank_lines_before_code() {
        // Blank lines (e.g. after a shebang) don't end the header; the first
        // line of real code does.
        let code = concat!(
            "#!/usr/bin/env python\n",
            "\n",
            "# tag heavy\n",
            "import os\n",
            "# tag light\n",
        );
        let out = parse_pipeline_annotations(code);
        assert_eq!(out.tag.as_deref(), Some("heavy"));
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
    fn materialize_managed_default() {
        let out = parse_pipeline_annotations("// materialize ducklake://analytics/orders_daily");
        let m = out.materialize.expect("materialize");
        assert_eq!(m.target_kind, AssetKind::Ducklake);
        assert_eq!(m.target_path, "analytics/orders_daily");
        // managed by default; replace strategy (no append / key)
        assert!(!m.manual);
        assert!(!m.append);
        assert_eq!(m.unique_key, None);
    }

    #[test]
    fn materialize_manual_escape_hatch() {
        let out =
            parse_pipeline_annotations("// materialize manual ducklake://analytics/orders_daily");
        let m = out.materialize.expect("materialize");
        assert!(m.manual);
        assert_eq!(m.target_path, "analytics/orders_daily");
    }

    #[test]
    fn materialize_merge_and_append_options() {
        let out =
            parse_pipeline_annotations("// materialize ducklake://a/orders_daily key=order_id");
        let m = out.materialize.expect("materialize");
        assert_eq!(m.unique_key.as_deref(), Some("order_id"));
        assert!(!m.append);

        let out = parse_pipeline_annotations("// materialize ducklake://a/events append");
        let m = out.materialize.expect("materialize");
        assert!(m.append);
        assert_eq!(m.unique_key, None);
    }

    #[test]
    fn materialize_scd2_history_flag_with_key_and_track() {
        // Primary spelling: `key=<col> history` on a merge.
        let out = parse_pipeline_annotations(
            "// materialize ducklake://a/dim key=id history track=name,tier",
        );
        let m = out.materialize.expect("materialize");
        assert!(m.scd2);
        assert!(!m.manual);
        assert_eq!(m.unique_key.as_deref(), Some("id"));
        assert_eq!(m.track, vec!["name".to_string(), "tier".to_string()]);
    }

    #[test]
    fn materialize_scd2_keyword_is_alias_for_history() {
        let out = parse_pipeline_annotations("// materialize scd2 ducklake://a/dim key=id");
        let m = out.materialize.expect("materialize");
        assert!(m.scd2);
        assert_eq!(m.unique_key.as_deref(), Some("id"));
        assert!(m.track.is_empty());
        // soft-delete default
        assert!(!m.close_deleted);
    }

    #[test]
    fn materialize_scd2_deletes_close_opt() {
        let out = parse_pipeline_annotations(
            "// materialize ducklake://a/dim key=id history deletes=close",
        );
        let m = out.materialize.expect("materialize");
        assert!(m.scd2);
        assert!(m.close_deleted);
        // any other value keeps the soft-delete default
        let out = parse_pipeline_annotations(
            "// materialize ducklake://a/dim key=id history deletes=ignore",
        );
        assert!(!out.materialize.expect("materialize").close_deleted);
    }

    #[test]
    fn materialize_key_without_history_is_plain_merge() {
        let out = parse_pipeline_annotations("// materialize ducklake://a/dim key=id");
        let m = out.materialize.expect("materialize");
        assert!(!m.scd2, "no history flag ⇒ SCD1 merge, not scd2");
        assert_eq!(m.unique_key.as_deref(), Some("id"));
    }

    #[test]
    fn materialize_default_syntax_shorthand() {
        let out = parse_pipeline_annotations("// materialize ducklake");
        let m = out.materialize.expect("materialize");
        assert_eq!(m.target_kind, AssetKind::Ducklake);
        assert_eq!(m.target_path, "main");
        assert!(!m.manual);
    }

    #[test]
    fn materialize_manual_only_is_dropped() {
        // `manual` with no target is not a valid materialization.
        let out = parse_pipeline_annotations("// materialize manual");
        assert!(out.materialize.is_none());
    }

    #[test]
    fn materialize_first_wins() {
        let out = parse_pipeline_annotations(
            "// materialize ducklake://a/x\n# materialize manual ducklake://b/y",
        );
        let m = out.materialize.expect("materialize");
        assert_eq!(m.target_path, "a/x");
        assert!(!m.manual);
    }

    #[test]
    fn combined() {
        let code = concat!(
            "// pipeline\n",
            "// on schedule\n",
            "// on s3://in.csv\n",
            "// partitioned daily tz=\"UTC\"\n",
            "// freshness 2h\n",
            "// tag heavy\n",
            "// retry 3 5s\n",
            "// materialize ducklake://analytics/orders_daily key=order_id\n"
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
        let m = out.materialize.expect("materialize");
        assert!(!m.manual);
        assert_eq!(m.target_path, "analytics/orders_daily");
        assert_eq!(m.unique_key.as_deref(), Some("order_id"));
    }

    #[test]
    fn split_trailing_kv_opts_uses_exact_token_offset() {
        // Regression for the token-offset computation in
        // `split_trailing_kv_opts`. The asset ref's path token contains the
        // exact text of the trailing `debounce=60s` opt as a substring, and
        // the same `debounce=60s` text also appears a second time before the
        // real opt. Offsetting by the `&str` slice's pointer is exact; a
        // substring scan (`find`) is what this guards against regressing to.
        let (ref_part, opts) = split_trailing_kv_opts("s3://lake/debounce=60s/raw debounce=60s");
        assert_eq!(ref_part, "s3://lake/debounce=60s/raw");
        assert_eq!(opts.get("debounce").map(String::as_str), Some("60s"));

        // End-to-end through the annotation parser: the ref must parse to the
        // full S3 path (not truncated at the embedded `debounce=`), and the
        // per-edge debounce override must be picked up from the trailing opt.
        let out = parse_pipeline_annotations("// on s3://lake/debounce=60s/raw debounce=90s");
        assert_eq!(out.triggers.len(), 1);
        match &out.triggers[0] {
            TriggerSpec::Asset { asset_kind, path, debounce } => {
                assert_eq!(*asset_kind, AssetKind::S3Object);
                assert_eq!(path, "lake/debounce=60s/raw");
                assert_eq!(debounce.as_deref(), Some("90s"));
            }
            other => panic!("expected asset trigger, got {other:?}"),
        }
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

    #[test]
    fn data_test_builtins() {
        let code = concat!(
            "// data_test unique order_id\n",
            "// data_test not_null user_id\n",
            "// data_test accepted_values status = paid,pending,refunded\n",
            "// data_test relationships user_id -> datatable://prod/users.id\n",
        );
        let out = parse_pipeline_annotations(code);
        assert_eq!(
            out.data_tests,
            vec![
                DataTest::Unique { column: "order_id".to_string() },
                DataTest::NotNull { column: "user_id".to_string() },
                DataTest::AcceptedValues {
                    column: "status".to_string(),
                    values: vec![
                        "paid".to_string(),
                        "pending".to_string(),
                        "refunded".to_string()
                    ],
                },
                DataTest::Relationships {
                    column: "user_id".to_string(),
                    to_kind: AssetKind::DataTable,
                    to_path: "prod/users".to_string(),
                    to_column: "id".to_string(),
                },
            ]
        );
    }

    #[test]
    fn data_test_accepts_quotes_and_spacing() {
        let out = parse_pipeline_annotations("// data_test accepted_values kind = \"a b\", 'c' ,d");
        assert_eq!(
            out.data_tests,
            vec![DataTest::AcceptedValues {
                column: "kind".to_string(),
                values: vec!["a b".to_string(), "c".to_string(), "d".to_string()],
            }]
        );
    }

    #[test]
    fn data_test_custom_escape_hatch() {
        // A non-built-in single token is a custom script path; default-syntax
        // asset shorthands are NOT triggered here (a path is just a path).
        let out = parse_pipeline_annotations("// data_test f/tests/orders_amount_sane");
        assert_eq!(
            out.data_tests,
            vec![DataTest::Custom { path: "f/tests/orders_amount_sane".to_string() }]
        );
    }

    #[test]
    fn data_test_relationships_ducklake_shorthand() {
        let out = parse_pipeline_annotations(
            "// data_test relationships sku -> ducklake://warehouse/dim_products.sku",
        );
        assert_eq!(
            out.data_tests,
            vec![DataTest::Relationships {
                column: "sku".to_string(),
                to_kind: AssetKind::Ducklake,
                to_path: "warehouse/dim_products".to_string(),
                to_column: "sku".to_string(),
            }]
        );
    }

    #[test]
    fn data_test_malformed_dropped_fail_safe() {
        // A misspelled built-in with trailing content is not a valid path token
        // → dropped, not misread as a custom test. An empty value list, a
        // missing arrow target, and a bare keyword are all dropped too.
        let out = parse_pipeline_annotations(concat!(
            "// data_test uniq order_id\n",        // typo'd built-in + arg
            "// data_test accepted_values s =\n",  // no values
            "// data_test relationships a -> b\n", // no `.refcol`
            "// data_test unique\n",               // missing column
            "// data_test\n",                      // bare keyword
        ));
        assert!(out.data_tests.is_empty());
    }

    #[test]
    fn data_test_not_confused_with_ci_test_annotation() {
        // `// test:` is the unrelated CI-test annotation — it must NOT be
        // parsed as a data test (no whitespace after `test`, and the keyword
        // is `data_test` anyway).
        let out = parse_pipeline_annotations("// test: f/foo/bar\n// data_test unique id");
        assert_eq!(
            out.data_tests,
            vec![DataTest::Unique { column: "id".to_string() }]
        );
    }

    #[test]
    fn column_lineage_basic() {
        let code = concat!(
            "// column order_total <- ducklake://warehouse/orders.amount, ducklake://warehouse/orders.tax\n",
            "// column user_name <- datatable://prod/users.name\n",
        );
        let out = parse_pipeline_annotations(code);
        assert_eq!(
            out.column_lineage,
            vec![
                ColumnLineage {
                    column: "order_total".to_string(),
                    inputs: vec![
                        ColumnRef {
                            from_kind: AssetKind::Ducklake,
                            from_path: "warehouse/orders".to_string(),
                            from_column: "amount".to_string(),
                        },
                        ColumnRef {
                            from_kind: AssetKind::Ducklake,
                            from_path: "warehouse/orders".to_string(),
                            from_column: "tax".to_string(),
                        },
                    ],
                },
                ColumnLineage {
                    column: "user_name".to_string(),
                    inputs: vec![ColumnRef {
                        from_kind: AssetKind::DataTable,
                        from_path: "prod/users".to_string(),
                        from_column: "name".to_string(),
                    }],
                },
            ]
        );
    }

    #[test]
    fn column_lineage_schema_qualified_keeps_last_dot_as_column() {
        // The column is the segment after the FINAL dot, so a schema-qualified
        // ducklake table (`main.dim_products`) survives intact.
        let out = parse_pipeline_annotations(
            "// column sku <- ducklake://warehouse/main.dim_products.sku",
        );
        assert_eq!(
            out.column_lineage,
            vec![ColumnLineage {
                column: "sku".to_string(),
                inputs: vec![ColumnRef {
                    from_kind: AssetKind::Ducklake,
                    from_path: "warehouse/main.dim_products".to_string(),
                    from_column: "sku".to_string(),
                }],
            }]
        );
    }

    #[test]
    fn column_lineage_drops_malformed_refs_keeps_valid() {
        // `bad_no_dot` has no `.col` and is dropped; the line survives on its
        // one valid ref. Mirrors accepted_values' drop-empties-keep-≥1 stance.
        let out = parse_pipeline_annotations(
            "// column total <- bad_no_dot, datatable://prod/orders.amount",
        );
        assert_eq!(
            out.column_lineage,
            vec![ColumnLineage {
                column: "total".to_string(),
                inputs: vec![ColumnRef {
                    from_kind: AssetKind::DataTable,
                    from_path: "prod/orders".to_string(),
                    from_column: "amount".to_string(),
                }],
            }]
        );
    }

    #[test]
    fn merge_column_lineage_annotation_overrides_inferred() {
        let inferred = vec![
            ColumnLineage {
                column: "total".to_string(),
                inputs: vec![ColumnRef {
                    from_kind: AssetKind::Ducklake,
                    from_path: "w/o".to_string(),
                    from_column: "amount".to_string(),
                }],
            },
            ColumnLineage {
                column: "qty".to_string(),
                inputs: vec![ColumnRef {
                    from_kind: AssetKind::Ducklake,
                    from_path: "w/o".to_string(),
                    from_column: "qty".to_string(),
                }],
            },
        ];
        // Annotation redefines `total` (wins) and leaves `qty` to inference.
        let annotated = vec![ColumnLineage {
            column: "total".to_string(),
            inputs: vec![ColumnRef {
                from_kind: AssetKind::DataTable,
                from_path: "prod/x".to_string(),
                from_column: "grand_total".to_string(),
            }],
        }];
        let merged = merge_column_lineage(inferred, annotated);
        assert_eq!(merged.len(), 2);
        // Annotation entry kept first and authoritative.
        assert_eq!(merged[0].column, "total");
        assert_eq!(merged[0].inputs[0].from_column, "grand_total");
        // Inferred `qty` survives (no annotation for it); inferred `total` dropped.
        assert_eq!(merged[1].column, "qty");
    }

    #[test]
    fn column_lineage_malformed_lines_dropped_fail_safe() {
        // No arrow, a multi-token output column, and a line whose every ref is
        // malformed are all dropped entirely.
        let out = parse_pipeline_annotations(concat!(
            "// column no_arrow datatable://prod/x.y\n", // missing `<-`
            "// column a b <- datatable://prod/x.y\n",   // output not a single ident
            "// column total <- bad_no_dot\n",           // no valid ref
            "// column\n",                               // bare keyword
        ));
        assert!(out.column_lineage.is_empty());
    }
}
