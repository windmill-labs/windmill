/**
 * What a tool call ran with and returned, recovered from its job.
 *
 * A conversation row stores only a summary sentence ("Used X tool"), but a Windmill tool
 * runs as its own job, and that job already holds everything the card needs: `args` are
 * the arguments the model supplied, `result` is what came back, and `script_path` names
 * the tool. Reading them there keeps one copy of the data instead of two.
 *
 * Two kinds of tool are out of reach and keep the sentence:
 *  - an MCP tool runs inside the agent's own job, so its row carries no job id;
 *  - a provider-native tool (web search) runs inside the completion, and its row points at
 *    the *agent's* job — whose args are the agent's configuration, not the search's. Using
 *    them would show confidently wrong details, so an aiagent job is ignored.
 */
import { JobService } from '$lib/gen'

export type ToolCallDetails = {
	toolName?: string
	parameters?: any
	result?: any
}

const EMPTY: ToolCallDetails = {}

/** The tool's own name, which is the last segment of the job's path. */
function toolNameFromPath(path: string | undefined): string | undefined {
	const name = path?.split('/').filter(Boolean).pop()
	return name && name !== '' ? name : undefined
}

export function jobToToolCallDetails(job: any): ToolCallDetails {
	// The agent's own job means this row is a provider-native tool; its args describe the
	// agent, not the call.
	if (!job || job.job_kind === 'aiagent') return EMPTY
	const parameters =
		job.args && typeof job.args === 'object' && Object.keys(job.args).length > 0
			? job.args
			: undefined
	return {
		toolName: toolNameFromPath(job.script_path),
		parameters,
		result: job.result
	}
}

/** Per-conversation cache of tool jobs by id. One fetch per tool row while mounted. */
export class ToolCallStore {
	#workspace: () => string | undefined
	#byJob = $state<Record<string, ToolCallDetails>>({})
	#inFlight = new Set<string>()

	constructor(workspace: () => string | undefined) {
		this.#workspace = workspace
	}

	/** The call behind a tool row, fetching on first ask. Empty until the job lands. */
	get(jobId: string | null | undefined): ToolCallDetails {
		if (!jobId) return EMPTY
		const cached = this.#byJob[jobId]
		if (cached) return cached
		void this.#load(jobId)
		return EMPTY
	}

	async #load(jobId: string) {
		const workspace = this.#workspace()
		if (!workspace || this.#inFlight.has(jobId)) return
		this.#inFlight.add(jobId)
		try {
			const job = await JobService.getJob({ workspace, id: jobId, noLogs: true })
			this.#byJob = { ...this.#byJob, [jobId]: jobToToolCallDetails(job) }
		} catch {
			// A purged job, or one this user cannot read: the row keeps its summary.
			this.#byJob = { ...this.#byJob, [jobId]: EMPTY }
		} finally {
			this.#inFlight.delete(jobId)
		}
	}
}
