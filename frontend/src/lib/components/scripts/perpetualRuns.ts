import { JobService, ScriptService, type QueuedJob, type Script } from '$lib/gen'
import { computeDiff } from '$lib/components/schema/schemaUtils.svelte'

const QUEUE_PAGE_SIZE = 1000

export type PerpetualRunsAtPath = {
	/** Undefined when the runs at the path could not be listed: the deploy restarts them either way. */
	count: number | undefined
	/** Arguments the deployed schema removes, retypes or newly requires, compared with the runs' versions. */
	mismatchedArgs: string[]
}

const RUNS_UNKNOWN: PerpetualRunsAtPath = { count: undefined, mismatchedArgs: [] }

/** By path rather than by the ids listed: a loop restarts every 10s, so the run listed when the
 * modal opened is usually gone, and this endpoint makes the two passes that window needs. */
export async function stopPerpetualRuns(workspace: string, path: string): Promise<void> {
	await JobService.cancelPersistentQueuedJobs({
		workspace,
		path,
		requestBody: { reason: 'stopped before a new version was deployed' }
	})
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
	// A deploy restarts every perpetual run at the path whatever this finds, so anything it cannot
	// read leaves the count unknown rather than reporting none and skipping the prompt.
	let queued: QueuedJob[]
	try {
		queued = await listQueuedAtPath(workspace, path)
	} catch (error) {
		console.error('Could not list the runs of this perpetual script', error)
		return RUNS_UNKNOWN
	}
	// Only what the backend restarts: never a flow step or a run already canceled, and only a run
	// of a perpetual version.
	const candidates = queued.filter((job) => !job.is_flow_step && !job.canceled && job.script_hash)
	const hashes = [...new Set(candidates.map((job) => job.script_hash!))]
	let versions: Map<string, Script>
	try {
		versions = new Map(
			await Promise.all(
				hashes.map(
					async (hash) => [hash, await ScriptService.getScriptByHash({ workspace, hash })] as const
				)
			)
		)
	} catch (error) {
		console.error('Could not read the versions the runs of this script are on', error)
		return RUNS_UNKNOWN
	}
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
