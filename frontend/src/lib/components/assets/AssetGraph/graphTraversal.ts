import type { AssetGraphResponse, AssetGraphSelection } from './types'
import { assetKey, buildAssetSubscribers, isWriteEdge } from './lib'

// Scripts that WRITE the selected asset (its producers), from the graph's
// `w`/`rw` lineage edges. `[]` for a non-asset selection. Shared by the route
// page and the dev preview so "who writes this asset" is defined once and can't
// drift between the two surfaces.
export function assetProducers(
	graph: AssetGraphResponse,
	selection: AssetGraphSelection | undefined
): Array<{ kind: 'script' | 'flow'; path: string; unsaved?: boolean }> {
	if (!selection || selection.kind !== 'asset') return []
	return graph.edges
		.filter((e) => {
			const access = e.access_type ?? 'r'
			return (
				(access === 'w' || access === 'rw') &&
				e.asset_kind === selection.asset_kind &&
				e.asset_path === selection.path
			)
		})
		.map((e) => ({
			kind: e.runnable_kind as 'script' | 'flow',
			path: e.runnable_path,
			unsaved: e.unsaved
		}))
}

// Execution-DAG traversal over the resolved asset graph (drafts included).
//
// The execution edges are producer→subscriber: a script S1 *produces* an
// asset A when it has a write lineage edge (`edges`, access 'w'/'rw'), and a
// script S2 *subscribes* to A when it declares `// on <A>` (`triggers`,
// trigger_kind 'asset'). Flow subscribers and self-loops are excluded,
// mirroring the backend dispatch policy (asset_dispatch.rs).
//
// Used by both the node badge counts (AssetGraphCanvas) and the dev-run
// cascade orchestrator — keep it dependency-free and pure.

/** `producer script path` → set of subscriber script paths (one hop). */
export function buildDownstreamMap(g: AssetGraphResponse): Map<string, Set<string>> {
	const subscribersByAsset = buildAssetSubscribers(g)
	const downstream = new Map<string, Set<string>>()
	for (const e of g.edges ?? []) {
		if (!isWriteEdge(e)) continue
		const subs = subscribersByAsset.get(assetKey(e))
		if (!subs) continue
		const merged = downstream.get(e.runnable_path) ?? new Set<string>()
		for (const s of subs) if (s !== e.runnable_path) merged.add(s)
		downstream.set(e.runnable_path, merged)
	}
	return downstream
}

export type DownstreamClosure = {
	/** Every script in the transitive downstream of root, root excluded. */
	nodes: string[]
	/**
	 * In-closure dependency edges, deduped: `edges.get(p)` = subscribers of
	 * `p` that are inside the closure (root included as a producer).
	 */
	edges: Map<string, Set<string>>
	/**
	 * In-closure upstream count per node (root counts). A node is ready to
	 * run once all its in-closure upstreams completed — running in that
	 * order is a topological execution of the closure.
	 */
	indegree: Map<string, number>
	/**
	 * Nodes reachable from root but unschedulable because they sit on a
	 * dependency cycle (or depend on one). They are excluded from `nodes`/
	 * `edges`/`indegree`; surfaced so callers can warn instead of hanging.
	 */
	cyclic: string[]
}

/**
 * Transitive downstream closure of `root`, ready for Kahn-style scheduling.
 * Cycle-safe: nodes on (or fed only through) a cycle are reported in
 * `cyclic` rather than deadlocking the schedule.
 */
export function computeDownstreamClosure(g: AssetGraphResponse, root: string): DownstreamClosure {
	const oneHop = buildDownstreamMap(g)

	// BFS to collect the reachable set (root excluded from `nodes`).
	const reachable = new Set<string>()
	const queue = [root]
	while (queue.length > 0) {
		const cur = queue.shift()!
		for (const next of oneHop.get(cur) ?? []) {
			if (next === root || reachable.has(next)) continue
			reachable.add(next)
			queue.push(next)
		}
	}

	// In-closure edges + indegrees (root participates as a producer only).
	const edges = new Map<string, Set<string>>()
	const indegree = new Map<string, number>()
	for (const n of reachable) indegree.set(n, 0)
	for (const p of [root, ...reachable]) {
		const subs = new Set<string>()
		for (const s of oneHop.get(p) ?? []) {
			if (!reachable.has(s)) continue
			subs.add(s)
			indegree.set(s, (indegree.get(s) ?? 0) + 1)
		}
		if (subs.size > 0) edges.set(p, subs)
	}

	// Kahn pass purely to detect what is schedulable; anything left with a
	// positive indegree at the end is on/behind a cycle. Seeded with the
	// root (the run that kicks the cascade off) — its completion is what
	// unlocks the first wave.
	const remaining = new Map(indegree)
	const ready = [root]
	const ordered: string[] = []
	while (ready.length > 0) {
		const n = ready.shift()!
		if (n !== root) ordered.push(n)
		for (const s of edges.get(n) ?? []) {
			const d = (remaining.get(s) ?? 0) - 1
			remaining.set(s, d)
			if (d === 0) ready.push(s)
		}
	}
	const cyclic = [...reachable].filter((n) => !ordered.includes(n))
	if (cyclic.length === 0) {
		return { nodes: ordered, edges, indegree, cyclic }
	}

	// Drop cyclic nodes from the schedulable structures.
	const cyclicSet = new Set(cyclic)
	const cleanEdges = new Map<string, Set<string>>()
	const cleanIndegree = new Map<string, number>()
	for (const n of ordered) cleanIndegree.set(n, 0)
	for (const [p, subs] of edges) {
		if (cyclicSet.has(p)) continue
		const kept = new Set([...subs].filter((s) => !cyclicSet.has(s)))
		if (kept.size > 0) cleanEdges.set(p, kept)
		for (const s of kept) cleanIndegree.set(s, (cleanIndegree.get(s) ?? 0) + 1)
	}
	return { nodes: ordered, edges: cleanEdges, indegree: cleanIndegree, cyclic }
}

