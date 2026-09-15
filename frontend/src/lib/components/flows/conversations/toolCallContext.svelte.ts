/**
 * What a tool call ran with and returned, recovered from its job.
 *
 * A conversation row stores only a summary sentence ("Used X tool"), but a Windmill tool
 * runs as its own job, and that job already holds everything the card needs: `args` are
 * the arguments the model supplied, `result` is what came back, and `script_path` names
 * the tool. Reading them there keeps one copy of the data instead of two.
 *
 * Two kinds of tool are out of reach and keep the sentence: an MCP tool, and a
 * provider-native one (web search). Neither gets a job of its own — both run inside the
 * agent's — so both rows name the agent's job, which retention needs to collect them. Its
 * args are the agent's configuration and its result the agent's answer, so reading them
 * would show confidently wrong details: an `aiagent` job is ignored here, and an MCP row
 * stores its own call on the row instead (`tool_arguments` / `tool_result`).
 */
import { JobService } from '$lib/gen'
import { JobBackedStore } from './jobBackedStore.svelte'

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
	// The agent's own job means this row is a tool that ran inside it — MCP or provider-native
	// — and names that job so retention can collect it. Its args describe the agent and its
	// result is the agent's answer, so reading either would show confidently wrong details.
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

/** The tool jobs behind the transcript's tool rows. One fetch per row while mounted. */
export class ToolCallStore extends JobBackedStore<ToolCallDetails> {
	constructor(workspace: () => string | undefined) {
		super(workspace, EMPTY, async (ws, jobId) =>
			jobToToolCallDetails(await JobService.getJob({ workspace: ws, id: jobId, noLogs: true }))
		)
	}
}
