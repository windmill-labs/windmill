import type { DisplayMessage, ToolDisplayMessage } from './shared'

// A tool missing from these lists always renders as its own row, so a new write never gets
// hidden by default.

// Tools that fold into an edit group, with the flow each call targets. Flow-mode tools edit
// the flow open in the editor, so they carry no path and all share the '' target.
const FLOW_EDIT_TOOLS = new Set([
	'patch_flow_json',
	'set_flow_module_code',
	'write_flow',
	'set_flow_json',
	'set_module_code',
	'set_preprocessor_module',
	'set_failure_module'
])
// Reads of the flow being edited: the model reads a step before patching it, and splitting
// the group on every read would leave one group per edit.
const FLOW_READ_TOOLS = new Set([
	'read_flow_module_code',
	'inspect_inline_script',
	'get_lint_errors'
])
// Calls that only look things up. Not derived from `planModeSafe`, which also admits test
// runs and plan-document writes.
const READ_TOOLS = new Set([
	...FLOW_READ_TOOLS,
	'list_workspace_items',
	'read_workspace_item',
	'search_workspace',
	'get_runnable_details',
	'search_hub_scripts',
	'search_resource_types',
	'resource_type',
	'search_docs',
	'read_docs_page',
	'get_db_schema',
	'search_npm_packages',
	'get_instructions',
	'get_instructions_for_code_generation',
	'read_skill',
	'get_trigger_schema',
	'get_schedule_schema',
	'list_runs',
	'list_workers',
	'list_app_runs',
	'get_app_runtime_logs',
	'get_preview_status',
	'get_current_page_name',
	'read_app_file',
	'search_app',
	'search_dom',
	'read_dom',
	'read_file',
	'search_files',
	'list_data_metrics',
	'list_ducklakes',
	'get_pipeline_graph',
	'read_pipeline_node',
	'list_artifacts',
	'read_artifact',
	'list_artifact_versions',
	'search_mcp_tools',
	'call_mcp_read_tool'
])

export type ToolGroup = {
	kind: 'group'
	/** 'edit': edits of one flow. 'explore': consecutive lookups of anything. */
	groupKind: 'edit' | 'explore'
	/** First call's id, so the group keeps its identity (and expand state) as it grows. */
	key: string
	/** Edit groups: the flow path, or '' for the flow open in the editor. */
	target: string
	entries: { message: DisplayMessage; index: number }[]
}

export type ChatItem = { kind: 'message'; message: DisplayMessage; index: number } | ToolGroup

function groupableCall(message: DisplayMessage): ToolDisplayMessage | undefined {
	if (message.role !== 'tool' || !message.toolName) return undefined
	// A row waiting on the user, or refused by plan mode, is a decision the user must see; a
	// row with its own card (run, question, image, sources) is the content itself.
	if (message.needsConfirmation || message.blockedByPlanMode) return undefined
	if (message.runForm || message.inspectedRun || message.userQuestion) return undefined
	if (message.imageUrl || message.webSearchSources) return undefined
	return message
}

function flowMembership(message: DisplayMessage): { target: string; edit: boolean } | undefined {
	const call = groupableCall(message)
	if (!call) return undefined
	const params = call.parameters ?? {}
	const path = typeof params.path === 'string' ? params.path : ''
	if (FLOW_EDIT_TOOLS.has(call.toolName!)) return { target: path, edit: true }
	if (FLOW_READ_TOOLS.has(call.toolName!)) return { target: path, edit: false }
	if (call.toolName === 'read_workspace_item' && params.type === 'flow' && path) {
		return { target: path, edit: false }
	}
	return undefined
}

function isReadCall(message: DisplayMessage): boolean {
	const call = groupableCall(message)
	return call !== undefined && READ_TOOLS.has(call.toolName!)
}

// Thinking between two calls stays inside the group; visible text ends it. The live
// streaming message is never absorbed, or the reasoning in progress would be hidden.
function isSilentAssistant(message: DisplayMessage): boolean {
	return message.role === 'assistant' && !message.streaming && message.content.trim() === ''
}

/** Index of the last call in the run starting at `start`, crossing silent assistant
 * messages; trailing silent messages stay outside. */
function runEnd(
	messages: DisplayMessage[],
	start: number,
	accepts: (m: DisplayMessage) => boolean
) {
	let end = start
	for (let j = start + 1; j < messages.length; j++) {
		if (accepts(messages[j])) end = j
		else if (!isSilentAssistant(messages[j])) break
	}
	return end
}

