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
// Horizontal gutter between two disjoint subgraphs. Wider than the
// within-subgraph sibling gap so visually-unrelated components (e.g. two
// pipelines that share no asset) read as separate boxes instead of bleeding
// into each other's columns — but only ~2×, so they don't drift far apart.
const COMPONENT_GAP = SIBLING_GAP * 2

interface GraphInput {
	nodes: Array<{ id: string; data: AssetGraphNodeData }>
	edges: Array<{ source: string; target: string }>
}

interface Positioned {
	x: number
	y: number
}

// Sugiyama a single connected component. Returns positions normalized so the
// component's left/top edge sits at 0, so the caller can pack components
// side-by-side by translating on x. Throws on cyclic input (caller falls back
// to a grid for the whole graph).
function layoutComponent(
	nodes: GraphInput['nodes'],
	edges: GraphInput['edges']
): Map<string, Positioned> {
	const out = new Map<string, Positioned>()

	const parentsByChild = new Map<string, string[]>()
	for (const n of nodes) parentsByChild.set(n.id, [])
	for (const e of edges) {
		const arr = parentsByChild.get(e.target)
		if (arr && arr.indexOf(e.source) === -1) arr.push(e.source)
	}

	const dagNodes = nodes.map((n) => ({ id: n.id, parentIds: parentsByChild.get(n.id) ?? [] }))
	const dag = dagStratify().id(({ id }: { id: string }) => id)(dagNodes)
	const layout = sugiyama()
		// decrossOpt is exponential — only safe on small components.
		.decross(nodes.length > 30 ? decrossTwoLayer() : decrossOpt())
		.coord(coordCenter())
		.nodeSize(
			() => [NODE_WIDTH + SIBLING_GAP, NODE_HEIGHT + LAYER_GAP] as readonly [number, number]
		)
	layout(dag as any)
	for (const desc of dag.descendants()) {
		const id = (desc as any).data.id as string
		out.set(id, {
			x: (desc as any).x ?? 0,
			y: ((desc as any).y ?? 0) - (NODE_HEIGHT + LAYER_GAP) / 2
		})
	}
	// Normalize so the component's min x,y = 0.
	let minX = Infinity
	let minY = Infinity
	for (const p of out.values()) {
		if (p.x < minX) minX = p.x
		if (p.y < minY) minY = p.y
	}
	if (isFinite(minX) && isFinite(minY)) {
		for (const p of out.values()) {
			p.x -= minX
			p.y -= minY
		}
	}
	return out
}

// Sugiyama layered layout (top-down, same orientation as the flow editor —
// see compoundLayout.ts): producers above → assets in the middle → consumers
// below.
//
// Each weakly-connected component (treating edges as undirected) is laid out
// independently, then the components are packed left-to-right with a wide
// gutter between them. Running Sugiyama over the whole graph at once would
// interleave disjoint subgraphs in shared layers, leaving nodes from unrelated
// triggers horizontally adjacent; per-component layout makes each subgraph
// occupy its own horizontal band whose width is its own max breadth, so they
// read as clearly separated boxes.
//
// `anchorId` is an optional UI affordance node (the pipeline `+` button) that
// is wired to every root via synthetic edges. It must NOT merge the otherwise-
// disjoint components, so it's excluded from component detection and instead
// re-placed centered one layer above the whole packed graph.
//
// Falls back to a stable grid if d3-dag throws (e.g., cyclic inputs).
export function layoutAssetGraph(graph: GraphInput, anchorId?: string): Map<string, Positioned> {
	const byId = new Map<string, Positioned>()
	if (graph.nodes.length === 0) return byId

	// Real graph excludes the anchor and any edge incident to it — those edges
	// only exist to pin the anchor to the top and would otherwise stitch every
	// component into one.
	const nodes = anchorId ? graph.nodes.filter((n) => n.id !== anchorId) : graph.nodes
	const edges = anchorId
		? graph.edges.filter((e) => e.source !== anchorId && e.target !== anchorId)
		: graph.edges

	// 1. Weakly-connected components via undirected flood fill.
	const adj = new Map<string, string[]>()
	for (const n of nodes) adj.set(n.id, [])
	for (const e of edges) {
		if (adj.has(e.source) && adj.has(e.target)) {
			adj.get(e.source)!.push(e.target)
			adj.get(e.target)!.push(e.source)
		}
	}
	const compOf = new Map<string, number>()
	let nComp = 0
	for (const n of nodes) {
		if (compOf.has(n.id)) continue
		const stack = [n.id]
		compOf.set(n.id, nComp)
		while (stack.length) {
			const cur = stack.pop()!
			for (const nb of adj.get(cur) ?? []) {
				if (!compOf.has(nb)) {
					compOf.set(nb, nComp)
					stack.push(nb)
				}
			}
		}
		nComp++
	}

	// 2. Bucket nodes/edges by component (preserving input order for stable,
	// deterministic packing — components appear left-to-right in first-seen
	// order).
	const compNodes: GraphInput['nodes'][] = Array.from({ length: nComp }, () => [])
	for (const n of nodes) compNodes[compOf.get(n.id)!].push(n)
	const compEdges: GraphInput['edges'][] = Array.from({ length: nComp }, () => [])
	for (const e of edges) {
		const c = compOf.get(e.source)
		if (c !== undefined && c === compOf.get(e.target)) compEdges[c].push(e)
	}

	try {
		// 3. Lay out each component and pack side-by-side. xOffset advances by
		// the laid-out width of each component (max node center + a node width,
		// since coordCenter positions are node centers) plus the gutter.
		let xOffset = 0
		for (let c = 0; c < nComp; c++) {
			const positions = layoutComponent(compNodes[c], compEdges[c])
			let maxX = 0
			for (const p of positions.values()) if (p.x > maxX) maxX = p.x
			for (const [id, p] of positions) {
				byId.set(id, { x: p.x + xOffset, y: p.y })
			}
			xOffset += maxX + NODE_WIDTH + COMPONENT_GAP
		}

		// 4. Re-place the anchor centered horizontally over the whole packed
		// graph, one layer above it; then renormalize so the anchor sits at the
		// top (y = 0), pushing the components down a layer to make room (mirrors
		// the previous "anchor is the parent of every root" behaviour).
		if (anchorId && graph.nodes.some((n) => n.id === anchorId)) {
			if (byId.size === 0) {
				byId.set(anchorId, { x: 0, y: 0 })
			} else {
				let minX = Infinity
				let maxX = -Infinity
				let minY = Infinity
				for (const p of byId.values()) {
					if (p.x < minX) minX = p.x
					if (p.x > maxX) maxX = p.x
					if (p.y < minY) minY = p.y
				}
				byId.set(anchorId, { x: (minX + maxX) / 2, y: minY - (NODE_HEIGHT + LAYER_GAP) })
				let nMinY = Infinity
				for (const p of byId.values()) if (p.y < nMinY) nMinY = p.y
				for (const p of byId.values()) p.y -= nMinY
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