export type InducedSchedule = {
	/** Selected scripts that are schedulable, in a topological order. */
	nodes: string[]
	/** In-set dependency edges: `edges.get(p)` = in-set subscribers of `p`. */
	edges: Map<string, Set<string>>
	/** In-set upstream count per node. */
	indegree: Map<string, number>
	/** Schedulable scripts with no in-set upstream — the schedule's seeds. */
	roots: string[]
	/**
	 * Selected scripts on (or fed only through) a cycle. Excluded from
	 * `nodes`/`edges`/`indegree`/`roots`; surfaced so callers can warn.
	 */
	cyclic: string[]
}

/**
 * Topological schedule of an arbitrary *set* of scripts (e.g. a bounded-cascade
 * selection), respecting only the dependency edges that fall *inside* the set.
 * Multi-root: every selected script with no in-set upstream is a seed.
 * Cycle-safe in the same way as `computeDownstreamClosure`.
 *
 * `oneHop` is the producer→consumer adjacency (script path → downstream script
 * paths). It defaults to the `// on` subscriber map — but a bounded run passes
 * a *read-aware* map (boundedCascade.buildLineageDownstreamMap) so a selected
 * script that merely *reads* an upstream asset still runs after its producer,
 * matching the CLI's `topoOrder`. Keep the default subscriber-only: it mirrors
 * the production asset-trigger dispatch the unbounded cascade simulates.
 */
export function computeInducedSchedule(
	g: AssetGraphResponse,
	selected: Set<string>,
	oneHop: Map<string, Set<string>> = buildDownstreamMap(g)
): InducedSchedule {
	// In-set edges + indegrees.
	const edges = new Map<string, Set<string>>()
	const indegree = new Map<string, number>()
	for (const n of selected) indegree.set(n, 0)
	for (const p of selected) {
		const subs = new Set<string>()
		for (const s of oneHop.get(p) ?? []) {
			if (!selected.has(s)) continue
			subs.add(s)
			indegree.set(s, (indegree.get(s) ?? 0) + 1)
		}
		if (subs.size > 0) edges.set(p, subs)
	}

	// Kahn from every indegree-0 node.
	const seeds = [...selected].filter((n) => (indegree.get(n) ?? 0) === 0)
	const remaining = new Map(indegree)
	const ready = [...seeds]
	const ordered: string[] = []
	while (ready.length > 0) {
		const n = ready.shift()!
		ordered.push(n)
		for (const s of edges.get(n) ?? []) {
			const d = (remaining.get(s) ?? 0) - 1
			remaining.set(s, d)
			if (d === 0) ready.push(s)
		}
	}

	const orderedSet = new Set(ordered)
	const cyclic = [...selected].filter((n) => !orderedSet.has(n))
	if (cyclic.length === 0) {
		return { nodes: ordered, edges, indegree, roots: seeds, cyclic }
	}

	// Strip cyclic nodes from the schedulable structures.
	const cyclicSet = new Set(cyclic)
	const cleanEdges = new Map<string, Set<string>>()
	const cleanIndegree = new Map<string, number>()
	for (const n of ordered) cleanIndegree.set(n, 0)
	for (const [p, subs] of edges) {
		if (cyclicSet.has(p)) continue
		const kept = new Set([...subs].filter((s) => !cyclicSet.has(s)))
		if (kept.size > 0) cleanEdges.set(p, kept)
		for (const s of kept) cleanIndegree.set(s, (cleanIndegree.get(s) ?? 0) + 1)
	}
	const roots = ordered.filter((n) => (cleanIndegree.get(n) ?? 0) === 0)
	return { nodes: ordered, edges: cleanEdges, indegree: cleanIndegree, roots, cyclic }
}