function toolGroup(
	messages: DisplayMessage[],
	start: number,
	end: number,
	groupKind: ToolGroup['groupKind'],
	target: string
): ToolGroup | undefined {
	const run = messages.slice(start, end + 1)
	const calls = run.filter((m) => m.role === 'tool')
	if (calls.length < 2) return undefined
	if (groupKind === 'edit' && !calls.some((m) => flowMembership(m)?.edit)) return undefined
	return {
		kind: 'group',
		groupKind,
		key: (messages[start] as ToolDisplayMessage).tool_call_id,
		target,
		entries: run.map((message, k) => ({ message, index: start + k }))
	}
}

export function groupToolRuns(messages: DisplayMessage[]): ChatItem[] {
	const items: ChatItem[] = []
	let i = 0
	while (i < messages.length) {
		const flow = flowMembership(messages[i])
		// An edit group wins over an explore group: its reads belong to the edits they prepare.
		const group =
			(flow &&
				toolGroup(
					messages,
					i,
					runEnd(messages, i, (m) => flowMembership(m)?.target === flow.target),
					'edit',
					flow.target
				)) ||
			(isReadCall(messages[i])
				? toolGroup(messages, i, runEnd(messages, i, isReadCall), 'explore', '')
				: undefined)
		if (group) {
			items.push(group)
			i = group.entries.at(-1)!.index + 1
		} else {
			items.push({ kind: 'message', message: messages[i], index: i })
			i++
		}
	}
	return items
}

function mcpServerName(call: ToolDisplayMessage): string | undefined {
	if (call.toolName !== 'call_mcp_read_tool') return undefined
	const server = call.mcpServer?.path ?? call.parameters?.server
	return typeof server === 'string' ? server.split('/').at(-1) : undefined
}

function callName(call: ToolDisplayMessage): string {
	const mcpTool = call.parameters?.tool
	const name =
		call.toolName === 'call_mcp_read_tool' && typeof mcpTool === 'string'
			? mcpTool
			: (call.toolName ?? '')
	return name.replaceAll('_', ' ')
}

// A draft save that failed or hit a conflict reports it through `result` alone, not `error`
// (draftWriteFailure in global/core.ts), and a collapsed group would otherwise hide it.
const FAILED_SAVE_RESULTS = new Set(['Save failed', 'Conflict'])
export function callFailed(call: ToolDisplayMessage): boolean {
	return (
		call.error !== undefined ||
		(typeof call.result === 'string' && FAILED_SAVE_RESULTS.has(call.result))
	)
}

function plural(n: number, word: string): string {
	return `${n} ${word}${n === 1 ? '' : 's'}`
}

// Each read reads one thing, so the count goes on the object ("read 3 workspace item"). On
// other verbs it would miscount what came back ("list 2 runs" after listing twice).
const COUNTED_VERBS = new Set(['read', 'inspect'])
function repeated(name: string, n: number): string {
	if (n === 1) return name
	const [verb, ...rest] = name.split(' ')
	return COUNTED_VERBS.has(verb) && rest.length > 0
		? `${verb} ${n} ${rest.join(' ')}`
		: `${name} ${n} times`
}

/** The group's header, e.g. `Edited` + `f/a/flow · 3 changes`,
 * `search workspace 2 times, read 2 workspace item`, `github` + `list issues, get issue 2 times`. */
export function groupHeader(group: ToolGroup, running: boolean): { prefix: string; label: string } {
	const calls = group.entries
		.map((e) => e.message)
		.filter((m): m is ToolDisplayMessage => m.role === 'tool')
	if (group.groupKind === 'edit') {
		const edits = calls.filter(
			(m) => FLOW_EDIT_TOOLS.has(m.toolName ?? '') && !callFailed(m)
		).length
		return {
			prefix: running ? 'Editing' : 'Edited',
			label: `${group.target || 'the flow'} · ${plural(edits, 'change')}`
		}
	}
	const counts = new Map<string, number>()
	for (const call of calls) counts.set(callName(call), (counts.get(callName(call)) ?? 0) + 1)
	const list = [...counts].map(([name, n]) => repeated(name, n)).join(', ')
	const servers = new Set(calls.map(mcpServerName))
	const [server] = servers
	if (servers.size === 1 && server) return { prefix: server, label: list }
	return { prefix: '', label: list.charAt(0).toUpperCase() + list.slice(1) }
}
