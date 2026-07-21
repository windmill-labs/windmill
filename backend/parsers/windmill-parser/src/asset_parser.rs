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
    // `// measure <name> = <agg> [where <pred>]` — table-scoped aggregations of
    // the produced asset. Accumulating, deduped by name. Catalogued at deploy;
    // executes nothing.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub measures: Vec<Measure>,
    // `// dimension <name> = <expr>` — table-scoped slicers any measure can be
    // grouped by. Accumulating, deduped by name.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub dimensions: Vec<Dimension>,
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

impl PartitionKind {
    /// The `strftime`/chrono format that renders a time grain's identity
    /// string. SINGLE SOURCE OF TRUTH: the partition resolver stamps the stored
    /// identity with this, and the `wm_partition` materialize macro filters
    /// against it — the two must never drift, so both read it from here.
    /// `Dynamic` has no wall-clock identity; it falls back to a plain date only
    /// where some format is unconditionally required (never for bucketing).
    pub fn default_time_format(&self) -> &'static str {
        match self {
            PartitionKind::Hourly => "%Y-%m-%dT%H",
            PartitionKind::Weekly => "%G-W%V",
            PartitionKind::Monthly => "%Y-%m",
            _ => "%Y-%m-%d",
        }
    }
}

impl PartitionSpec {
    /// Effective identity format for a TIME partition: the explicit `format=`
    /// override, else the per-grain default. `None` for `dynamic` — its
    /// identity is a caller-supplied key, not a formatted instant, so there is
    /// no `strftime` bucketing expression (and hence no `wm_partition` macro).
    pub fn time_strftime_format(&self) -> Option<&str> {
        match self.kind {
            PartitionKind::Dynamic { .. } => None,
            _ => Some(
                self.format
                    .as_deref()
                    .unwrap_or_else(|| self.kind.default_time_format()),
            ),
        }
    }
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
// `on_schema_change=ignore` suppresses downstream schema-contract warnings for
// the produced asset (save-time metadata only; default `warn`).
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
    // `on_schema_change=warn|ignore|fail|sync` governs two orthogonal things:
    //   • Save-time contract warnings (gap #2b): consumers referencing columns
    //     the captured schema no longer has warn by default; only `ignore`
    //     suppresses those warnings (`warn`/`fail`/`sync` all keep them).
    //   • Run-time write guardrails for the persist-and-mutate strategies
    //     (partitioned replace, merge, append), where the table schema is fixed
    //     at first CREATE and the write is positional — a renamed/added/removed
    //     SELECT column silently lands in the wrong column. `warn` logs the
    //     drift and proceeds positionally; `fail` aborts before mutating; `sync`
    //     ALTERs the table to match and writes by name. Whole-table replace and
    //     scd2 are unaffected. See `sql_materialize.rs`.
    #[serde(skip_serializing_if = "OnSchemaChange::is_warn", default)]
    pub on_schema_change: OnSchemaChange,
}

impl MaterializeSpec {
    /// The `<target>_current` SCD2 companion view this managed materialize also
    /// produces, or `None` when it isn't a managed scd2 target. Managed scd2
    /// creates the base table *and* a `<dim>_current` "latest row per key" view
    /// each run (see `sql_materialize.rs`); `manual` mode owns its own DDL and
    /// short-circuits before that codegen, so it produces no companion.
    pub fn scd2_current_target(&self) -> Option<(AssetKind, String)> {
        (self.scd2 && !self.manual)
            .then(|| (self.target_kind, format!("{}_current", self.target_path)))
    }

    /// Every asset this managed materialize produces: the base table, plus — for
    /// managed scd2 — the `<target>_current` companion view. The producer's
    /// trailing `SELECT` doesn't express these writes (the runtime generates the
    /// DDL), so this is the single source of truth every graph surface (deploy
    /// asset rows, the CLI `--local` graph, and the frontend live graph) uses to
    /// link reads of the base *and* the `_current` view back to this producer.
    pub fn write_targets(&self) -> Vec<(AssetKind, String)> {
        let mut targets = vec![(self.target_kind, self.target_path.clone())];
        targets.extend(self.scd2_current_target());
        targets
    }
}

