import { AssetService, JobService, type DbtColumnLineage } from '$lib/gen'
import {
	buildDbtColumnGraph,
	EMPTY_COLUMN_GRAPH,
	type ColumnLineageGraph
} from './columnLineageGraph'

/** Which stored dbt graph a view is drawing. A job — a run's snapshot, or the
 *  editor's parse of its own buffer — is asked through the job route, the only
 *  way to reach a graph that names no deployed version; otherwise the deployed
 *  version by hash, or the current one when there is no hash. */
export type DbtGraphPin = { jobId?: string; scriptHash?: string | number }

/** What a selection's dbt column lineage is doing right now. `loading` is
 *  separate because a project still being fetched and one that never asked for
 *  the analysis pass are the same empty graph otherwise. */
export type DbtColumnLineageState = {
	readonly graph: ColumnLineageGraph
	readonly loading: boolean
	/** The component reaches past what `graph` holds — the API cut it at the
	 *  part nearest the selection. */
	readonly truncated: boolean
	/** The request failed, so `graph` is empty for a reason that is not "this
	 *  project has no column lineage". */
	readonly failed: boolean
}

function fetchLineage(
	workspace: string,
	assetPaths: string[],
	pin: DbtGraphPin | undefined
): Promise<DbtColumnLineage> {
	return pin?.jobId
		? JobService.getDbtRunColumnLineage({ workspace, id: pin.jobId, assetPath: assetPaths })
		: AssetService.getDbtColumnLineage({
				workspace,
				assetPath: assetPaths,
				dbtScriptHash: pin?.scriptHash != undefined ? String(pin.scriptHash) : undefined
			})
}

/** Follow the selection, fetching the dbt column lineage it reaches.
 *
 *  One request per selection, whatever it reaches: the API takes every relation
 *  at once and walks out from all of them, so there is no partial answer to hold
 *  on to between selections and nothing to go stale behind a redeploy.
 *
 *  Per selection rather than off the graph response: the graph is folder-wide
 *  and a run page polls it, while this is drawn for one selection. It also means
 *  the request is never made for a project that did not opt into the analysis
 *  pass — the pane simply never shows the section.
 */
export function useDbtColumnLineage(args: {
	workspace: () => string | undefined
	/** The dbt relations to expand. The selection itself when it is one; for a
	 *  selection of another kind, every dbt relation its own lineage reaches —
	 *  a ducklake table can be derived from several, and expanding only the
	 *  first would leave the rest as leaves. */
	assetPaths: () => string[]
	/** The graph on screen, so the lineage describes the same project. */
	pin?: () => DbtGraphPin | undefined
	/** Which fetch of that graph is on screen. It changes when the view goes and
	 *  gets the graph again — a Refresh, a deploy — and asking again is the whole
	 *  point: the relation, the pin and the seeds are all unchanged by a
	 *  redeploy, so without this the pane would pair a freshly fetched model's
	 *  SQL and columns with the edges of the version before it. It is also what
	 *  retries a request that failed. */
	generation?: () => unknown
}): DbtColumnLineageState {
	let graph = $state<ColumnLineageGraph>(EMPTY_COLUMN_GRAPH)
	let loading = $state(false)
	let truncated = $state(false)
	let failed = $state(false)

	// The question the state in hand answers, and a counter deciding which answer
	// is still wanted. Neither is a cache of edges: the API returns a whole
	// component, so an answer is either the current selection's or nothing.
	let asked: string | undefined = undefined
	let latest = 0

	$effect(() => {
		const workspace = args.workspace()
		const paths = [...new Set(args.assetPaths())].sort()
		const pin = args.pin?.()
		const question = JSON.stringify([
			workspace,
			pin?.jobId,
			pin?.scriptHash,
			args.generation?.() ?? null,
			paths
		])
		// A selection re-derived from a graph that polled is the same question. Not
		// asking it again is what keeps a run page from refetching a component's
		// worth of edges every poll to redraw what is already on screen — while a
		// graph the view deliberately went and fetched moves `generation`, so that
		// IS a new question.
		if (question === asked) return
		asked = question
		const id = ++latest
		if (!workspace || paths.length === 0) {
			graph = EMPTY_COLUMN_GRAPH
			truncated = false
			failed = false
			loading = false
			return
		}
		loading = true
		fetchLineage(workspace, paths, pin).then(
			(r) => {
				if (id !== latest) return
				graph = buildDbtColumnGraph(r?.edges ?? [])
				truncated = r?.truncated ?? false
				failed = false
				loading = false
			},
			// Lineage annotates a graph that renders without it, so a failed fetch
			// leaves that branch unexpanded rather than putting an error over the
			// model — but it SAYS so, because an empty trace is what a project
			// without the analysis pass looks like, and the two must not read
			// alike. Not retried on its own: the effect reruns whenever the canvas
			// redraws, and a failing endpoint would then be asked once per redraw.
			// A Refresh moves `generation` and asks again.
			() => {
				if (id !== latest) return
				graph = EMPTY_COLUMN_GRAPH
				truncated = false
				failed = true
				loading = false
			}
		)
	})

	return {
		get graph() {
			return graph
		},
		get loading() {
			return loading
		},
		get truncated() {
			return truncated
		},
		get failed() {
			return failed
		}
	}
}
