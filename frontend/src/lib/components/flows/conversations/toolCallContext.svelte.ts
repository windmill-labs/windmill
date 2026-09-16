/**
 * A tool call's arguments and result, read from the job a Windmill tool runs as. An MCP or
 * provider-native tool has no job of its own and its row names the agent's job instead,
 * whose args and result describe the agent, so an `aiagent` job reads as nothing.
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
