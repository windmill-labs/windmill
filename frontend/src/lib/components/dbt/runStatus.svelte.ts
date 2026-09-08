/**
 * What a dbt run is doing to each relation on a model graph, live and then
 * settled.
 *
 * Two sources, in that order of authority:
 *
 * * the run's own `run_results.json`, once it has one — joined on dbt's
 *   `unique_id`, which is what both sides carry. The per-relation state table
 *   holds ONE row per relation stamped with its last writer, so reading that
 *   for a finished run would show it a later run's outcomes.
 * * `dbt_run_progress` while it is in flight, polled. Only `dbt-core-1x` emits
 *   the node events behind it, and only a database-connected worker records
 *   them, so on the other engines and on agent workers this stays empty and the
 *   graph colours at the end instead.
 *
 * One definition, used by the run page and by the editor: a second one would be
 * a second answer to "what colour is this model right now".
 */
import { JobService } from '$lib/gen'
import type { AssetGraphResponse, AssetRunState } from '$lib/components/assets/AssetGraph/types'
import { parseDbtRun, relationOutcome } from './parseDbtRun'

export function useDbtRunStatus(opts: {
	workspace: () => string | undefined
	/** The run whose progress to show, or `undefined` for a graph with no run. */
	jobId: () => string | undefined
	running: () => boolean
	/** The finished job's result, which carries a status per dbt node. */
	result: () => unknown
	/** The graph on screen, for the `unique_id` → relation mapping. */
	graph: () => AssetGraphResponse | undefined
	/** Guards every response against a navigation: a poll that outlives the run
	 *  it was issued for would colour the next one's models. */
	generation: () => number
	destroyed: () => boolean
}) {
	let polled = $state<Map<string, AssetRunState>>(new Map())

	async function load() {
		const ws = opts.workspace()
		const id = opts.jobId()
		if (!ws || !id) return
		const gen = opts.generation()
		try {
			const rows = await JobService.getRunProgress({ workspace: ws, id })
			if (gen !== opts.generation() || opts.destroyed()) return
			const next = new Map<string, AssetRunState>()
			for (const r of rows) {
				next.set(`asset:${r.asset_kind}:${r.asset_path}`, {
					status: r.status,
					rowCount: r.row_count
				})
			}
			polled = next
		} catch {
			// A progress hiccup must not blank the graph.
		}
	}

	let run = $derived(parseDbtRun(opts.result()))

	let settled = $derived.by(() => {
		if (opts.running()) return undefined
		const g = opts.graph()
		if (!run?.nodes?.length || !g) return undefined
		const assetByNode = new Map<string, string>()
		for (const a of g.assets) {
			if (a.dbt?.unique_id) assetByNode.set(a.dbt.unique_id, `asset:${a.kind}:${a.path}`)
		}
		const out = new Map<string, AssetRunState>()
		for (const n of run.nodes) {
			const id = assetByNode.get(n.unique_id)
			const outcome = id && relationOutcome(n.status, n.outcome)
			// A test or an analysis matches no relation, and a skipped node says
			// nothing about one; both are left uncoloured rather than guessed at.
			if (id && outcome) out.set(id, { status: outcome, rowCount: n.rows_affected })
		}
		return out.size > 0 ? out : undefined
	})

	return {
		/** Poll once. The caller owns the cadence — a run page ticks while its job
		 *  runs, the editor while a build does. */
		load,
		/** Drop what a previous run left, so its colours do not survive into the
		 *  next one. */
		reset: () => (polled = new Map()),
		get status() {
			return settled ?? polled
		},
		/** Whether the run settled itself, which is what tells a caller it no
		 *  longer needs to poll. */
		get isSettled() {
			return settled != undefined
		},
		get run() {
			return run
		}
	}
}
