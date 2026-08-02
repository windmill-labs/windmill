import type { AssetKind } from '$lib/gen'
import type { ColumnLineage, DataTest } from './parsePipelineAnnotations'

export type GraphUsageKind = 'script' | 'flow'

export interface AssetGraphAssetNode {
	kind: AssetKind
	path: string
	// Fork workspaces only: 'fork' when this ducklake asset was materialized in
	// the fork itself, 'deferred' when reads fall back to the parent workspace's
	// current table via a defer view. Absent outside forks / for other kinds /
	// when never materialized anywhere. Lockstep with Rust `GraphAssetNode`.
	fork_materialization?: 'fork' | 'deferred'
	// Base dimension path this node is the SCD2 `<dim>_current` companion view of
	// (its producer declares `// materialize … history` on `<dim>`). Set only on
	// the `_current` node; lets the canvas mark it as a derived "current view"
	// rather than an unrelated table. Lockstep with Rust `GraphAssetNode`.
	derived_from?: string
	// Set on a `dbt://` asset a dbt project produces (or, for a source,
	// consumes). A dbt script is ONE runnable node with many model assets, so
	// per-model metadata hangs off the asset, not off the script. Lockstep with
	// Rust `GraphAssetNode.dbt`.
	dbt?: DbtAssetProvenance
}

// Describes the RELATION, not a producer: several dbt scripts (different
// selections of one project) can materialize the same model, and the producer
// edges already name them.
export interface DbtAssetProvenance {
	unique_id: string
	resource_type: 'model' | 'snapshot' | 'seed' | 'source' | (string & {})
	// dbt's own word (`table`, `view`, `incremental`, `snapshot`). Kept because
	// `view` and `ephemeral` have no Windmill write-strategy analogue.
	materialized?: string
	materialize_strategy?: 'replace' | 'append' | 'merge' | 'scd2'
	tags?: string[]
	description?: string
	data_tests?: DbtDataTest[]
	/** Declared column metadata (name -> description). NOT column lineage:
	 *  `manifest.json` carries none (docs/dbt-runtime.md, decision 14). */
	columns?: Record<string, string>
	/** A source's declared freshness policy. */
	freshness?: unknown
	/** The model's SQL as written — the transform behind the node. Read-only:
	 *  this is the copy captured when the graph on screen was parsed, whether
	 *  that was a deploy or a refresh from the editor's buffer, and the file
	 *  itself lives in the producing script's `__dbt/` bundle. Absent for tests
	 *  and for nodes with no body. */
	raw_code?: string
	/** Its path inside the dbt project, e.g. `models/staging/stg_orders.sql`. */
	original_file_path?: string
}

export interface DbtDataTest {
	// The four dbt generic tests map one-for-one onto the `// data_test` kinds;
	// a package test keeps its namespaced name (`dbt_utils.accepted_range`).
	kind: string
	column?: string
	args?: unknown
	// Lowercased. dbt's own severity decides whether a failure fails the run,
	// so the badge shows it rather than assuming every test is blocking.
	severity?: string
}