// dbt's `on_schema_change`, covering both the save-time contract check and the
// run-time write guardrail for the positional persist-and-mutate strategies:
//   • `warn` (default): surface consumer contract warnings; at write time, log
//     the drift loudly and proceed with the positional write against the fixed
//     table schema.
//   • `ignore`: suppress consumer contract warnings; at write time, no guard
//     (the pre-guardrail behaviour).
//   • `fail`: keep contract warnings; at write time, abort the run before
//     mutating when the SELECT's column *set* diverges from the table's.
//   • `sync`: keep contract warnings; at write time, ALTER the table to match
//     the SELECT (add/drop columns) and INSERT BY NAME.
// `warn`/`fail` drift detection is name-set based (added/removed columns), which
// is what the positional persist-and-mutate INSERT can misalign on. It does NOT
// flag a pure *reorder* of same-named columns: `SELECT b, a` into a `(a, b)`
// table has an identical column set, so `fail` does not abort and the positional
// INSERT swaps the values. Reorder-safety is exactly what `sync` provides
// (INSERT BY NAME maps by name), so a SELECT whose column order is not pinned to
// the table's should use `sync`, not `fail`. (An ordered-list comparison would
// close this, but a false positive there would abort a correctly-aligned write,
// so the guard stays on the set difference.)
// `fail`/`sync` only affect partitioned replace, merge and append; whole-table
// replace already rebuilds each run, and scd2 has no positional write — for an
// scd2 target `sync` degrades to `warn` (no write-time effect; deploy-time
// rejection is out of scope here).
#[derive(Serialize, Debug, PartialEq, Eq, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum OnSchemaChange {
    #[default]
    Warn,
    Ignore,
    Fail,
    Sync,
}

impl OnSchemaChange {
    pub fn is_warn(&self) -> bool {
        matches!(self, OnSchemaChange::Warn)
    }
}

