import { dagStratify, sugiyama, coordCenter, decrossTwoLayer, decrossOpt } from 'd3-dag'
import type { AssetGraphNodeData } from './types'
import { NODE } from '$lib/components/graph/util'

// Match the flow editor's per-node sizing and inter-node gaps so the asset
// graph's spacing feels familiar. NODE.height is the flow-step pill height (34
// px) which is too tight for a 2-line asset card, so we add a bit of vertical
// breathing room.
const NODE_WIDTH = NODE.width
const NODE_HEIGHT = NODE.height + 30
const LAYER_GAP = NODE.gap.vertical
const SIBLING_GAP = NODE.gap.horizontal

interface GraphInput {
	nodes: Array<{ id: string; data: AssetGraphNodeData }>
	edges: Array<{ source: string; target: string }>
}

interface Positioned {
	x: number
	y: number
}

// Sugiyama layered layout (top-down, same orientation as the flow editor —
// see compoundLayout.ts): producers above → assets in the middle → consumers
// below. Falls back to a stable grid if d3-dag throws (e.g., cyclic inputs).
export function layoutAssetGraph(graph: GraphInput): Map<string, Positioned> {
	const byId = new Map<string, Positioned>()
	if (graph.nodes.length === 0) return byId

	const parentsByChild = new Map<string, string[]>()
	for (const n of graph.nodes) parentsByChild.set(n.id, [])
	for (const e of graph.edges) {
		const arr = parentsByChild.get(e.target)
		if (arr && arr.indexOf(e.source) === -1) arr.push(e.source)
	}

	try {
		const dagNodes = graph.nodes.map((n) => ({
			id: n.id,
			parentIds: parentsByChild.get(n.id) ?? []
		}))
		const dag = dagStratify().id(({ id }: { id: string }) => id)(dagNodes)
		const layout = sugiyama()
			.decross(graph.nodes.length > 30 ? decrossTwoLayer() : decrossOpt())
			.coord(coordCenter())
			.nodeSize(
				() => [NODE_WIDTH + SIBLING_GAP, NODE_HEIGHT + LAYER_GAP] as readonly [number, number]
			)
		layout(dag as any)
		for (const desc of dag.descendants()) {
			const id = (desc as any).data.id as string
			byId.set(id, {
				x: (desc as any).x ?? 0,
				y: ((desc as any).y ?? 0) - (NODE_HEIGHT + LAYER_GAP) / 2
			})
		}
		// Normalize so min x,y = 0
		let minX = Infinity
		let minY = Infinity
		for (const p of byId.values()) {
			if (p.x < minX) minX = p.x
			if (p.y < minY) minY = p.y
		}
		if (isFinite(minX) && isFinite(minY)) {
			for (const p of byId.values()) {
				p.x -= minX
				p.y -= minY
			}
		}
		return byId
	} catch {
		const cols = Math.max(1, Math.ceil(Math.sqrt(graph.nodes.length)))
		graph.nodes.forEach((n, i) => {
			byId.set(n.id, {
				x: (i % cols) * (NODE_WIDTH + SIBLING_GAP),
				y: Math.floor(i / cols) * (NODE_HEIGHT + LAYER_GAP)
			})
		})
		return byId
	}
}
