import {
	isActiveRunForm,
	type DisplayMessage,
	type TestedItemKind,
	type Tool,
	type ToolDisplayMessage
} from './shared'
import { DRAFT_CONFLICT_RESULT, DRAFT_SAVE_FAILED_RESULT } from './draftWriteResults'
import { webSearchResultOf } from './webSearchResult'

// A tool missing from these lists, whose definition sets none of `lookup`, `testRun` and
// `draftWrite`, renders as its own row, so a new write never gets hidden by default.

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
// The edits a test group takes between two runs of a script, besides the flow and app tools
// above. edit_code is the script editor's own, which acts on the open script.
const SCRIPT_EDIT_TOOLS = new Set(['write_script', 'edit_script', 'edit_code'])

/** What the definitions in the chat showing the transcript declare, by tool name: the tools
 * that set `lookup`, those that set `testRun` with the kind of item they run, and those that
 * set `draftWrite` with the noun they count as. A name that chat does not define (a flow's own
 * agent tools) is none of these. */
export type GroupingTools = {
	lookups: ReadonlySet<string>
	testRuns: ReadonlyMap<string, TestedItemKind>
	draftWrites: ReadonlyMap<string, string>
}

export function groupingToolsOf(tools: Iterable<Tool<any>>): GroupingTools {
	const lookups = new Set<string>()
	const testRuns = new Map<string, TestedItemKind>()
	const draftWrites = new Map<string, string>()
	for (const tool of tools) {
		const name = tool.def.function.name
		if (tool.lookup) lookups.add(name)
		if (tool.testRun) testRuns.set(name, tool.testRun)
		if (tool.draftWrite) draftWrites.set(name, tool.draftWrite)
	}
	return { lookups, testRuns, draftWrites }
}

export type ToolGroup = {
	kind: 'group'
	/** 'edit': edits of one flow or app. 'draft': draft writes of schedules, triggers,
	 * resources and variables. 'explore': consecutive lookups of anything.
	 * 'waiting': calls queued behind the one running or awaiting the user. 'test': test runs
	 * of one item and its edits between them; it always ends on a run, whose card stays shown.
	 */
	groupKind: 'edit' | 'draft' | 'explore' | 'waiting' | 'test'
	/** First call's id, so the group keeps its identity (and expand state) as it grows. */
	key: string
	/** Edit and test groups: the item's path, or '' for the item open in the editor. */
	target: string
	/** Draft groups: the noun each draft write tool's calls are counted as. */
	draftNouns?: ReadonlyMap<string, string>
	entries: { message: DisplayMessage; index: number }[]
	/** Test groups: the kind of item run, and how many of the entries are its runs. */
	tested?: { kind: TestedItemKind; runs: number }
}

export type ChatItem = { kind: 'message'; message: DisplayMessage; index: number } | ToolGroup

/** A tool row is keyed by its call, not its position: loading another chat puts a different
 * call at the same index, and a reused row would carry the old call's held label over. */
export function chatItemKey(item: ChatItem): string {
	if (item.kind === 'group') return `g:${item.key}`
	if (item.message.role === 'tool') return `t:${item.message.tool_call_id}`
	return `m:${item.index}`
}

// A row waiting on the user, or refused by plan mode, is a decision the user must see, and so
// is the user's own refusal; a call held for folder instructions never ran, so it is not a
// change, a lookup or a run. No group takes these.
function standsAlone(message: ToolDisplayMessage): boolean {
	return Boolean(
		message.needsConfirmation ||
			message.blockedByPlanMode ||
			(message.outOfScope && !message.outOfScope.decision) ||
			message.heldForFolderInstructions ||
			message.userQuestion ||
			message.secretRequest ||
			message.declinedByUser
	)
}

// A row with its own card (run, image, sources) is the content itself.
function hasContentCard(message: ToolDisplayMessage): boolean {
	return Boolean(
		message.runForm ||
			message.inspectedRun ||
			message.imageUrl ||
			message.webSearchSources ||
			(!message.error && webSearchResultOf(message.result))
	)
}