impl MaterializeSpec {
    /// Deploy-time validation of the option combination against the script's
    /// partitioning, returning a human-facing error for combinations the
    /// runtime cannot honor. Called at save (`create_script_internal`) so a
    /// misconfigured script is rejected up front, and again in the DuckDB
    /// executor as a safety net for preview/test runs that never deploy. Both
    /// checks are SCD2-specific and inert for `manual` mode (which owns its DDL
    /// and ignores the reconciliation strategy). `partitioned` is whether the
    /// script declares `// partitioned`.
    pub fn validate(&self, partitioned: bool) -> Result<(), String> {
        if self.manual || !self.scd2 {
            return Ok(());
        }
        // SCD2 needs a natural key to identify an entity across versions.
        if self.unique_key.as_deref().map_or(true, str::is_empty) {
            return Err(
                "materialize scd2: requires a natural key — add `key=<col>` (e.g. \
                 `// materialize ducklake://<name>/<table> key=id history`)"
                    .to_string(),
            );
        }
        // SCD2's diff/close/open shape has no partition-scoped form in v1.
        if partitioned {
            return Err(
                "materialize scd2: `// partitioned` is not supported with scd2 in v1 — remove \
                 `// partitioned`, or drop `history`/`scd2` to materialize the partition without \
                 history"
                    .to_string(),
            );
        }
        Ok(())
    }
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

// The two table-scoped metric primitives, declared on the script that
// materializes the table (`docs/pipeline-metrics-layer.md`). A *measure* is an
// aggregation; a *dimension* is a slicer. Table-scoped, not per-measure: a
// dimension belongs to the table, so every measure can be sliced by every
// dimension. Metadata only: these are catalogued at deploy and read back by
// editors and agents, which write their own SQL. Names are validated at deploy
// against the producer's captured schema, the same machinery as `// column`.
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Measure {
    pub name: String,
    // Aggregate SQL over the table's columns (`sum(amount)`). Trusted author text.
    pub expr: String,
    // Optional row predicate from a trailing `where`. Kept separate from `expr`
    // so a reader can render it as an aggregate `FILTER (WHERE …)`, letting two
    // measures with different filters share one GROUP BY (a shared `WHERE`
    // cannot express that).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Dimension {
    pub name: String,
    // Slicing expression over the table's columns (`region`,
    // `date_trunc('month', ordered_at)`). Trusted author text.
    pub expr: String,
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
    pub measures: Vec<Measure>,
    pub dimensions: Vec<Dimension>,
    pub macros: bool,
    pub use_libs: Vec<String>,
    // `// mute <asset>` — suppress the auto-derived cascade edge for a read
    // that would otherwise trigger this script (a lookup / slowly-changing
    // dimension you read every run but don't want to re-run on). Only Asset
    // specs are stored; native trigger kinds are never auto-derived, so
    // muting them is meaningless.
    pub mute: Vec<TriggerSpec>,
    // `// mute all` — opt out of auto-derivation entirely for this script.
    // Falls back to explicit-`// on`-only semantics. Explicit `// on` edges
    // are unaffected.
    pub mute_all: bool,
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
            measures: pipeline.measures,
            dimensions: pipeline.dimensions,
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
            // Canonicalize S3 keys to a single asset identity. The SDK object
            // form (`{ s3: "key" }` / `S3Object(s3="key")`, default storage)
            // resolves to `s3:///key`, whose path is `/key`, while DuckDB
            // `s3://key` and `// on s3://key` yield the bare `key`. Strip every
            // leading slash so the triple-slash default-storage form and the
            // `s3://storage/key` form share one path — otherwise a TS/Python
            // writer and a DuckDB reader of the same object become disconnected
            // nodes in the pipeline graph. Stripping ALL leading slashes (not
            // just one) keeps the identity stable through URI reconstruction:
            // `trigger_spec_to_row` rebuilds `s3://<path>`, so a canonical path
            // must never itself start with `/` or the rebuilt ref would parse
            // back to a different key. Only leading slashes are touched, so
            // Hive-partition keys (`s3://b/y=2024/f.parquet`) are untouched.
            let path = if matches!(kind, AssetKind::S3Object) {
                path.trim_start_matches('/')
            } else {
                path
            };
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

        // `// mute all` opts out of auto-derived cascade edges entirely;
        // `// mute <asset>` suppresses the one edge. Only asset refs are
        // muteable — native trigger kinds are never auto-derived. Checked
        // before the generic `on`/asset shorthand (a complete word, so
        // prose like `// muted for now` never matches).
        if let Some(after_kw) = consume_keyword(rest, "mute") {
            let arg = after_kw.trim();
            if arg == "all" {
                out.mute_all = true;
            } else if let Some(spec @ TriggerSpec::Asset { .. }) = parse_trigger_spec(arg) {
                if !out.mute.contains(&spec) {
                    out.mute.push(spec);
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

        // `// measure <name> = <agg> [where <pred>]` / `// dimension <name> =
        // <expr>` — metrics primitives. Complete words (checked before the
        // `on`/asset shorthands), accumulating, deduped by name (first wins).
        // Malformed lines drop fail-safe like the rest of the annotation family.
        if let Some(after_kw) = consume_keyword(rest, "measure") {
            if let Some(spec) = parse_measure_spec(after_kw.trim()) {
                if !out.measures.iter().any(|m| m.name == spec.name) {
                    out.measures.push(spec);
                }
            }
            continue;
        }

        if let Some(after_kw) = consume_keyword(rest, "dimension") {
            if let Some(spec) = parse_dimension_spec(after_kw.trim()) {
                if !out.dimensions.iter().any(|d| d.name == spec.name) {
                    out.dimensions.push(spec);
                }
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

// Count `// data_test <…>` lines in the leading comment header whose right-hand
// side fails to parse into a check. `parse_pipeline_annotations` drops these
// fail-safe (a malformed line just yields no check), which is the wrong default
// for a data-quality assertion: a typo silently disables the test. The deploy
// path uses this count to warn. Same leading-block boundary as the parser (stop
// at the first non-comment line) so a body comment can't be miscounted, and the
// same `parse_data_test_spec` grammar so "malformed" means exactly what the
// parser rejects — no second grammar to drift.
pub fn count_malformed_data_tests(code: &str) -> usize {
    let mut malformed = 0;
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
            break;
        };
        if let Some(after_kw) = consume_keyword(rest.trim_start(), "data_test") {
            if parse_data_test_spec(after_kw.trim()).is_none() {
                malformed += 1;
            }
        }
    }
    malformed
}

// Count `// measure` / `// dimension` header lines whose right-hand side fails
// to parse, returning `(malformed_measures, malformed_dimensions)`. Same
// fail-safe drop as the rest of the family (a typo silently omits the metric),
// so the deploy path warns off this count. Same leading-block boundary and
// grammar as `parse_pipeline_annotations`, so "malformed" means exactly what the
// parser rejects.
pub fn count_malformed_metric_annotations(code: &str) -> (usize, usize) {
    let (mut measures, mut dimensions) = (0, 0);
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
            break;
        };
        let rest = rest.trim_start();
        if let Some(after_kw) = consume_keyword(rest, "measure") {
            if parse_measure_spec(after_kw.trim()).is_none() {
                measures += 1;
            }
        } else if let Some(after_kw) = consume_keyword(rest, "dimension") {
            if parse_dimension_spec(after_kw.trim()).is_none() {
                dimensions += 1;
            }
        }
    }
    (measures, dimensions)
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
    // `on_schema_change=ignore|fail|sync`; any other value (or absence) keeps
    // the `warn` default, fail-safe like `deletes=` above (a typo must never
    // silently disable the guardrail).
    let on_schema_change = match opts.get("on_schema_change").map(String::as_str) {
        Some("ignore") => OnSchemaChange::Ignore,
        Some("fail") => OnSchemaChange::Fail,
        Some("sync") => OnSchemaChange::Sync,
        _ => OnSchemaChange::Warn,
    };
    Some(MaterializeSpec {
        target_kind,
        target_path: path.to_string(),
        manual,
        append,
        unique_key,
        scd2,
        track,
        close_deleted,
        on_schema_change,
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

// `// measure <name> = <agg> [where <pred>]`. The name is a single identifier;
// the first `=` separates it from the body (SQL comparison operators live after
// it, so splitting on the first `=` is unambiguous). A whitespace-bounded
// ` where ` splits the aggregate from its row filter — a `where` buried in a
// string literal or subquery would mis-split, at which point the compiled SQL
// fails at DuckDB parse (fail-loud, not silently wrong). An empty aggregate is
// rejected. Modifier metadata (format/label/additivity) is deferred: `|` and
// `||` are valid SQL operators, so a modifier delimiter must not collide with a
// measure body — that grammar is chosen when the explorer needs the fields.
fn parse_measure_spec(s: &str) -> Option<Measure> {
    let (name_part, body) = s.split_once('=')?;
    let name = single_ident(name_part)?;
    let (expr, filter) = split_measure_filter(body.trim());
    let expr = expr.trim();
    // A bare trailing `where` with no predicate is a typo, not an unfiltered
    // measure: reject the whole line (counted malformed) rather than emit
    // `sum(amount) where` as the expression.
    if expr.is_empty() || matches!(&filter, Some(f) if f.is_empty()) {
        return None;
    }
    Some(Measure { name, expr: expr.to_string(), filter })
}

// `// dimension <name> = <expr>`. Name is a single identifier; the expression is
// trusted SQL emitted verbatim into SELECT and GROUP BY. No filter clause.
fn parse_dimension_spec(s: &str) -> Option<Dimension> {
    let (name_part, expr) = s.split_once('=')?;
    let name = single_ident(name_part)?;
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }
    Some(Dimension { name, expr: expr.to_string() })
}

// Split a measure body on the first top-level whitespace-bounded ` where` into
// (aggregate, filter). "Top-level" = outside string/quoted-identifier literals and
// outside parentheses, so `count_if(note = 'some where value')` is not split on the
// `where` buried in its string. `to_ascii_lowercase` is byte-length-preserving, so
// an offset in the lowercased copy indexes the original; the predicate keeps its
// source casing. A trailing ` where` yields an empty predicate, which the caller
// rejects, so `sum(amount) where` is not read as a filterless aggregate.
fn split_measure_filter(body: &str) -> (&str, Option<String>) {
    // Byte scan, comparing on the ASCII-lowercased bytes: `to_ascii_lowercase` is
    // byte-length-preserving, and byte-slice matching never panics on a non-UTF-8
    // boundary the way `lower[i..]` would for a non-ASCII body (`sum(x) + π`).
    let lower = body.to_ascii_lowercase();
    let lb = lower.as_bytes();
    let b = body.as_bytes();
    let (mut in_single, mut in_double) = (false, false);
    let mut depth: i32 = 0;
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i];
        if in_single {
            if c == b'\'' {
                // A doubled quote is an escaped one, not the end.
                if b.get(i + 1) == Some(&b'\'') {
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if c == b'"' {
                if b.get(i + 1) == Some(&b'"') {
                    i += 2;
                    continue;
                }
                in_double = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'\'' => in_single = true,
            b'"' => in_double = true,
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ if depth == 0 && lb[i..].starts_with(b" where") => {
                let after = i + b" where".len();
                // Whitespace-bounded: end of input, or a space follows (so a token
                // like ` wherever` is not matched). `i` is the space before `where`
                // and `after` is just past it — both ASCII, so the slices are valid.
                if after == b.len() || b[after].is_ascii_whitespace() {
                    return (&body[..i], Some(body[after..].trim().to_string()));
                }
            }
            _ => {}
        }
        i += 1;
    }
    (body, None)
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
    fn s3_key_normalization_unifies_uri_forms() {
        // A TS/Python SDK write of `{ s3: "exports/x" }` (default storage)
        // resolves to the URI `s3:///exports/x`, while a DuckDB read of
        // `s3://exports/x` and the `// on s3://exports/x` trigger form yield the
        // bare `exports/x`. All three must canonicalize to one asset key so
        // the writer and reader connect in the pipeline graph.
        let sdk_write = parse_asset_syntax("s3:///exports/x", false);
        let duckdb_read = parse_asset_syntax("s3://exports/x", false);
        assert_eq!(sdk_write, Some((AssetKind::S3Object, "exports/x")));
        assert_eq!(duckdb_read, Some((AssetKind::S3Object, "exports/x")));
        assert_eq!(sdk_write, duckdb_read);

        // The `// on` trigger annotation goes through the same function.
        assert_eq!(
            parse_asset_syntax("s3:///exports/x", true),
            parse_asset_syntax("s3://exports/x", true)
        );

        // Explicit-storage form is unaffected (no leading slash to strip).
        assert_eq!(
            parse_asset_syntax("s3://mybucket/exports/x", false),
            Some((AssetKind::S3Object, "mybucket/exports/x"))
        );

        // Hive-partition keys and nested paths under default storage are
        // preserved verbatim (only leading slashes are stripped).
        assert_eq!(
            parse_asset_syntax("s3:///t/year=2024/month=01/f.parquet", false),
            Some((AssetKind::S3Object, "t/year=2024/month=01/f.parquet"))
        );

        // Every leading slash is stripped so a canonical S3 path never starts
        // with `/`. `S3Object(s3="/x")` resolves to the quad-slash URI
        // `s3:////x`; the identity must be the bare `x` (not `/x`) so the ref
        // that `trigger_spec_to_row` rebuilds round-trips back to it.
        assert_eq!(
            parse_asset_syntax("s3:////x", false),
            Some((AssetKind::S3Object, "x"))
        );
        assert_eq!(
            parse_asset_syntax("s3://///deep///", false),
            Some((AssetKind::S3Object, "deep///"))
        );

        // Non-S3 kinds keep their leading slash (their paths are workspace-
        // relative and the slash is significant).
        assert_eq!(
            parse_asset_syntax("res://f/foo", false),
            Some((AssetKind::Resource, "f/foo"))
        );
        assert_eq!(
            parse_asset_syntax("ducklake://analytics/orders", false),
            Some((AssetKind::Ducklake, "analytics/orders"))
        );
    }

    #[test]
    fn s3_explicit_storage_aliases_default_storage_nested_key() {
        // Accepted tradeoff of one canonical key: the explicit-storage form
        // `s3://storage/key` and the default-storage nested-key form
        // `s3:///storage/key` collapse to the same node `storage/key`, even
        // though they name different objects. This is a best-effort lineage
        // graph that does not split the first segment as a storage name; the
        // collision only happens when a storage config is named to match a
        // default-storage prefix. Pinned so the aliasing is intentional, not a
        // latent surprise.
        assert_eq!(
            parse_asset_syntax("s3://mybucket/x", false),
            parse_asset_syntax("s3:///mybucket/x", false)
        );
        assert_eq!(
            parse_asset_syntax("s3://mybucket/x", false),
            Some((AssetKind::S3Object, "mybucket/x"))
        );
    }

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
    fn measures_and_dimensions_parse() {
        let out = parse_pipeline_annotations(
            "-- materialize ducklake://f/finance/orders\n\
             -- measure revenue = sum(amount) where not is_refund\n\
             -- measure orders  = count(*)\n\
             -- dimension region = region\n\
             -- dimension month  = date_trunc('month', ordered_at)\n\
             SELECT 1;",
        );
        assert_eq!(
            out.measures,
            vec![
                Measure {
                    name: "revenue".to_string(),
                    expr: "sum(amount)".to_string(),
                    filter: Some("not is_refund".to_string()),
                },
                Measure { name: "orders".to_string(), expr: "count(*)".to_string(), filter: None },
            ]
        );
        assert_eq!(
            out.dimensions,
            vec![
                Dimension { name: "region".to_string(), expr: "region".to_string() },
                Dimension {
                    name: "month".to_string(),
                    expr: "date_trunc('month', ordered_at)".to_string(),
                },
            ]
        );
    }

    #[test]
    fn a_where_inside_a_string_or_parens_is_not_a_filter_delimiter() {
        // The `where` is inside the aggregate's string argument, so the whole
        // expression is the measure and there is no filter.
        assert_eq!(
            split_measure_filter("count_if(note = 'some where value')"),
            ("count_if(note = 'some where value')", None)
        );
        // A real top-level filter still splits.
        assert_eq!(
            split_measure_filter("sum(amount) where not is_refund"),
            ("sum(amount)", Some("not is_refund".to_string()))
        );
        // A `where` inside parens is not the delimiter either.
        assert_eq!(
            split_measure_filter("sum(case when x then 1 end)"),
            ("sum(case when x then 1 end)", None)
        );
        // A non-ASCII body must not panic on a byte offset that is not a char
        // boundary.
        assert_eq!(split_measure_filter("sum(amount) + π"), ("sum(amount) + π", None));
        assert_eq!(
            split_measure_filter("sum(π) where region = 'π'"),
            ("sum(π)", Some("region = 'π'".to_string()))
        );
    }

    #[test]
    fn measures_dimensions_fail_safe_and_dedup() {
        let out = parse_pipeline_annotations(
            // Malformed (no `=`, empty body, bare trailing `where`) drop; a
            // duplicate name keeps the first; the body comment after real code
            // is never scanned.
            "-- measure broken\n\
             -- measure empty =   \n\
             -- measure dangling = sum(amount) where\n\
             -- measure revenue = sum(amount)\n\
             -- measure revenue = sum(other)\n\
             -- dimension region = region\n\
             SELECT 1;\n\
             -- measure sneaky = count(*)\n\
             -- dimension sneaky = x",
        );
        assert_eq!(
            out.measures,
            vec![Measure {
                name: "revenue".to_string(),
                expr: "sum(amount)".to_string(),
                filter: None,
            }]
        );
        assert_eq!(
            out.dimensions,
            vec![Dimension { name: "region".to_string(), expr: "region".to_string() }]
        );
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
    fn materialize_scd2_write_targets_include_current_view() {
        // A managed scd2 materialize produces the base table AND the
        // `<dim>_current` companion view, so the producer must be recorded as
        // writing both (reads of the view otherwise resolve to an orphan asset).
        let out = parse_pipeline_annotations(
            "// materialize ducklake://main/dim_customers key=id history",
        );
        let m = out.materialize.expect("materialize");
        assert_eq!(
            m.write_targets(),
            vec![
                (AssetKind::Ducklake, "main/dim_customers".to_string()),
                (
                    AssetKind::Ducklake,
                    "main/dim_customers_current".to_string()
                ),
            ]
        );
        assert_eq!(
            m.scd2_current_target(),
            Some((
                AssetKind::Ducklake,
                "main/dim_customers_current".to_string()
            ))
        );
    }

    #[test]
    fn materialize_non_scd2_write_targets_are_base_only() {
        // A plain merge (no `history`) creates no companion view — only the base.
        let out = parse_pipeline_annotations("// materialize ducklake://main/dim_customers key=id");
        let m = out.materialize.expect("materialize");
        assert_eq!(
            m.write_targets(),
            vec![(AssetKind::Ducklake, "main/dim_customers".to_string())]
        );
        assert_eq!(m.scd2_current_target(), None);
    }

    #[test]
    fn materialize_manual_scd2_has_no_companion_view() {
        // `manual` mode owns its own DDL and never creates the `_current` view,
        // so registering it would be a false producer edge.
        let out = parse_pipeline_annotations(
            "// materialize manual ducklake://main/dim_customers key=id history",
        );
        let m = out.materialize.expect("materialize");
        assert!(m.manual && m.scd2);
        assert_eq!(m.scd2_current_target(), None);
        assert_eq!(
            m.write_targets(),
            vec![(AssetKind::Ducklake, "main/dim_customers".to_string())]
        );
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
    fn materialize_validate_scd2_requires_key() {
        // scd2 without `key=` is rejected at deploy (was a run-time error).
        let m = parse_pipeline_annotations("// materialize ducklake://a/dim history")
            .materialize
            .expect("materialize");
        assert!(m.scd2 && m.unique_key.is_none());
        let err = m.validate(false).expect_err("scd2 without key must fail");
        assert!(err.contains("requires a natural key"));
        // with a key it validates
        let m = parse_pipeline_annotations("// materialize ducklake://a/dim key=id history")
            .materialize
            .expect("materialize");
        assert!(m.validate(false).is_ok());
    }

    #[test]
    fn materialize_validate_scd2_rejects_partitioned() {
        // scd2 + `// partitioned` has no v1 form — rejected at deploy.
        let m = parse_pipeline_annotations("// materialize ducklake://a/dim key=id history")
            .materialize
            .expect("materialize");
        let err = m.validate(true).expect_err("scd2 + partitioned must fail");
        assert!(err.contains("`// partitioned` is not supported with scd2"));
        // unpartitioned scd2 is fine
        assert!(m.validate(false).is_ok());
    }

    #[test]
    fn materialize_validate_non_scd2_and_manual_are_inert() {
        // Non-scd2 strategies are unconstrained by these checks, partitioned or not.
        let m = parse_pipeline_annotations("// materialize ducklake://a/orders key=id")
            .materialize
            .expect("materialize");
        assert!(m.validate(true).is_ok());
        assert!(m.validate(false).is_ok());
        // `manual` owns its DDL and ignores the strategy — never rejected here,
        // even with a partitioned scd2-looking combo.
        let m = parse_pipeline_annotations("// materialize manual ducklake://a/dim history")
            .materialize
            .expect("materialize");
        assert!(m.manual && m.scd2);
        assert!(m.validate(true).is_ok());
    }

    #[test]
    fn materialize_on_schema_change_opt() {
        let out = parse_pipeline_annotations(
            "// materialize ducklake://a/orders on_schema_change=ignore",
        );
        let m = out.materialize.expect("materialize");
        assert_eq!(m.on_schema_change, OnSchemaChange::Ignore);
        // default is warn
        let out = parse_pipeline_annotations("// materialize ducklake://a/orders");
        assert_eq!(
            out.materialize.expect("materialize").on_schema_change,
            OnSchemaChange::Warn
        );
        // fail + sync parse to their own variants
        let out =
            parse_pipeline_annotations("// materialize ducklake://a/orders on_schema_change=fail");
        assert_eq!(
            out.materialize.expect("materialize").on_schema_change,
            OnSchemaChange::Fail
        );
        let out =
            parse_pipeline_annotations("// materialize ducklake://a/orders on_schema_change=sync");
        assert_eq!(
            out.materialize.expect("materialize").on_schema_change,
            OnSchemaChange::Sync
        );
        // unknown/junk value keeps the warn default (fail-safe, like `deletes=`)
        let out =
            parse_pipeline_annotations("// materialize ducklake://a/orders on_schema_change=bogus");
        assert_eq!(
            out.materialize.expect("materialize").on_schema_change,
            OnSchemaChange::Warn
        );
        // composes with other opts
        let out = parse_pipeline_annotations(
            "// materialize ducklake://a/dim key=id history on_schema_change=ignore",
        );
        let m = out.materialize.expect("materialize");
        assert!(m.scd2);
        assert_eq!(m.on_schema_change, OnSchemaChange::Ignore);
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

    #[test]
    fn count_malformed_data_tests_counts_only_broken_header_lines() {
        let n = count_malformed_data_tests(concat!(
            "-- data_test not_null id\n",                 // valid
            "-- data_test unique id\n",                   // valid
            "-- data_test accepted_values status paid\n", // malformed: missing `=`
            "-- data_test relationships cust ducklake\n", // malformed: no `->`
            "-- data_test\n",                             // malformed: bare keyword
            "-- data_test f/tests/custom\n",              // valid: custom script path
            "SELECT 1 -- data_test not_a_test\n",         // body line: not counted
        ));
        assert_eq!(n, 3);
        // A clean header has zero.
        assert_eq!(
            count_malformed_data_tests("-- data_test unique id\nSELECT 1"),
            0
        );
    }
}
