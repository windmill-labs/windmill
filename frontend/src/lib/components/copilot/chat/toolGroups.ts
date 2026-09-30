import type { DisplayMessage, ToolDisplayMessage } from './shared'
import { DRAFT_CONFLICT_RESULT, DRAFT_SAVE_FAILED_RESULT } from './draftWriteResults'
import { webSearchResultOf } from './webSearchResult'

// A tool missing from these lists always renders as its own row, so a new write never gets
// hidden by default.

type EditedItemKind = 'flow' | 'app'

// Tools that fold into an edit group, by the kind of item they edit; the item is the call's
// `path` ('' for flow mode's tools, which edit the open flow). Scripts are left out on purpose: each script edit
// renders its own diff card, and the diff is what the user reads.
const EDIT_TOOLS: Record<string, EditedItemKind> = {
	patch_flow_json: 'flow',
	set_flow_module_code: 'flow',
	write_flow: 'flow',
	set_flow_json: 'flow',
	set_module_code: 'flow',
	set_preprocessor_module: 'flow',
	set_failure_module: 'flow',
	init_app: 'app',
	write_app_file: 'app',
	patch_app_file: 'app',
	delete_app_file: 'app',
	write_app_runnable: 'app',
	delete_app_runnable: 'app'
}
// Reads of the item being edited: the model reads a step or file before changing it, and
// splitting the group on every read would leave one group per edit.
const ITEM_READ_TOOLS: Record<string, EditedItemKind> = {
	read_flow_module_code: 'flow',
	inspect_inline_script: 'flow',
	get_lint_errors: 'flow',
	read_app_file: 'app',
	search_app: 'app'
}
// The grouped tools flow mode (flow/core.ts) offers without a `path`: they act on the flow open
// in the editor. patch_flow_json shares its name with the global tool, which takes a path.
const PATHLESS_FLOW_MODE_TOOLS = new Set([
	'set_flow_json',
	'patch_flow_json',
	'set_module_code',
	'set_preprocessor_module',
	'set_failure_module',
	'inspect_inline_script',
	'get_lint_errors'
])
// Calls that only look things up. Not derived from `planModeSafe`, which also admits test
// runs and plan-document writes.
const READ_TOOLS = new Set([
	...Object.keys(ITEM_READ_TOOLS),
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
	/** 'edit': edits of one flow or app. 'explore': consecutive lookups of anything. */
	groupKind: 'edit' | 'explore'
	/** First call's id, so the group keeps its identity (and expand state) as it grows. */
	key: string
	/** Edit groups: the item's path, or '' for the flow open in the editor. */
	target: string
	entries: { message: DisplayMessage; index: number }[]
}

export type ChatItem = { kind: 'message'; message: DisplayMessage; index: number } | ToolGroup

function groupableCall(message: DisplayMessage): ToolDisplayMessage | undefined {
	if (message.role !== 'tool' || !message.toolName) return undefined
	// A row waiting on the user, or refused by plan mode, is a decision the user must see; a
	// row with its own card (run, question, diff, image, sources) is the content itself; a call
	// held for folder instructions never ran, so it is not a change or a lookup.
	if (message.needsConfirmation || message.blockedByPlanMode) return undefined
	if (message.heldForFolderInstructions) return undefined
	if (message.runForm || message.inspectedRun || message.userQuestion) return undefined
	if (message.codeDiff || message.imageUrl || message.webSearchSources) return undefined
	if (!message.error && webSearchResultOf(message.result)) return undefined
	// The user's own refusal is a decision, not a failure to fold away.
	if (message.declinedByUser) return undefined
	return message
}

/** `path` undefined: the call's arguments have not named its item yet (still streaming, or
 * not on the row yet). */
type Membership = { kind: EditedItemKind; path: string | undefined; edit: boolean }

// While arguments stream, `parameters` is the partial JSON text. The path usually lands well
// before the large code or content field, so read it out once its closing quote has arrived.
function streamedPath(text: string): string | undefined {
	const match = /"path"\s*:\s*"((?:[^"\\]|\\.)*)"/.exec(text)
	if (!match) return undefined
	try {
		return JSON.parse(`"${match[1]}"`)
	} catch {
		return undefined
	}
}

function itemMembership(message: DisplayMessage): Membership | undefined {
	const call = groupableCall(message)
	if (!call) return undefined
	const streaming = typeof call.parameters === 'string'
	const params = streaming ? {} : (call.parameters ?? {})
	const tool = call.toolName!
	// No `parameters` at all: a call queued before its arguments reached the row (tools that do
	// not stream them), so its item is not known yet. Arguments without a path mean the open
	// flow only for a tool flow mode offers without one; any other tool requires a path, so
	// its item is unknown.
	const path = streaming
		? streamedPath(call.parameters)
		: call.parameters === undefined
			? undefined
			: typeof params.path === 'string'
				? params.path
				: PATHLESS_FLOW_MODE_TOOLS.has(tool)
					? ''
					: undefined
	if (Object.hasOwn(EDIT_TOOLS, tool)) return { kind: EDIT_TOOLS[tool], path, edit: true }
	if (Object.hasOwn(ITEM_READ_TOOLS, tool)) {
		return { kind: ITEM_READ_TOOLS[tool], path, edit: false }
	}
	if (tool === 'read_workspace_item' && (params.type === 'flow' || params.type === 'app') && path) {
		return { kind: params.type, path, edit: false }
	}
	return undefined
}

