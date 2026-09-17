import { JobService, ScriptService } from '$lib/gen'
import { computeDiff } from '$lib/components/schema/schemaUtils.svelte'

const RUNS_PAGE_SIZE = 100

export type PerpetualRunsAtPath = {
	count: number
	/** More runs than one page holds are queued at the path, so `count` is a lower bound. */
	truncated: boolean
	/** Arguments the deployed schema removes, retypes or newly requires, compared with the runs' versions. */
	mismatchedArgs: string[]
}

export async function loadPerpetualRunsAtPath(
	workspace: string,
	path: string,
	schema: { [key: string]: any } | undefined
): Promise<PerpetualRunsAtPath | undefined> {
	const queued = await JobService.listQueue({
		workspace,
		scriptPathExact: path,
		jobKinds: 'script',
		perPage: RUNS_PAGE_SIZE
	})
	// Only what the backend restarts: never a flow step, and only a run of a perpetual version.
	const candidates = queued.filter((job) => !job.is_flow_step && job.script_hash)
	const hashes = [...new Set(candidates.map((job) => job.script_hash!))]
	const versions = new Map(
		await Promise.all(
			hashes.map(
				async (hash) =>
					[
						hash,
						await ScriptService.getScriptByHash({ workspace, hash }).catch(() => undefined)
					] as const
			)
		)
	)
	const runs = candidates.filter((job) => versions.get(job.script_hash!)?.restart_unless_cancelled)
	if (runs.length === 0) {
		return undefined
	}

	const mismatchedArgs = new Set<string>()
	for (const hash of new Set(runs.map((job) => job.script_hash!))) {
		const previous = versions.get(hash)?.schema
		for (const [arg, { diff }] of Object.entries(computeDiff(schema, previous))) {
			// An added argument only breaks a reused run when it is required, checked below.
			if (diff !== 'same' && diff !== 'added') mismatchedArgs.add(arg)
		}
		const previouslyRequired: unknown[] = Array.isArray(previous?.required) ? previous.required : []
		for (const arg of schema?.required ?? []) {
			if (!previouslyRequired.includes(arg)) mismatchedArgs.add(arg)
		}
	}

	return {
		count: runs.length,
		truncated: queued.length === RUNS_PAGE_SIZE,
		mismatchedArgs: [...mismatchedArgs]
	}
}
