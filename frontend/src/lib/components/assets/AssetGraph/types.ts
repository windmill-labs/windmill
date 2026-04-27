import type { AssetKind } from '$lib/gen'

export type GraphUsageKind = 'script' | 'flow'

export interface AssetGraphAssetNode {
	kind: AssetKind
	path: string
}

export interface AssetGraphRunnableNode {
	path: string
	usage_kind: GraphUsageKind
	// Script has `// materialize` annotation. Drives the pipeline-member
	// visual state; unrelated to what the script actually writes (that's
	// parsed separately into lineage edges).
	is_materializer?: boolean
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
			path: string
			runnable_kind: GraphUsageKind
			runnable_path: string
			unsaved?: boolean
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
