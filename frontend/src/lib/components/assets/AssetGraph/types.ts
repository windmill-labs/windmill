import type { AssetKind } from '$lib/gen'

export type GraphUsageKind = 'script' | 'flow'

export interface AssetGraphAssetNode {
	kind: AssetKind
	path: string
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
	// Synthesized by the page from a local draft; the script doesn't exist
	// in the DB yet. Drives a dashed/lower-opacity rendering to mirror how
	// unsaved triggers are styled — visually distinct from persisted nodes.
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
export type NativeTriggerKind =
	| 'webhook'
	| 'email'
	| 'kafka'
	| 'mqtt'
	| 'nats'
	| 'postgres'
	| 'sqs'
	| 'gcp'

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
			trigger_kind: 'schedule'
			cron: string
			runnable_kind: GraphUsageKind
			runnable_path: string
			unsaved?: boolean
	  }
	| {
			trigger_kind: NativeTriggerKind
			// path of the matching trigger row (kafka_trigger.path, etc.).
			// Undefined when `missing` is true — the script has the
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

export interface AssetGraphResponse {
	assets: AssetGraphAssetNode[]
	runnables: AssetGraphRunnableNode[]
	edges: AssetGraphEdge[]
	triggers: AssetGraphTrigger[]
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