function groupableCall(message: DisplayMessage): ToolDisplayMessage | undefined {
	if (message.role !== 'tool' || !message.toolName) return undefined
	if (standsAlone(message) || hasContentCard(message)) return undefined
	// A diff is the content too, but an app edit's diff covers one file of many, so it stays a
	// row inside the app's group.
	if (message.codeDiff && EDIT_TOOLS[message.toolName] !== 'app') return undefined
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

// No `parameters` at all: a call queued before its arguments reached the row (tools that do
// not stream them), so its item is not known yet. Arguments without a path mean the open item
// only for a tool that can be offered without one; any other tool requires a path, so its
// item is unknown.
function callPath(call: ToolDisplayMessage, pathless: boolean): string | undefined {
	if (typeof call.parameters === 'string') return streamedPath(call.parameters)
	if (call.parameters === undefined) return undefined
	// Arguments of JSON `null` reach the row too; they name no path.
	const path = callParams(call).path
	if (typeof path === 'string') return path
	return pathless ? '' : undefined
}

function callParams(call: ToolDisplayMessage): Record<string, unknown> {
	return typeof call.parameters === 'object' && call.parameters ? call.parameters : {}
}

function itemMembership(message: DisplayMessage): Membership | undefined {
	const call = groupableCall(message)
	if (!call) return undefined
	const params = callParams(call)
	const tool = call.toolName!
	const path = callPath(call, PATHLESS_FLOW_MODE_TOOLS.has(tool))
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

// An agent is drafted through write_resource but gets its own way in (a preview chip, or an
// Open agent action), and a group shows only one chip: it stays a row. Read from the
// arguments, not the chip, which lands only once the save does. `resource_type` streams after
// the value, so until it has arrived the call may be an agent and joins no group.
function isNonAgentResourceWrite(call: ToolDisplayMessage): boolean {
	const type =
		typeof call.parameters === 'string'
			? /"resource_type"\s*:\s*"([^"]*)"/.exec(call.parameters)?.[1]
			: call.parameters?.resource_type
	return typeof type === 'string' && type !== 'ai_agent'
}

function isDraftWrite(message: DisplayMessage, tools: GroupingTools): boolean {
	const call = groupableCall(message)
	if (call === undefined || !tools.draftWrites.has(call.toolName!)) return false
	return call.toolName !== 'write_resource' || isNonAgentResourceWrite(call)
}

