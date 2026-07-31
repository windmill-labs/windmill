import { JobService } from '$lib/gen'

/** A model's rows, as `dbt show` returns them. */
export type DbtPreview =
	| { pending: true }
	| { rows: Record<string, unknown>[]; node?: string; tookMs: number }
	| { error: string }

/**
 * Preview one model's rows by running its own project's `dbt show`.
 *
 * A job, not a query: the rows come from the warehouse through the project's
 * profile, with its vars and its adapter, which is the only place that knows how
 * to resolve `ref()` and where the relation actually lives. `show` is therefore
 * not a run-form command — it is what a table's preview is made of, here and on
 * the run page's graph.
 *
 * `stillWanted` is asked before each poll and before the result is used, so a
 * preview outlives neither the page that asked for it nor a navigation.
 */
export async function previewDbtRows(opts: {
	workspace: string
	scriptPath: string
	/** Pins the preview to a deployed version, for a graph showing that version. */
	scriptHash?: string | number
	/** One node: a model name, or `package.model` where two packages share one. */
	model: string
	/** The run's own vars, so a descriptor with a required `{{ }}` var resolves. */
	vars?: Record<string, unknown>
	limit?: number
	/** Extra top-level arguments — a run's `{{ placeholder }}` values. */
	args?: Record<string, unknown>
	stillWanted?: () => boolean
}): Promise<DbtPreview | undefined> {
	const { workspace, scriptPath, scriptHash, model, vars, limit, args, stillWanted } = opts
	const startedAt = Date.now()
	const requestBody = {
		...(args ?? {}),
		command: { label: 'show', vars: vars ?? {}, model, limit: limit ?? 25 }
	}
	try {
		// By HASH whenever the caller pins one: the SQL on screen is that version's,
		// and running the deployed one would show today's rows under it — or fail
		// outright for a model since removed.
		const id = scriptHash
			? await JobService.runScriptByHash({
					workspace,
					hash: String(scriptHash),
					requestBody
				})
			: await JobService.runScriptByPath({ workspace, path: scriptPath, requestBody })
		// Polled rather than awaited: a preview is a job, and its engine may need
		// provisioning on a cold worker.
		for (let i = 0; i < 90; i++) {
			await new Promise((r) => setTimeout(r, 1000))
			if (stillWanted && !stillWanted()) return undefined
			const done = await JobService.getCompletedJobResultMaybe({ workspace, id })
			if (!done.completed) continue
			const res = done.result as { node?: string; show?: Record<string, unknown>[] } | undefined
			return done.success && res?.show
				? { rows: res.show, node: res.node, tookMs: Date.now() - startedAt }
				: { error: 'The preview job failed — open it from Runs for the detail.' }
		}
		return { error: 'The preview is still running; open it from Runs.' }
	} catch (e) {
		return { error: e instanceof Error ? e.message : String(e) }
	}
}
