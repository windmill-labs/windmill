/** A node or edge of the canvas model, reduced to what the assets-only view reads. */
export type ViewNode = { id: string; type: string; data: any }
export type ViewEdge = {
	id: string
	source: string
	target: string
	kind: string
	unsaved?: boolean
}

/** What an asset shows of the script and trigger that produce it, in the assets-only view. */
export type AssetUpstream =
	| { multiple: true }
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

	const upstream = new Map<string, AssetUpstream>()
	for (const [assetId, writes] of producersOf) {
		const runnableIds = [...new Set(writes.map((w) => w.source))]
		if (runnableIds.length !== 1) {
			upstream.set(assetId, { multiple: true })
			continue
		}
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
		if (triggers.length > 1) {
			upstream.set(assetId, { multiple: true })
			continue
		}
		const t = triggers[0]
		const tNode = t && nodeById.get(t.source)
		upstream.set(assetId, {
			multiple: false,
			runnableId,
			trigger: !t
				? undefined
				: t.kind === 'trigger-asset'
					? { kind: 'asset' }
					: { nodeId: t.source, kind: tNode?.data?.kind, data: tNode?.data }
		})
	}

	const nodeIds = new Set(nodes.filter((n) => n.type === 'asset').map((n) => n.id))
	return { nodeIds, edges: [...out.values()], upstream }
}
