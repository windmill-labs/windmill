import type { AssetGraphEdge, AssetGraphResponse } from './types'

/**
 * The bare folder name a pipeline is scoped to, from either form a caller may
 * hold (`analytics` or the owner path `f/analytics`). Every pipeline surface —
 * the `/pipeline/<folder>` route, the editor's `folder` prop, the AI folder
 * context — keys on the NAME and builds `f/<folder>/<node>` paths from it, so a
 * value that still carries the prefix double-prefixes every path built from it.
 * Folder names cannot contain `/`, so a leading `f/` is unambiguously the prefix.
 */
export function normalizePipelineFolder(folder: string): string {
	return folder
		.trim()
		.replace(/^\/+|\/+$/g, '')
		.replace(/^f\//, '')
}

// Shared graph predicates used by both the execution-DAG traversal
// (graphTraversal.ts) and the post-deploy drift check (deployGraphDiff.ts).
// A "write edge" is what makes a script a *producer* of an asset; the default
// `?? 'r'` treats a null access_type as a read so a missing direction never
// counts as a write.

/** `kind:path` identity for an asset, the dedup key used across the graph. */
export function assetKey(asset: { asset_kind: string; asset_path: string }): string {
	return `${asset.asset_kind}:${asset.asset_path}`
}

/** True for a script write lineage edge (access 'w' or 'rw'). */
export function isWriteEdge(edge: AssetGraphEdge): boolean {
	if (edge.runnable_kind !== 'script') return false
	const access = edge.access_type ?? 'r'
	return access === 'w' || access === 'rw'
}

/**
 * `asset key` → set of subscriber script paths (`// on <asset>` declarations).
 * Flows are excluded, mirroring the backend dispatch policy.
 */
export function buildAssetSubscribers(g: AssetGraphResponse): Map<string, Set<string>> {
	const subscribersByAsset = new Map<string, Set<string>>()
	for (const t of g.triggers ?? []) {
		if (t.trigger_kind !== 'asset' || t.runnable_kind !== 'script') continue
		const key = assetKey(t)
		const set = subscribersByAsset.get(key) ?? new Set<string>()
		set.add(t.runnable_path)
		subscribersByAsset.set(key, set)
	}
	return subscribersByAsset
}
