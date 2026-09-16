/**
 * A tool call's arguments and result, read from the job a Windmill tool runs as, a nested
 * agent included. An MCP or provider-native tool has no job of its own and its row names
 * the owning agent's job instead, whose args and result describe that agent.
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

/** A nested agent tool runs at `<agent path>/tools/<tool id>`; a flow's agent step at
 *  `<flow path>/<step id>`. */
function isNestedAgentTool(path: string | undefined): boolean {
	const segments = path?.split('/') ?? []
	return segments.length >= 3 && segments[segments.length - 2] === 'tools'
}

export function jobToToolCallDetails(job: any): ToolCallDetails {
	// An owning agent's job would show the agent's configuration as the tool's arguments
	// and its answer as the tool's result.
	if (!job || (job.job_kind === 'aiagent' && !isNestedAgentTool(job.script_path))) return EMPTY
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