export interface AssetGraphRunnableNode {
	path: string
	usage_kind: GraphUsageKind
	// Script has `// pipeline` annotation. Drives the pipeline-member
	// visual state; unrelated to what the script actually writes (that's
	// parsed separately into lineage edges).
	in_pipeline?: boolean
	// Partition kind (`daily` | `hourly` | `weekly` | `monthly` | `dynamic`)
	// surfaced from `// partitioned <kind>` for the badge — full PartitionSpec
	// would carry tz/format/start; here we just need the label.
	partition_kind?: 'daily' | 'hourly' | 'weekly' | 'monthly' | 'dynamic'
	// Raw `// freshness <duration>` value, e.g. "1h", "30m". Surfaced for
	// the badge; the runtime parses it as needed.
	freshness?: string
	// Completion time (ISO) of the newest successful run of this pipeline
	// member visible to the caller. The freshness chip compares it against
	// the `// freshness` window to render fresh/stale. Absent = no
	// successful run found (or none visible under job RLS).
	last_success_at?: string
	// `// tag <name>` worker-tag override. Surfaced for the badge so users
	// can see which worker pool will pick this script up at a glance.
	tag?: string
	// `// retry <count> [<delay>]` cascade retry policy. `delay` is the raw
	// duration string (`"5s"`, `"30s"`); absent = back-to-back. Surfaced as
	// a badge so retry-enabled scripts are visible without opening the pane.
	retry?: { count: number; delay?: string }
	// `// data_test <kind> …` data-quality checks run against the materialized
	// asset. Surfaced as a count badge (with a per-test breakdown in the title)
	// so test coverage is visible on the node without opening the pane.
	data_tests?: DataTest[]
	// `// column <out> <- <src>.<col>` declared column-level lineage for this
	// script's materialized output. Surfaced as a count badge on the write-edge
	// and as a column-to-column diagram in the asset details pane.
	column_lineage?: ColumnLineage[]
	// `// materialize <asset>` target — the asset `column_lineage` describes.
	// Lets the column graph anchor lineage to the exact output instead of
	// guessing a ducklake write-edge (a multi-output script writes several).
	materialize_target?: { kind: AssetKind; path: string }
	// Managed `// materialize` write strategy. Absent for non-materializing or
	// `manual` scripts. Used (with `partition_kind`) to decide whether a
	// produced asset's schema can evolve: only whole-table `replace` can, since
	// `append`/`merge`/`scd2`/partitioned writes INSERT into a fixed-schema
	// table. `scd2` also identifies the producer of a `<dim>_current` companion
	// view for the schema-contract `_current` → base-table fallback.
	materialize_strategy?: 'replace' | 'append' | 'merge' | 'scd2'
	// `on_schema_change=ignore` on the managed materialize — the producer's
	// opt-out from downstream schema-contract warnings. Only present when set
	// to `ignore` (default `warn` is absent). Threaded into the editor's
	// contract mirror so it suppresses the same warnings the server check does.
	materialize_on_schema_change?: string
	// Macros this script provides to the workspace registry (deployed
	// `// macros` library). Non-empty marks the node as a macro library;
	// drives the "defines N macros" badge and the details-pane signature
	// list. `params` is the verbatim parameter list.
	macros?: { name: string; params: string; is_table: boolean }[]
	// Set on a `ScriptLang::Dbt` script: it owns a whole dbt project, so the node
	// says how many models it materializes rather than reading as a single-output
	// script. Lockstep with Rust `GraphRunnableNode.dbt`.
	dbt?: { model_count: number }
	// Synthesized by the page from a local draft; the script doesn't exist
	// in the DB yet. Drives a dashed/lower-opacity rendering to mirror how
	// unsaved triggers are styled — visually distinct from persisted nodes.
	// AI-built nodes are plain drafts too (no separate pending/approval state).
	unsaved?: boolean
}

// Lineage edge from parsed r/w usages — informational only, not the
// execution DAG. `unsaved: true` for edges synthesized by a draft overlay
// (e.g. the random output asset attached at draft creation).
export interface AssetGraphEdge {
	runnable_path: string
	runnable_kind: GraphUsageKind
	asset_kind: AssetKind
	asset_path: string
	access_type: 'r' | 'w' | 'rw' | null
	unsaved?: boolean
}

// Declared `// on <trigger>` — the actual execution DAG edges.
// `unsaved: true` marks overlays computed live from editor buffer that
// haven't been persisted to script_trigger yet.
//
// `schedule` is in the family — the cron lives on the schedule row the user
// creates separately; the annotation is just the binding declaration, same
// as kafka/mqtt/etc.
//
// `data_upload` is the UI-first odd one out: no event source and no trigger
// row anywhere. The script declares an `S3Object` input and the user uploads
// a file via the auto-generated S3 picker, which runs the pipeline. Like
// webhook, it's never rendered as a "missing" placeholder.
export type NativeTriggerKind =
	| 'schedule'
	| 'webhook'
	| 'email'
	| 'kafka'
	| 'mqtt'
	| 'amqp'
	| 'nats'
	| 'postgres'
	| 'sqs'
	| 'gcp'
	| 'data_upload'

