/**
 * A tool call's arguments and result, read from the job a Windmill tool runs as, a nested
 * agent included. An MCP or provider-native tool has no job of its own and its row names
 * the owning agent's job instead, whose args and result describe that agent.
 */
import { JobService, type FlowModule, type InputTransform } from '$lib/gen'
import { dfs } from '../dfs'
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

/** The worker's `is_completed_input_transform`: only these are evaluated into a tool job's
 *  args, beside what the model supplied, and they can resolve secrets. */
function isCompletedTransform(transform: InputTransform | undefined): boolean {
	if (transform?.type === 'static') return transform.value !== undefined && transform.value !== null
	if (transform?.type === 'javascript') return transform.expr.trim() !== ''
	return false
}

/** The flow's tools a job can be the run of: a flow-defined tool by its path suffix, a
 *  workspace script tool by its script path. */
function toolModulesFor(modules: FlowModule[], jobPath: string): FlowModule[] {
	return dfs(modules, (module) => {
		if (module.value.type !== 'aiagent') return []
		return ((module.value.tools ?? []) as FlowModule[]).filter(
			(tool) =>
				jobPath.endsWith(`/${module.id}/tools/${tool.id}`) ||
				(tool.value.type === 'script' && tool.value.path === jobPath)
		)
	}).flat()
}

/**
 * The arguments the model supplied, which is all the live card shows. None when no tool in
 * the flow matches the job (edited since, say): which inputs a transform filled is unknown.
 */
export function modelArguments(
	args: Record<string, any> | undefined,
	jobPath: string | undefined,
	modules: FlowModule[] | undefined
): Record<string, any> | undefined {
	if (!args || typeof args !== 'object' || !jobPath || !modules) return undefined
	const tools = toolModulesFor(modules, jobPath)
	if (tools.length === 0) return undefined
	const picked = Object.fromEntries(
		Object.entries(args).filter(([name]) =>
			tools.every((tool) => !isCompletedTransform((tool.value as any).input_transforms?.[name]))
		)
	)
	return Object.keys(picked).length > 0 ? picked : undefined
}

export function jobToToolCallDetails(job: any, modules?: FlowModule[]): ToolCallDetails {
	// An owning agent's job would show the agent's configuration as the tool's arguments
	// and its answer as the tool's result.
	if (!job || (job.job_kind === 'aiagent' && !isNestedAgentTool(job.script_path))) return EMPTY
	return {
		toolName: toolNameFromPath(job.script_path),
		parameters: modelArguments(job.args, job.script_path, modules),
		result: job.result
	}
}

/** The tool jobs behind the transcript's tool rows. One fetch per row while mounted. */
export class ToolCallStore extends JobBackedStore<ToolCallDetails> {
	constructor(workspace: () => string | undefined, modules: () => FlowModule[] | undefined) {
		super(workspace, EMPTY, async (ws, jobId) =>
			jobToToolCallDetails(
				await JobService.getJob({ workspace: ws, id: jobId, noLogs: true }),
				modules()
			)
		)
	}
}
