import { JobService, ScriptService } from '$lib/gen'
import { computeDiff } from '$lib/components/schema/schemaUtils.svelte'

const RUNS_PAGE_SIZE = 100

export type PerpetualRunsAtPath = {
	count: number
	/** More runs than one page holds are queued at the path, so `count` is a lower bound. */
	truncated: boolean
	/** Arguments the deployed schema adds as required, removes, or retypes, compared with the runs' versions. */
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
	// A flow step running this script never restarts, whatever the script's perpetual setting.
	const runs = queued.filter((job) => !job.parent_job)
	if (runs.length === 0) {
		return undefined
	}

	const hashes = [...new Set(runs.flatMap((job) => (job.script_hash ? [job.script_hash] : [])))]
	const versions = await Promise.all(
		hashes.map((hash) => ScriptService.getScriptByHash({ workspace, hash }).catch(() => undefined))
	)
	const mismatchedArgs = new Set<string>()
	for (const version of versions) {
		if (!version) continue
		for (const [arg, { diff }] of Object.entries(computeDiff(schema, version.schema))) {
			// A new optional argument takes its default when the reused arguments lack it.
			if (diff === 'same' || (diff === 'added' && !schema?.required?.includes(arg))) continue
			mismatchedArgs.add(arg)
		}
	}

	return {
		count: runs.length,
		truncated: queued.length === RUNS_PAGE_SIZE,
		mismatchedArgs: [...mismatchedArgs]
	}
}