export type AssetGraphTrigger =
	| {
			trigger_kind: 'asset'
			asset_kind: AssetKind
			asset_path: string
			runnable_kind: GraphUsageKind
			runnable_path: string
			unsaved?: boolean
	  }
	| {
			trigger_kind: NativeTriggerKind
			// path of the matching trigger row (kafka_trigger.path, schedule.path,
			// etc.). Undefined when `missing` is true — the script has the
			// annotation marker but no trigger row points at it.
			path?: string
			runnable_kind: GraphUsageKind
			runnable_path: string
			unsaved?: boolean
			// Annotation declared but no matching trigger row was found —
			// the canvas renders a red placeholder with a "Create trigger"
			// affordance instead of a fully-wired source.
			missing?: boolean
	  }

// Macro-library → consumer edge: the consumer calls `macro_names` of
// `lib_path`'s macros (deploy-recorded detection), or pulls in the whole
// library via `// use` (`via_use`, macro_names then lists the full library).
// `unsaved: true` marks a draft's `// use` overlay.
export interface AssetGraphMacroEdge {
	lib_path: string
	consumer_path: string
	macro_names: string[]
	via_use: boolean
	unsaved?: boolean
}

// Ordering-only "must-run-after" edge: `runnable_path`'s `// data_test`
// (a `relationships` ref, or a custom test reading a pipeline asset) needs
// `asset` materialized before the tested script runs — but the tested script
// doesn't consume the asset's rows, so this is NOT a lineage edge. Resolved
// server-side to the referenced asset's in-pipeline producer; fed into the
// cascade topo-sort (buildLineageDag) so a cold cascade orders the referenced
// dimension first, and rendered dashed on the canvas (like macro edges).
export interface AssetGraphTestEdge {
	producer_kind: GraphUsageKind
	producer_path: string
	runnable_kind: GraphUsageKind
	runnable_path: string
	asset_kind: AssetKind
	asset_path: string
}

export interface AssetGraphResponse {
	assets: AssetGraphAssetNode[]
	runnables: AssetGraphRunnableNode[]
	edges: AssetGraphEdge[]
	triggers: AssetGraphTrigger[]
	macro_edges?: AssetGraphMacroEdge[]
	test_edges?: AssetGraphTestEdge[]
	dbt_edges?: AssetGraphDbtEdge[]
	/** The job whose snapshot the dbt half was resolved from, when one was
	 *  asked for and found. A run page polls the graph until this is its own
	 *  job, which is how it knows a dynamic descriptor's ingest has landed. */
	dbt_snapshot_job?: string
	/** When the dbt half on screen was parsed, for a graph pinned to a job.
	 *  What the dbt editor labels its provenance with — a buffer refresh and the
	 *  deployed version's graph are drawn identically. */
	dbt_graph_ingested_at?: string
}

// `ref()` lineage BETWEEN two dbt models, in the terms the canvas draws
// (relations, not dbt node ids). Without these every model hangs off the one
// dbt runnable, which reads as a flat fan-out rather than the project's shape.
export interface AssetGraphDbtEdge {
	from_asset_path: string
	to_asset_path: string
}

export type AssetGraphNodeData =
	| {
			kind: 'asset'
			asset_kind: AssetKind
			path: string
	  }
	| {
			kind: 'runnable'
			runnable_kind: GraphUsageKind
			path: string
	  }

export type AssetGraphSelection = AssetGraphNodeData

/** Page-level mode for /pipeline/[folder]: read-only deployed view
 * (default) or full editor. View can additionally overlay unsaved drafts
 * ("show drafts" chip) — a view variant, not a separate mode. */
export type PipelineMode = 'view' | 'edit'

/** What a run in view is doing to one relation, and what it produced. The row
 *  count is the cheapest answer to "did this model actually output anything" —
 *  the worker already records it per relation, so nothing extra is fetched. */
export type AssetRunState = {
	status: 'running' | 'materialized' | 'failed'
	rowCount?: number | null
}
