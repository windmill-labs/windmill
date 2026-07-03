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

const LAYER_H = NODE_HEIGHT + LAYER_GAP

interface GraphInput {
	nodes: Array<{ id: string; data: AssetGraphNodeData }>
	edges: Array<{ source: string; target: string }>
}

interface Positioned {
	x: number
	y: number
}

// Horizontal band reserved by a placed subtree, with its vertical (layer)
// extent. Two bands may share x only when their layer ranges don't intersect.
interface Band {
	left: number
	right: number
	top: number
	bottom: number
}

// Tidy-tree layout of a single connected component.
//
// Each node reserves a horizontal band at least as wide as the sum of its
// children's bands (`W(n) = max(NODE_WIDTH, Σ W(children) + gaps)`), children
// are packed side-by-side inside the parent's band, and the parent is centered
// over it. Bands are exclusive, so two nodes with different parents can never
// interleave horizontally — the failure mode of the previous Sugiyama layout.
//
// The band recursion stops at *join points*: a node with two or more parents
// belongs to no single parent's band (it would be double-counted and would
// force conflicting reservations). Joins instead root their own subtree,
// placed after the source trees, centered under the mean x of their parents
// and pushed right just enough to not overlap any band whose layer range
// intersects theirs.
//
// y comes from longest-path layering (same top-down orientation as before:
// producers above, assets in the middle, consumers below). Returns positions
// (band centers) normalized so the component's min x,y = 0. Cyclic input is
// handled by dropping feedback edges (see the Kahn step below).
function layoutComponent(
	nodes: GraphInput['nodes'],
	edges: GraphInput['edges']
): Map<string, Positioned> {
	const out = new Map<string, Positioned>()
	const ids = new Set(nodes.map((n) => n.id))

	const parents = new Map<string, string[]>()
	const children = new Map<string, string[]>()
	for (const n of nodes) {
		parents.set(n.id, [])
		children.set(n.id, [])
	}
	for (const e of edges) {
		if (!ids.has(e.source) || !ids.has(e.target)) continue
		// Dedup parallel edges (read+write pairs produce two edges between the
		// same nodes) so they don't skew child ordering or join detection.
		if (!children.get(e.source)!.includes(e.target)) children.get(e.source)!.push(e.target)
		if (!parents.get(e.target)!.includes(e.source)) parents.get(e.target)!.push(e.source)
	}

	// Kahn topological order. Cycles don't abort the layout: when the queue
	// drains with nodes left, the unplaced node with the fewest outstanding
	// parents (first in input order on ties) is forced into the order and its
	// not-yet-placed parent edges are dropped as feedback edges — layering and
	// tree-building then operate on the resulting DAG while the rendered graph
	// keeps every edge. (The caller already resolves write⇄read 2-cycles by
	// omitting the read direction; this handles any longer cycle.)
	const indeg = new Map<string, number>()
	for (const n of nodes) indeg.set(n.id, parents.get(n.id)!.length)
	const queue = nodes.filter((n) => indeg.get(n.id) === 0).map((n) => n.id)
	const placed = new Set<string>()
	const topo: string[] = []
	while (topo.length < nodes.length) {
		if (queue.length === 0) {
			let pick: string | undefined
			for (const n of nodes) {
				if (placed.has(n.id)) continue
				if (pick === undefined || indeg.get(n.id)! < indeg.get(pick)!) pick = n.id
			}
			parents.set(
				pick!,
				parents.get(pick!)!.filter((p) => placed.has(p))
			)
			queue.push(pick!)
		}
		const cur = queue.shift()!
		if (placed.has(cur)) continue
		placed.add(cur)
		topo.push(cur)
		for (const c of children.get(cur)!) {
			if (placed.has(c)) continue
			const d = indeg.get(c)! - 1
			indeg.set(c, d)
			if (d === 0) queue.push(c)
		}
	}

	// Longest-path layering: a node sits one layer below its lowest parent.
	const layer = new Map<string, number>()
	for (const id of topo) {
		const ps = parents.get(id)!
		layer.set(id, ps.length === 0 ? 0 : Math.max(...ps.map((p) => layer.get(p)!)) + 1)
	}

	// Spanning forest: single-parent nodes hang under their parent; joins
	// (≥ 2 parents) root their own tree. Child order follows edge input order
	// for a stable, deterministic left-to-right.
	const treeChildren = new Map<string, string[]>()
	for (const n of nodes) treeChildren.set(n.id, [])
	for (const n of nodes) {
		const ps = parents.get(n.id)!
		if (ps.length === 1) treeChildren.get(ps[0])!.push(n.id)
	}

	// Band widths, post-order (reverse topo visits children before parents).
	const W = new Map<string, number>()
	for (let i = topo.length - 1; i >= 0; i--) {
		const id = topo[i]
		const kids = treeChildren.get(id)!
		const kidsW =
			kids.reduce((acc, k) => acc + W.get(k)!, 0) + SIBLING_GAP * Math.max(0, kids.length - 1)
		W.set(id, Math.max(NODE_WIDTH, kidsW))
	}

	// Vertical (pixel) extent of a subtree, for band collision checks.
	function treeSpan(root: string): { top: number; bottom: number } {
		let lo = layer.get(root)!
		let hi = lo
		const stack = [root]
		while (stack.length) {
			const cur = stack.pop()!
			const l = layer.get(cur)!
			if (l < lo) lo = l
			if (l > hi) hi = l
			for (const k of treeChildren.get(cur)!) stack.push(k)
		}
		return { top: lo * LAYER_H, bottom: hi * LAYER_H + NODE_HEIGHT }
	}

	// Recursive placement: node centered over its band, children packed
	// side-by-side and centered within it.
	function placeTree(id: string, left: number) {
		const w = W.get(id)!
		out.set(id, { x: left + w / 2, y: layer.get(id)! * LAYER_H })
		const kids = treeChildren.get(id)!
		if (kids.length === 0) return
		const kidsW = kids.reduce((acc, k) => acc + W.get(k)!, 0) + SIBLING_GAP * (kids.length - 1)
		let cursor = left + (w - kidsW) / 2
		for (const k of kids) {
			placeTree(k, cursor)
			cursor += W.get(k)! + SIBLING_GAP
		}
	}

	const bands: Band[] = []
	function placeAndRecord(id: string, left: number) {
		placeTree(id, left)
		const span = treeSpan(id)
		bands.push({ left, right: left + W.get(id)!, top: span.top, bottom: span.bottom })
	}

	// 1. Source trees (true roots), packed left-to-right in input order.
	let cursor = 0
	for (const n of nodes) {
		if (parents.get(n.id)!.length !== 0) continue
		placeAndRecord(n.id, cursor)
		cursor += W.get(n.id)! + SIBLING_GAP
	}

	// 2. Join trees, in topo order so every parent is already placed (it
	// lives in a source tree or in an earlier join's tree). Centered under
	// the mean of the parents, then pushed right past any band it would
	// overlap (same-x is fine when the layer ranges are disjoint).
	for (const id of topo) {
		const ps = parents.get(id)!
		if (ps.length < 2) continue
		const w = W.get(id)!
		const span = treeSpan(id)
		let center = ps.reduce((acc, p) => acc + out.get(p)!.x, 0) / ps.length
		let moved = true
		while (moved) {
			moved = false
			for (const b of bands) {
				if (span.top > b.bottom || span.bottom < b.top) continue
				const left = center - w / 2
				const right = center + w / 2
				if (right > b.left - SIBLING_GAP && left < b.right + SIBLING_GAP) {
					center = b.right + SIBLING_GAP + w / 2
					moved = true
				}
			}
		}
		placeAndRecord(id, center - w / 2)
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

// Top-down layered layout (same orientation as the flow editor — see
// compoundLayout.ts): producers above → assets in the middle → consumers
// below.
//
// Each weakly-connected component (treating edges as undirected) is laid out
// independently with the tidy-tree algorithm above, then the components are
// packed left-to-right with a wide gutter between them, so disjoint subgraphs
// read as clearly separated boxes.
//
// `anchorId` is an optional UI affordance node (the pipeline `+` button) that
// is wired to every root via synthetic edges. It must NOT merge the otherwise-
// disjoint components, so it's excluded from component detection and instead
// re-placed centered one layer above the whole packed graph.
//
// Falls back to a stable grid if the component layout throws (defensive —
// cycles are already absorbed by feedback-edge dropping in layoutComponent).
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
		// since positions are node centers) plus the gutter.
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
				byId.set(anchorId, { x: (minX + maxX) / 2, y: minY - LAYER_H })
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
				y: Math.floor(i / cols) * LAYER_H
			})
		})
		return byId
	}
}
