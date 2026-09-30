import type { NativeTriggerKind } from './types'

/** A node or edge of the canvas model, reduced to what the assets-only view reads. */
export type ViewNode = { id: string; type: string; data: any }
export type ViewEdge = {
	id: string
	source: string
	target: string
	kind: string
	unsaved?: boolean
}

/** The assets-only node standing for the scripts that build no asset. */
export const NO_ASSET_NODE_ID = 'no-asset'

/** The script and trigger an assets-only asset's delete removes. */
export type AssetUpstreamDelete = {
	script: { path: string; unsaved: boolean }
	trigger?: { kind: NativeTriggerKind; path: string; draft: boolean }
}

/** What an asset shows of the script and trigger that produce it, in the assets-only view. */
export type AssetUpstream =
	/** Several producing scripts, or one with several triggers. */
	| { multiple: true; runnableIds: string[] }
	| {
			multiple: false
			/** Canvas id of the producing runnable (`script:<path>`). */
			runnableId: string
			/** The producer's trigger; undefined when nothing triggers it (run by hand). */
			trigger?: { nodeId?: string; kind: string; data?: any }
	  }

/**
 * The asset → asset view of a pipeline graph: runnables and triggers disappear,
 * and every read (or asset trigger) of a script is joined to each of its writes,
 * so an asset points at the assets computed from it. Each produced asset keeps a
 * reference to its single producer and that producer's single trigger, or is
 * marked `multiple` when either is ambiguous.
 */
export function assetsOnlyView<N extends ViewNode, E extends ViewEdge>(
	nodes: N[],
	edges: E[]
): {
	nodeIds: Set<string>
	edges: Array<ViewEdge>
	upstream: Map<string, AssetUpstream>
	/** Some scripts build no asset; their upstream is under `NO_ASSET_NODE_ID`. */
	hasNoAssetNode: boolean
} {
	const nodeById = new Map(nodes.map((n) => [n.id, n]))
	const inputsOf = new Map<string, E[]>()
	const triggersOf = new Map<string, E[]>()
	const producersOf = new Map<string, E[]>()
	const push = (m: Map<string, E[]>, k: string, e: E) => m.set(k, [...(m.get(k) ?? []), e])
	for (const e of edges) {
		if (e.kind === 'lineage-write') push(producersOf, e.target, e)
		else if (e.kind === 'lineage-read' || e.kind === 'trigger-asset') {
			push(inputsOf, e.target, e)
			if (e.kind === 'trigger-asset') push(triggersOf, e.target, e)
		} else if (e.kind === 'trigger-native') push(triggersOf, e.target, e)
	}

	const out = new Map<string, ViewEdge>()
	for (const [assetId, writes] of producersOf) {
		for (const w of writes) {
			for (const r of inputsOf.get(w.source) ?? []) {
				// A read-modify-write of the same table is not a dependency on itself.
				if (r.source === assetId) continue
				const id = `flow:${r.source}->${assetId}`
				const prev = out.get(id)
				const unsaved = !!(w.unsaved || r.unsaved)
				if (prev) prev.unsaved = prev.unsaved && unsaved
				else out.set(id, { id, source: r.source, target: assetId, kind: 'asset-flow', unsaved })
			}
		}
	}
	for (const e of edges) if (e.kind === 'dbt-ref') out.set(e.id, e)

	function describe(runnableIds: string[]): AssetUpstream {
		if (runnableIds.length !== 1) return { multiple: true, runnableIds }
		const runnableId = runnableIds[0]
		// Every asset trigger of a script is one cause, "an input changed".
		const triggers = [
			...new Map(
				(triggersOf.get(runnableId) ?? []).map((t) => [
					t.kind === 'trigger-asset' ? 'asset' : t.source,
					t
				])
			).values()
		]
		if (triggers.length > 1) return { multiple: true, runnableIds }
		const t = triggers[0]
		const tNode = t && nodeById.get(t.source)
		return {
			multiple: false,
			runnableId,
			trigger: !t
				? undefined
				: t.kind === 'trigger-asset'
					? { kind: 'asset' }
					: { nodeId: t.source, kind: tNode?.data?.kind, data: tNode?.data }
		}
	}

	const upstream = new Map<string, AssetUpstream>()
	for (const [assetId, writes] of producersOf) {
		upstream.set(assetId, describe([...new Set(writes.map((w) => w.source))]))
	}

	// Scripts that build no asset have nowhere to fold into, so they share one
	// "No asset" node, fed by what they read. dbt projects (whose writes are not
	// drawn) and macro libraries (which build nothing by design) stay out of it.
	const writers = new Set([...producersOf.values()].flat().map((w) => w.source))
	const orphans = nodes
		.filter(
			(n) => n.type === 'runnable' && !writers.has(n.id) && !n.data?.dbt && !n.data?.macros?.length
		)
		.map((n) => n.id)
	if (orphans.length > 0) {
		upstream.set(NO_ASSET_NODE_ID, describe(orphans))
		for (const id of orphans) {
			for (const r of inputsOf.get(id) ?? []) {
				const eid = `flow:${r.source}->${NO_ASSET_NODE_ID}`
				const prev = out.get(eid)
				if (prev) prev.unsaved = prev.unsaved && !!r.unsaved
				else
					out.set(eid, {
						id: eid,
						source: r.source,
						target: NO_ASSET_NODE_ID,
						kind: 'asset-flow',
						unsaved: !!r.unsaved
					})
			}
		}
	}

	const nodeIds = new Set(nodes.filter((n) => n.type === 'asset').map((n) => n.id))
	return { nodeIds, edges: [...out.values()], upstream, hasNoAssetNode: orphans.length > 0 }
}

/** What deleting an asset's upstream removes: the script unless another asset
 * is also built by it, and with it the trigger unless that fires other scripts.
 * Undefined when nothing can go. */
export function upstreamDeletion(
	u: AssetUpstream & { multiple: false },
	r: { runnable_kind: string; path: string; unsaved?: boolean },
	all: Map<string, AssetUpstream>
): AssetUpstreamDelete | undefined {
	if (r.runnable_kind !== 'script') return undefined
	const producesOthers =
		[...all.values()].filter((o) =>
			o.multiple ? o.runnableIds.includes(u.runnableId) : o.runnableId === u.runnableId
		).length > 1
	if (producesOthers) return undefined
	const t = u.trigger
	const kind = t?.kind as NativeTriggerKind | undefined
	const targets: string[] = t?.data?.runnable_paths ?? []
	const ownTrigger =
		kind &&
		kind !== 'webhook' &&
		kind !== 'data_upload' &&
		t?.data?.ref &&
		!t.data.missing &&
		targets.every((p) => p === r.path)
	return {
		script: { path: r.path, unsaved: !!r.unsaved },
		trigger: ownTrigger ? { kind, path: t.data.ref, draft: !!t.data.draft } : undefined
	}
}
