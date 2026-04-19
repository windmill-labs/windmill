import type { AssetKind } from '$lib/gen'

export type GraphUsageKind = 'script' | 'flow'

export interface AssetGraphAssetNode {
	kind: AssetKind
	path: string
}

export interface AssetGraphRunnableNode {
	path: string
	usage_kind: GraphUsageKind
}

export interface AssetGraphEdge {
	runnable_path: string
	runnable_kind: GraphUsageKind
	asset_kind: AssetKind
	asset_path: string
	access_type: 'r' | 'w' | 'rw' | null
}

export interface AssetGraphResponse {
	assets: AssetGraphAssetNode[]
	runnables: AssetGraphRunnableNode[]
	edges: AssetGraphEdge[]
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
