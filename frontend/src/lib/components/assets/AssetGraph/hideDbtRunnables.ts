import type { AssetGraphResponse } from './types'

/**
 * The pipeline's view of a folder that also holds a dbt project.
 *
 * The relations a dbt project materializes belong on this graph: they are what
 * a pipeline script downstream of a mart reads, and what makes it cascade from
 * the dbt run. The dbt *script* does not. It is not a pipeline member — it is
 * authored in a local `dbt run` loop and has its own script and run pages — so
 * rendering it beside the pipeline's own scripts is what blurs which of the two
 * a folder holds.
 *
 * Its models stay, with their `ref()` lineage. Only the node is dropped.
 */
export function hideDbtRunnables(graph: AssetGraphResponse): AssetGraphResponse {
	const dbtRunnables = new Set(graph.runnables.filter((r) => r.dbt).map((r) => r.path))
	if (dbtRunnables.size === 0) return graph

	const kept = (path: string) => !dbtRunnables.has(path)
	return {
		...graph,
		runnables: graph.runnables.filter((r) => kept(r.path)),
		edges: graph.edges.filter((e) => kept(e.runnable_path)),
		triggers: graph.triggers.filter((t) => kept(t.runnable_path)),
		macro_edges: graph.macro_edges?.filter((m) => kept(m.lib_path) && kept(m.consumer_path)),
		test_edges: graph.test_edges?.filter((t) => kept(t.producer_path) && kept(t.runnable_path))
	}
}