function isReadCall(message: DisplayMessage, tools: GroupingTools): boolean {
	const call = groupableCall(message)
	return call !== undefined && tools.lookups.has(call.toolName!)
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

function isWaitingCall(message: DisplayMessage): boolean {
	return message.role === 'tool' && message.isQueued === true && message.error === undefined
}

function inFlight(message: DisplayMessage): boolean {
	return (
		message.role === 'tool' &&
		Boolean(message.isLoading || message.isQueued || message.isStreamingArguments)
	)
}

// Calls of one turn run one at a time, so the ones queued behind a call fold into a single
// row: each would otherwise sit as its own row until its turn. Only behind a call still in
// flight: between two calls the previous one has settled, and the call next in line must
// still show as itself rather than dip into the row and back out.
function waitingGroupAt(
	messages: DisplayMessage[],
	start: number,
	previous: ChatItem | undefined,
	tools: GroupingTools
): ToolGroup | undefined {
	if (!isWaitingCall(messages[start]) || !previous) return undefined
	const ahead =
		previous.kind === 'group' ? previous.entries.map((e) => e.message) : [previous.message]
	if (!ahead.some(inFlight)) return undefined
	const end = runEnd(
		messages,
		start,
		(m, j) => isWaitingCall(m) && !kindGroupAt(messages, j, tools)
	)
	return {
		kind: 'group',
		groupKind: 'waiting',
		key: (messages[start] as ToolDisplayMessage).tool_call_id,
		target: '',
		entries: messages.slice(start, end + 1).map((message, k) => ({ message, index: start + k }))
	}
}

type TestedItem = { kind: TestedItemKind; path: string | undefined }

// A run form waiting for the user's arguments is the thing they must act on, so it stays out
// of every group until it is answered, and so does a run that has not reached its form yet:
// joining before the form opens would fold the group, then unfold it as the form appears.
function testRunOf(message: DisplayMessage, tools: GroupingTools): TestedItem | undefined {
	if (message.role !== 'tool' || !message.toolName) return undefined
	const kind = tools.testRuns.get(message.toolName)
	if (!kind) return undefined
	if (standsAlone(message) || isActiveRunForm(message)) return undefined
	if (message.isQueued || message.isStreamingArguments) return undefined
	// Failed with no job result: refused before any job started (an item that does not exist,
	// arguments that do not parse), so it never ran and its row names no item to group by.
	if (!message.runForm && message.error !== undefined && message.result === undefined) {
		return undefined
	}
	// test_run_app_runnable runs without a form and nests its inputs under `args`, so its
	// `path` is the app's.
	if (kind === 'app') return { kind, path: callPath(message, false) }
	// The global chat's runs open their form once prepared, so a loading row without one has
	// not got there yet. The editors' runs never have a form, so they join once settled.
	if (!message.runForm && message.isLoading) return undefined
	// A run form replaces the row's `parameters` with the run's arguments, so its item is read
	// off the form. test_run_code runs the code it is sent, so its runs are all of one item.
	// The editors' runs act on the open item: their parameters are the run's own inputs, where
	// a `path` is an argument, not an item.
	if (message.runForm && kind !== 'code') return { kind, path: message.runForm.path }
	return { kind, path: '' }
}

// The kind of item a call edits, for the edits a test group takes and counts. An agent is an
// `ai_agent` resource, so it is edited through write_resource.
function testedEditKind(call: ToolDisplayMessage): TestedItemKind | undefined {
	const tool = call.toolName ?? ''
	if (Object.hasOwn(EDIT_TOOLS, tool)) return EDIT_TOOLS[tool]
	if (SCRIPT_EDIT_TOOLS.has(tool)) return 'script'
	if (tool === 'write_resource' && callParams(call).resource_type === 'ai_agent') return 'agent'
	return undefined
}

// An edit or read of the item between two of its runs: what the model changes before running
// it again. Its diff folds with it, since the next run is what says whether the change worked.
function touchesTestedItem(message: DisplayMessage, item: TestedItem): boolean {
	if (message.role !== 'tool' || !message.toolName) return false
	if (standsAlone(message) || hasContentCard(message)) return false
	const tool = message.toolName
	const type = callParams(message).type
	const kind: TestedItemKind | undefined =
		testedEditKind(message) ??
		ITEM_READ_TOOLS[tool] ??
		(tool === 'read_workspace_item' && (type === 'script' || type === 'flow' || type === 'app')
			? type
			: tool === 'read_workspace_item' && message.readsAgent
				? 'agent'
				: undefined)
	if (!kind) return false
	const path = callPath(message, PATHLESS_FLOW_MODE_TOOLS.has(tool) || tool === 'edit_code')
	return sameTestedItem(item, { kind, path })
}

// An editor holds one item, so two pathless calls act on the same one whatever their kind:
// the script editor's lint check is filed under the flow tools.
// A call whose path has not arrived yet stays with the group, as in an edit group.
function sameTestedItem(item: TestedItem, other: TestedItem): boolean {
	if (item.path === '' && (other.path === '' || other.path === undefined)) return true
	return other.kind === item.kind && (other.path === undefined || other.path === item.path)
}

function testGroupAt(
	messages: DisplayMessage[],
	start: number,
	tools: GroupingTools
): ToolGroup | undefined {
	const item = testRunOf(messages[start], tools)
	if (!item || item.path === undefined) return undefined
	let end = runEnd(messages, start, (m) => {
		const run = testRunOf(m, tools)
		return run ? sameTestedItem(item, run) : touchesTestedItem(m, item)
	})
	// Edits after the last run are not yet tested, so they are left to group as edits.
	while (end > start && !testRunOf(messages[end], tools)) end--
	const runs = messages.slice(start, end + 1).filter((m) => testRunOf(m, tools)).length
	if (runs < 2) return undefined
	const group = toolGroup(messages, start, end, 'test', item.path)
	return group && { ...group, tested: { kind: item.kind, runs } }
}

// An edit group wins over an explore group: its reads belong to the edits they prepare, so an
// explore run also stops before a read that starts one. Both win over a waiting row, which
// stops before either: its header says less than `Edit f/a/flow · 2 changes`. A test group
// starts on a run, which no other group takes, so it never competes for a start.
function kindGroupAt(
	messages: DisplayMessage[],
	start: number,
	tools: GroupingTools
): ToolGroup | undefined {
	const explores = (m: DisplayMessage, j: number) =>
		isReadCall(m, tools) && !editGroupAt(messages, j)
	const drafts = (m: DisplayMessage) => isDraftWrite(m, tools)
	const draftGroup = drafts(messages[start])
		? toolGroup(messages, start, runEnd(messages, start, drafts), 'draft', '')
		: undefined
	return (
		testGroupAt(messages, start, tools) ??
		editGroupAt(messages, start) ??
		(draftGroup && { ...draftGroup, draftNouns: tools.draftWrites }) ??
		(isReadCall(messages[start], tools)
			? toolGroup(messages, start, runEnd(messages, start, explores), 'explore', '')
			: undefined)
	)
}

export function groupToolRuns(messages: DisplayMessage[], tools: GroupingTools): ChatItem[] {
	const items: ChatItem[] = []
	let i = 0
	while (i < messages.length) {
		const group =
			kindGroupAt(messages, i, tools) ?? waitingGroupAt(messages, i, items.at(-1), tools)
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

function sameChatItem(a: ChatItem, b: ChatItem): boolean {
	if (a.kind === 'message') {
		return b.kind === 'message' && a.message === b.message && a.index === b.index
	}
	return (
		b.kind === 'group' &&
		a.groupKind === b.groupKind &&
		a.key === b.key &&
		a.target === b.target &&
		a.draftNouns === b.draftNouns &&
		a.tested?.kind === b.tested?.kind &&
		a.tested?.runs === b.tested?.runs &&
		a.entries.length === b.entries.length &&
		a.entries.every((e, i) => e.message === b.entries[i].message && e.index === b.entries[i].index)
	)
}

/** `groupToolRuns` for a transcript that is re-sent whole on every streamed chunk and on every
 * tool status change: a row whose messages did not change keeps its previous item object. A
 * fresh item per row re-renders the whole history each time (every answer re-parsing its
 * markdown, every tool row its result), which freezes the tab on long sessions.
 *
 * A changed message is told apart by identity alone, so a row must be replaced, never mutated
 * in place, for its grouping to update. */
export function createChatItemGrouper(): (
	messages: DisplayMessage[],
	tools: GroupingTools
) => ChatItem[] {
	let settled: DisplayMessage[] = []
	let settledItems: ChatItem[] = []
	let settledTools: GroupingTools | undefined
	return (messages, tools) => {
		// The live answer only ever comes last, and it can neither join nor end a group
		// differently than the end of the transcript does, so it is grouped on its own and the
		// rows before it are not regrouped while it streams.
		const last = messages.at(-1)
		const live = last?.role === 'assistant' && last.streaming ? last : undefined
		const n = live ? messages.length - 1 : messages.length
		if (
			tools !== settledTools ||
			n !== settled.length ||
			settled.some((m, i) => m !== messages[i])
		) {
			const previous = new Map(settledItems.map((item) => [chatItemKey(item), item]))
			settled = messages.slice(0, n)
			settledTools = tools
			settledItems = groupToolRuns(settled, tools).map((item) => {
				const before = previous.get(chatItemKey(item))
				return before && sameChatItem(before, item) ? before : item
			})
		}
		return live ? [...settledItems, { kind: 'message', message: live, index: n }] : settledItems
	}
}

// The server's full resource path: two servers can share a last segment (u/alice/github,
// f/team/github), so only the display shortens it.
function mcpServerPath(call: ToolDisplayMessage): string | undefined {
	if (call.toolName !== 'call_mcp_read_tool') return undefined
	const server = call.mcpServer?.path ?? call.parameters?.server
	return typeof server === 'string' ? server : undefined
}

function callName(call: ToolDisplayMessage): string {
	if (call.readsAgent) return 'read agent'
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
	if (name === 'read agent') return `read ${n} agents`
	return COUNTED_VERBS.has(verb) && rest.length > 0
		? `${verb} ${n} ${rest.join(' ')}`
		: `${name} ${n} times`
}

/** `queued`: every call is still waiting its turn. Every call in a turn is queued before the
 * first one runs, so this state shows on each edit run; like a single row's queued label, it
 * says what will happen rather than what did. */
export type GroupState = 'queued' | 'running' | 'settled'

// Past this many kinds of call the header names the rest only as a count, so a long read run
// keeps its first kinds legible instead of being cut off by the row's ellipsis.
const LISTED_KINDS = 3

// What a test group's header names when its runs took no path: the item open in the editor,
// or for test_run_code the code it was sent.
const OPEN_ITEM: Record<TestedItemKind, string> = {
	flow: 'the flow',
	app: 'the app',
	script: 'the script',
	agent: 'the agent',
	code: 'code'
}

/** The group's header, e.g. `Edited` + `f/a/flow · 3 changes`,
 * `search workspace 2 times, read 2 workspace item`, `github` + `list issues, get issue 2 times`.
 * `more` holds the kinds left out of `label`. */
export function groupHeader(
	group: ToolGroup,
	state: GroupState
): { prefix: string; label: string; more: string[] } {
	const calls = group.entries
		.map((e) => e.message)
		.filter((m): m is ToolDisplayMessage => m.role === 'tool')
	if (group.groupKind === 'waiting') {
		// A queued row's content is its imperative label, e.g. `Execute SQL on "main"`.
		const names = [...new Set(calls.map((m) => m.content))]
		return {
			prefix: `${plural(calls.length, 'more call')} waiting:`,
			label: names.slice(0, LISTED_KINDS).join(', '),
			more: names.slice(LISTED_KINDS)
		}
	}
	if (group.groupKind === 'test' && group.tested) {
		const { kind, runs } = group.tested
		const edits = calls.filter((m) => testedEditKind(m) && !callFailed(m)).length
		return {
			prefix: { queued: 'Test', running: 'Testing', settled: 'Tested' }[state],
			label: [
				group.target || OPEN_ITEM[kind],
				plural(runs, 'run'),
				...(edits ? [plural(edits, 'change')] : [])
			].join(' · '),
			more: []
		}
	}
	if (group.groupKind === 'edit') {
		const edits = calls.filter(
			(m) => Object.hasOwn(EDIT_TOOLS, m.toolName ?? '') && !callFailed(m)
		).length
		return {
			prefix: { queued: 'Edit', running: 'Editing', settled: 'Edited' }[state],
			label: `${group.target || 'the flow'} · ${plural(edits, 'change')}`,
			more: []
		}
	}
	if (group.groupKind === 'draft') {
		const drafted = new Map<string, number>()
		for (const call of calls) {
			if (callFailed(call)) continue
			const kind = group.draftNouns?.get(call.toolName!) ?? 'draft'
			drafted.set(kind, (drafted.get(kind) ?? 0) + 1)
		}
		const kinds = [...drafted].map(([kind, n]) => plural(n, kind))
		return {
			prefix: { queued: 'Draft', running: 'Drafting', settled: 'Drafted' }[state],
			label: kinds.length ? kinds.join(', ') : plural(0, 'draft'),
			more: []
		}
	}
	const counts = new Map<string, number>()
	for (const call of calls) counts.set(callName(call), (counts.get(callName(call)) ?? 0) + 1)
	const kinds = [...counts].map(([name, n]) => repeated(name, n))
	const list = kinds.slice(0, LISTED_KINDS).join(', ')
	const more = kinds.slice(LISTED_KINDS)
	// A tool search spans every server and usually comes right before the calls it found, so it
	// does not stop the group from reading as one server's.
	const servers = new Set(
		calls.filter((call) => call.toolName !== 'search_mcp_tools').map(mcpServerPath)
	)
	const [server] = servers
	if (servers.size === 1 && server) return { prefix: server.split('/').at(-1)!, label: list, more }
	return { prefix: '', label: list.charAt(0).toUpperCase() + list.slice(1), more }
}