// A flow and an app can share a path, so the kind is part of what makes two calls one item. A
// call whose path has not arrived yet stays with the group it follows, or the edit in progress
// would leave its own group and the header would read as settled.
function sameItem(a: Membership | undefined, b: Membership): boolean {
	return a !== undefined && a.kind === b.kind && (a.path === undefined || a.path === b.path)
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
	accepts: (m: DisplayMessage, index: number) => boolean
) {
	let end = start
	for (let j = start + 1; j < messages.length; j++) {
		if (accepts(messages[j], j)) end = j
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
	if (groupKind === 'edit' && !calls.some((m) => itemMembership(m)?.edit)) return undefined
	return {
		kind: 'group',
		groupKind,
		key: (messages[start] as ToolDisplayMessage).tool_call_id,
		target,
		entries: run.map((message, k) => ({ message, index: start + k }))
	}
}

function editGroupAt(messages: DisplayMessage[], start: number): ToolGroup | undefined {
	const item = itemMembership(messages[start])
	// A call whose item is not known yet may join a group but not start one: two such calls
	// could be edits of different items.
	if (!item || item.path === undefined) return undefined
	const end = runEnd(messages, start, (m) => sameItem(itemMembership(m), item))
	return toolGroup(messages, start, end, 'edit', item.path)
}

export function groupToolRuns(messages: DisplayMessage[]): ChatItem[] {
	const items: ChatItem[] = []
	let i = 0
	while (i < messages.length) {
		// An edit group wins over an explore group: its reads belong to the edits they prepare,
		// so an explore run also stops before a read that starts one.
		const explores = (m: DisplayMessage, j: number) => isReadCall(m) && !editGroupAt(messages, j)
		const group =
			editGroupAt(messages, i) ??
			(isReadCall(messages[i])
				? toolGroup(messages, i, runEnd(messages, i, explores), 'explore', '')
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

// The server's full resource path: two servers can share a last segment (u/alice/github,
// f/team/github), so only the display shortens it.
function mcpServerPath(call: ToolDisplayMessage): string | undefined {
	if (call.toolName !== 'call_mcp_read_tool') return undefined
	const server = call.mcpServer?.path ?? call.parameters?.server
	return typeof server === 'string' ? server : undefined
}

function callName(call: ToolDisplayMessage): string {
	const mcpTool = call.parameters?.tool
	const name =
		call.toolName === 'call_mcp_read_tool' && typeof mcpTool === 'string'
			? mcpTool
			: (call.toolName ?? '')
	return name.replaceAll('_', ' ')
}

// A collapsed group would otherwise hide a draft save that did not land.
const FAILED_SAVE_RESULTS = new Set([DRAFT_CONFLICT_RESULT, DRAFT_SAVE_FAILED_RESULT])
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

/** `queued`: every call is still waiting its turn. Every call in a turn is queued before the
 * first one runs, so this state shows on each edit run; like a single row's queued label, it
 * says what will happen rather than what did. */
export type GroupState = 'queued' | 'running' | 'settled'

/** The group's header, e.g. `Edited` + `f/a/flow · 3 changes`,
 * `search workspace 2 times, read 2 workspace item`, `github` + `list issues, get issue 2 times`. */
export function groupHeader(
	group: ToolGroup,
	state: GroupState
): { prefix: string; label: string } {
	const calls = group.entries
		.map((e) => e.message)
		.filter((m): m is ToolDisplayMessage => m.role === 'tool')
	if (group.groupKind === 'edit') {
		const edits = calls.filter(
			(m) => Object.hasOwn(EDIT_TOOLS, m.toolName ?? '') && !callFailed(m)
		).length
		return {
			prefix: { queued: 'Edit', running: 'Editing', settled: 'Edited' }[state],
			label: `${group.target || 'the flow'} · ${plural(edits, 'change')}`
		}
	}
	const counts = new Map<string, number>()
	for (const call of calls) counts.set(callName(call), (counts.get(callName(call)) ?? 0) + 1)
	const list = [...counts].map(([name, n]) => repeated(name, n)).join(', ')
	// A tool search spans every server and usually comes right before the calls it found, so it
	// does not stop the group from reading as one server's.
	const servers = new Set(
		calls.filter((call) => call.toolName !== 'search_mcp_tools').map(mcpServerPath)
	)
	const [server] = servers
	if (servers.size === 1 && server) return { prefix: server.split('/').at(-1)!, label: list }
	return { prefix: '', label: list.charAt(0).toUpperCase() + list.slice(1) }
}
