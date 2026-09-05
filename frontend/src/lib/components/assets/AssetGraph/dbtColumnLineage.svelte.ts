import { untrack } from 'svelte'
import { AssetService, JobService } from '$lib/gen'
import {
	buildDbtColumnGraph,
	mergeColumnGraphs,
	type ColumnLineageGraph
} from './columnLineageGraph'

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

function pinKey(workspace: string, pin: DbtGraphPin | undefined): string {
	return `${workspace}|${pin?.jobId ?? ''}|${pin?.scriptHash ?? ''}`
}

function fetchComponent(
	workspace: string,
	assetPath: string,
	pin: DbtGraphPin | undefined
): Promise<ColumnLineageGraph> {
	const req = pin?.jobId
		? JobService.getDbtRunColumnLineage({ workspace, id: pin.jobId, assetPath })
		: AssetService.getDbtColumnLineage({
				workspace,
				assetPath,
				dbtScriptHash: pin?.scriptHash != undefined ? String(pin.scriptHash) : undefined
			})
	return req.then(
		(r) => buildDbtColumnGraph(r?.edges ?? []),
		// Lineage annotates a graph that renders without it, so a failed fetch
		// leaves that branch unexpanded rather than putting an error over the
		// model — and one failed boundary does not lose the others.
		() => EMPTY_COLUMN_GRAPH
	)
}

/** Follow the selection, fetching the dbt column lineage it reaches.
 *
 *  Per asset rather than off the graph response: the graph is folder-wide and a
 *  run page polls it, while this is drawn for one selection. It also means the
 *  request is never made for a project that did not opt into the analysis pass
 *  — the pane simply never shows the section.
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
}): DbtColumnLineageState {
	let graph = $state<ColumnLineageGraph>(EMPTY_COLUMN_GRAPH)
	let loading = $state(false)

	// Which pin the graph in hand was fetched against, and which relations were
	// actually ASKED about under it.
	let heldPin: string | undefined = undefined
	let asked = new Set<string>()

	$effect(() => {
		const workspace = args.workspace()
		const paths = args.assetPaths()
		const pin = args.pin?.()
		if (!workspace || paths.length === 0) {
			graph = EMPTY_COLUMN_GRAPH
			heldPin = undefined
			asked = new Set()
			loading = false
			return
		}
		const key = pinKey(workspace, pin)
		// `untrack`: this effect writes `graph`, so reading it as a dependency
		// would make it retrigger itself forever.
		const fresh = heldPin !== key
		const base = untrack(() => (fresh ? EMPTY_COLUMN_GRAPH : graph))
		if (fresh) asked = new Set()
		// Only a relation this pin has ASKED about is skipped, not every relation
		// present in what came back. A relation two projects describe has an owner
		// row in each, and a component fetched for one of them carries that
		// relation as an endpoint without the other project's half — so treating
		// "appears in the graph" as "resolved" would hide exactly the cross-project
		// edges the server's relation-keyed walk exists to merge.
		const missing = paths.filter((p) => !asked.has(p))
		if (missing.length === 0) {
			graph = base
			loading = false
			return
		}
		// A selection changes faster than a request completes, so an answer is
		// applied only while it is still the one being asked for.
		let current = true
		loading = true
		Promise.all(missing.map((p) => fetchComponent(workspace, p, pin))).then((parts) => {
			if (!current) return
			graph = mergeColumnGraphs(base, ...parts)
			heldPin = key
			for (const p of missing) asked.add(p)
			loading = false
		})
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
