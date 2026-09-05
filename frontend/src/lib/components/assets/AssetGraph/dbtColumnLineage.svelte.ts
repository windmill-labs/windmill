import { AssetService, JobService } from '$lib/gen'
import { buildDbtColumnGraph, type ColumnLineageGraph } from './columnLineageGraph'

export const EMPTY_COLUMN_GRAPH: ColumnLineageGraph = {
	nodes: new Map(),
	up: new Map(),
	down: new Map()
}

/** Which stored dbt graph a view is drawing. A job — a run's snapshot, or the
 *  editor's parse of its own buffer — is asked through the job route, the only
 *  way to reach a graph that names no deployed version; otherwise the deployed
 *  version by hash, or the current one when there is no hash. */
export type DbtGraphPin = { jobId?: string; scriptHash?: string | number }

/** What a dbt relation's column lineage is doing right now. `loading` is
 *  separate because a project still being fetched and one that never asked for
 *  the analysis pass are the same empty graph otherwise. */
export type DbtColumnLineageState = {
	readonly graph: ColumnLineageGraph
	readonly loading: boolean
}

/** Follow the selection, fetching the selected dbt relation's column lineage.
 *
 *  Per asset rather than off the graph response: the graph is folder-wide and a
 *  run page polls it, while this is drawn for one selection. It also means the
 *  request is never made for a project that did not opt into the analysis pass
 *  — the pane simply never shows the section.
 */
export function useDbtColumnLineage(args: {
	workspace: () => string | undefined
	/** The selected dbt relation, or undefined for any other selection. */
	assetPath: () => string | undefined
	/** The graph on screen, so the lineage describes the same project. */
	pin?: () => DbtGraphPin | undefined
}): DbtColumnLineageState {
	let graph = $state<ColumnLineageGraph>(EMPTY_COLUMN_GRAPH)
	let loading = $state(false)

	// What the graph in hand describes, so a selection already inside it can be
	// recognised without asking again.
	let held: { workspace: string; pin: string } | undefined = undefined

	$effect(() => {
		const workspace = args.workspace()
		const assetPath = args.assetPath()
		const pin = args.pin?.()
		const jobId = pin?.jobId
		const scriptHash = pin?.scriptHash
		if (!workspace || !assetPath) {
			graph = EMPTY_COLUMN_GRAPH
			loading = false
			return
		}
		// The answer is one connected component, so every relation inside the one
		// already held has the same answer — which is most clicks, since a
		// project's models are connected by construction. Keyed to the graph the
		// component was fetched against: the same relation under a different pin
		// is a different project.
		const key = `${workspace}|${jobId ?? ''}|${scriptHash ?? ''}`
		if (held?.workspace === workspace && held.pin === key) {
			for (const n of graph.nodes.values()) {
				if (n.path === assetPath) {
					loading = false
					return
				}
			}
		}
		// A selection changes faster than a request completes, so an answer is
		// applied only while it is still the one being asked for.
		let current = true
		loading = true
		const req = jobId
			? JobService.getDbtRunColumnLineage({ workspace, id: jobId, assetPath })
			: AssetService.getDbtColumnLineage({
					workspace,
					assetPath,
					dbtScriptHash: scriptHash != undefined ? String(scriptHash) : undefined
				})
		req.then(
			(r) => {
				if (!current) return
				graph = buildDbtColumnGraph(r?.edges ?? [])
				held = { workspace, pin: key }
				loading = false
			},
			() => {
				// Lineage annotates a graph that renders without it, so a failed
				// fetch shows no section rather than an error over the model.
				if (!current) return
				graph = EMPTY_COLUMN_GRAPH
				held = undefined
				loading = false
			}
		)
		return () => {
			current = false
		}
	})

	return {
		get graph() {
			return graph
		},
		get loading() {
			return loading
		}
	}
}
