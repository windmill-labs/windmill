import type { AssetGraphResponse } from './types'

/**
 * The pipeline's view of a folder that also holds a dbt project.
 *
 * The relations a dbt project materializes belong on this graph: they are what
 * a pipeline script downstream of a mart reads, so the lineage into it is
 * visible. The dbt *script* does not. It is not a pipeline member — it is
 * authored in a local `dbt run` loop and has its own script and run pages — so
 * rendering it beside the pipeline's own scripts is what blurs which of the two
 * a folder holds.
 *
 * Its models stay, with their `ref()` lineage. Only the node is dropped.
 */
export function hideDbtRunnables(graph: AssetGraphResponse): AssetGraphResponse {
	// Keyed by `(usage_kind, path)`, which is the graph's identity for a runnable
	// — a script and a flow may share a path, and keying on path alone would take
	// the flow's node, edges and triggers down with the dbt script's.
	const key = (usage_kind: string, path: string) => `${usage_kind}:${path}`
	const dbtRunnables = new Set(
		graph.runnables.filter((r) => r.dbt).map((r) => key(r.usage_kind, r.path))
	)
	if (dbtRunnables.size === 0) return graph

	const kept = (usage_kind: string, path: string) => !dbtRunnables.has(key(usage_kind, path))
	return {
		...graph,
		runnables: graph.runnables.filter((r) => kept(r.usage_kind, r.path)),
		edges: graph.edges.filter((e) => kept(e.runnable_kind, e.runnable_path)),
		triggers: graph.triggers.filter((t) => kept(t.runnable_kind, t.runnable_path)),
		// A macro library is a script; a dbt script defines no macros, so neither
		// end can be a flow. Kept on the script key for symmetry with the rest.
		macro_edges: graph.macro_edges?.filter(
			(m) => kept('script', m.lib_path) && kept('script', m.consumer_path)
		),
		test_edges: graph.test_edges?.filter(
			(t) => kept(t.producer_kind, t.producer_path) && kept(t.runnable_kind, t.runnable_path)
		)
	}
}
