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
	// `append`/`merge`/partitioned writes INSERT into a fixed-schema table.
	materialize_strategy?: 'replace' | 'append' | 'merge'
	// Macros this script provides to the workspace registry (deployed
	// `// macros` library). Non-empty marks the node as a macro library;
	// drives the "defines N macros" badge and the details-pane signature
	// list. `params` is the verbatim parameter list.
	macros?: { name: string; params: string; is_table: boolean }[]
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

export interface AssetGraphResponse {
	assets: AssetGraphAssetNode[]
	runnables: AssetGraphRunnableNode[]
	edges: AssetGraphEdge[]
	triggers: AssetGraphTrigger[]
	macro_edges?: AssetGraphMacroEdge[]
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
