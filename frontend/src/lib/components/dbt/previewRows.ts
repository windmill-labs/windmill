import { JobService, type ScriptModule } from '$lib/gen'

/** An unsaved project, submitted as its own preview job. Held as it was sent,
 *  never re-read from the editor: a graph describes the project it was parsed
 *  from, and so must anything run against that graph's nodes. */
export type DbtPreviewBuffer = {
	content: string
	modules: Record<string, ScriptModule> | undefined
	tag?: string
	timeout?: number
	/** The arguments the project was parsed under, in run-form shape. Part of the
	 *  snapshot because vars decide `enabled`, schemas and aliases: run a preview
	 *  under later ones and it can address a relation this graph never had. */
	args: Record<string, unknown> | undefined
}

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
 * Which project runs is decided by the caller, and must match the graph the SQL
 * on screen came from: a deployed version by hash, a buffer parse by shipping
 * that same buffer.
 *
 * `stillWanted` is asked before each poll and before the result is used, so a
 * preview outlives neither the page that asked for it nor a navigation.
 */
export async function previewDbtRows(opts: {
	workspace: string
	scriptPath: string
	/** Pins the preview to a deployed version, for a graph showing that version. */
	scriptHash?: string | number
	/** The project the graph was parsed from, when that was a buffer. Required
	 *  then, because there may be no deployed version at all — and when there
	 *  is, it can lack the model or build it from other SQL. */
	buffer?: DbtPreviewBuffer
	/** One node: a model name, or `package.model` where two packages share one. */
	model: string
	/** The run's own vars, so a descriptor with a required `{{ }}` var resolves. */
	vars?: Record<string, unknown>
	limit?: number
	/** Extra top-level arguments — a run's `{{ placeholder }}` values. */
	args?: Record<string, unknown>
	stillWanted?: () => boolean
}): Promise<DbtPreview | undefined> {
	const { workspace, scriptPath, scriptHash, buffer, model, vars, limit, args, stillWanted } = opts
	const startedAt = Date.now()
	const requestBody = {
		...(args ?? {}),
		command: { label: 'show', vars: vars ?? {}, model, limit: limit ?? 25 }
	}
	try {
		// The BUFFER when the caller has one, because then the graph and the SQL
		// above these rows are the buffer's: a model added since the deploy exists
		// in no other project, and one whose SQL changed builds different rows
		// there. It ships whole — dbt resolves `ref()` project-wide.
		//
		// Otherwise by HASH whenever the caller pins one: the SQL on screen is that
		// version's, and running the deployed one would show today's rows under
		// it — or fail outright for a model since removed.
		const id = buffer
			? await JobService.runScriptPreview({
					workspace,
					timeout: buffer.timeout,
					requestBody: {
						path: scriptPath,
						content: buffer.content,
						language: 'dbt',
						tag: buffer.tag,
						modules: buffer.modules,
						args: requestBody
					}
				})
			: scriptHash
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
