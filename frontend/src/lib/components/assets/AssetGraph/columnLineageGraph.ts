import type { AssetKind } from '$lib/gen'
import type { AssetGraphResponse } from './types'

// A node in the column-level lineage graph: one column of one asset.
export type ColumnNode = { kind: AssetKind; path: string; column: string }
export type ColumnNodeId = string

// Collision-proof node id — JSON-encoded tuple, so a `#`/`:` inside a path or
// (quoted) column name can't merge two distinct columns into one node.
export function colNodeId(kind: AssetKind, path: string, column: string): ColumnNodeId {
	return JSON.stringify([kind, path, column])
}

// The pipeline-wide column-lineage graph, stitched across every producer. Each
// producer's `column_lineage` contributes single-hop edges (its output column ←
// its source columns); shared (asset,column) nodes chain those hops into the
// full transitive graph (`orders.amount → staging.amt → daily.total`).
export type ColumnLineageGraph = {
	nodes: Map<ColumnNodeId, ColumnNode>
	// outputColumn → the source columns it derives from (walk upstream).
	up: Map<ColumnNodeId, Set<ColumnNodeId>>
	// sourceColumn → the output columns derived from it (walk downstream).
	down: Map<ColumnNodeId, Set<ColumnNodeId>>
}

// Build the column graph from a resolved asset graph. A producer's
// `column_lineage` describes the columns of the asset it materializes; that
// output asset is the ducklake target it writes (v1 materialize target), found
// from its write-edge. Producers without a known ducklake output are skipped
// (their columns can't be anchored to an asset node).
export function buildColumnGraph(graph: AssetGraphResponse): ColumnLineageGraph {
	const nodes = new Map<ColumnNodeId, ColumnNode>()
	const up = new Map<ColumnNodeId, Set<ColumnNodeId>>()
	const down = new Map<ColumnNodeId, Set<ColumnNodeId>>()

	const addNode = (n: ColumnNode): ColumnNodeId => {
		const id = colNodeId(n.kind, n.path, n.column)
		if (!nodes.has(id)) nodes.set(id, n)
		return id
	}
	const addEdge = (src: ColumnNodeId, out: ColumnNodeId) => {
		if (src === out) return
		;(up.get(out) ?? up.set(out, new Set()).get(out)!).add(src)
		;(down.get(src) ?? down.set(src, new Set()).get(src)!).add(out)
	}

	// The output asset a runnable's `column_lineage` describes. The declared
	// `// materialize` target is authoritative (a multi-output script writes
	// several ducklake tables, and the deployed write-edges are unordered, so
	// picking "a" write-edge can anchor to the wrong asset). Fall back to a
	// ducklake write-edge only for producers with no materialize annotation
	// (e.g. a literal single-output CTAS).
	const outputAsset = new Map<string, { kind: AssetKind; path: string }>()
	for (const r of graph.runnables ?? []) {
		if (r.materialize_target) {
			outputAsset.set(`${r.usage_kind}:${r.path}`, r.materialize_target)
		}
	}
	for (const e of graph.edges ?? []) {
		const access = e.access_type ?? 'r'
		const key = `${e.runnable_kind}:${e.runnable_path}`
		if (
			(access === 'w' || access === 'rw') &&
			e.asset_kind === 'ducklake' &&
			!outputAsset.has(key)
		) {
			outputAsset.set(key, { kind: e.asset_kind, path: e.asset_path })
		}
	}

	for (const r of graph.runnables ?? []) {
		const lineage = r.column_lineage
		if (!lineage || lineage.length === 0) continue
		const out = outputAsset.get(`${r.usage_kind}:${r.path}`)
		if (!out) continue
		for (const cl of lineage) {
			const outId = addNode({ kind: out.kind, path: out.path, column: cl.column })
			for (const inp of cl.inputs) {
				const srcId = addNode({
					kind: inp.from_kind,
					path: inp.from_path,
					column: inp.from_column
				})
				addEdge(srcId, outId)
			}
		}
	}

	return { nodes, up, down }
}

// Every node reachable from `start` by following `adj` (transitive closure,
// excluding `start` itself). Iterative to avoid deep-recursion limits.
function reach(start: ColumnNodeId, adj: Map<ColumnNodeId, Set<ColumnNodeId>>): Set<ColumnNodeId> {
	const seen = new Set<ColumnNodeId>()
	const stack = [start]
	while (stack.length) {
		const n = stack.pop()!
		for (const m of adj.get(n) ?? []) {
			if (!seen.has(m)) {
				seen.add(m)
				stack.push(m)
			}
		}
	}
	return seen
}

// The full transitive trace of a column: itself + all upstream ancestors + all
// downstream descendants. This is the impact set — "everything that feeds, or
// is fed by, this column".
export function traceColumn(id: ColumnNodeId, g: ColumnLineageGraph): Set<ColumnNodeId> {
	const out = new Set<ColumnNodeId>([id])
	for (const a of reach(id, g.up)) out.add(a)
	for (const d of reach(id, g.down)) out.add(d)
	return out
}

// The connected neighborhood of a set of seed columns (an asset's columns):
// the seeds plus everything upstream and downstream of any of them. This is the
// subgraph the trace view renders around a selected asset.
export function connectedComponent(
	seeds: ColumnNodeId[],
	g: ColumnLineageGraph
): Set<ColumnNodeId> {
	const out = new Set<ColumnNodeId>()
	for (const s of seeds) {
		if (!g.nodes.has(s)) continue
		out.add(s)
		for (const a of reach(s, g.up)) out.add(a)
		for (const d of reach(s, g.down)) out.add(d)
	}
	return out
}

// All column-node ids belonging to one asset (its seed set for a trace).
export function assetColumnNodes(
	g: ColumnLineageGraph,
	kind: AssetKind,
	path: string
): ColumnNodeId[] {
	const ids: ColumnNodeId[] = []
	for (const [id, n] of g.nodes) if (n.kind === kind && n.path === path) ids.push(id)
	return ids
}

// Longest-path depth of each node within `ids`, sources at depth 0 and depth
// increasing downstream — so a left→right layout reads upstream→downstream.
// Cycle-guarded (lineage is a DAG, but be defensive).
export function computeDepths(
	ids: Set<ColumnNodeId>,
	g: ColumnLineageGraph
): Map<ColumnNodeId, number> {
	const depth = new Map<ColumnNodeId, number>()
	const visiting = new Set<ColumnNodeId>()
	const d = (id: ColumnNodeId): number => {
		const memo = depth.get(id)
		if (memo !== undefined) return memo
		if (visiting.has(id)) return 0
		visiting.add(id)
		let m = 0
		for (const u of g.up.get(id) ?? []) if (ids.has(u)) m = Math.max(m, d(u) + 1)
		visiting.delete(id)
		depth.set(id, m)
		return m
	}
	for (const id of ids) d(id)
	return depth
}
