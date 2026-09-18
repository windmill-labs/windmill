import { JobService, ScriptService, type QueuedJob } from '$lib/gen'
import { computeDiff } from '$lib/components/schema/schemaUtils.svelte'

const QUEUE_PAGE_SIZE = 1000

export type PerpetualRunsAtPath = {
	count: number
	/** Arguments the deployed schema removes, retypes or newly requires, compared with the runs' versions. */
	mismatchedArgs: string[]
}

// Every page: the deploy switches every perpetual run at the path, so one past the first page
// still needs the prompt and its version's arguments compared.
async function listQueuedAtPath(workspace: string, path: string): Promise<QueuedJob[]> {
	const jobs = new Map<string, QueuedJob>()
	for (let page = 1; ; page++) {
		const batch = await JobService.listQueue({
			workspace,
			scriptPathExact: path,
			jobKinds: 'script',
			perPage: QUEUE_PAGE_SIZE,
			page
		})
		for (const job of batch) jobs.set(job.id, job)
		if (batch.length < QUEUE_PAGE_SIZE) return [...jobs.values()]
	}
}

export async function loadPerpetualRunsAtPath(
	workspace: string,
	path: string,
	schema: { [key: string]: any } | undefined
): Promise<PerpetualRunsAtPath | undefined> {
	const queued = await listQueuedAtPath(workspace, path)
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
		mismatchedArgs: [...mismatchedArgs]
	}
}
