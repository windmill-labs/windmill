import type { DisplayMessage, ToolDisplayMessage } from './shared'

// Tools that fold into an edit group, with the flow each call targets. Flow-mode tools edit
// the flow open in the editor, so they carry no path and all share the '' target. A tool
// missing from here always renders as its own row: an unlisted write never gets hidden.
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

export type ChatItem =
	| { kind: 'message'; message: DisplayMessage; index: number }
	| {
			kind: 'group'
			/** First call's id, so the group keeps its identity (and expand state) as it grows. */
			key: string
			/** Flow path, or '' for the flow open in the editor. */
			target: string
			entries: { message: DisplayMessage; index: number }[]
	  }

type Membership = { target: string; edit: boolean }

function membership(message: DisplayMessage): Membership | undefined {
	if (message.role !== 'tool' || !message.toolName) return undefined
	// A row waiting on the user, or refused by plan mode, is a decision the user must see.
	if (message.needsConfirmation || message.blockedByPlanMode) return undefined
	const params = message.parameters ?? {}
	const path = typeof params.path === 'string' ? params.path : ''
	if (FLOW_EDIT_TOOLS.has(message.toolName)) return { target: path, edit: true }
	if (FLOW_READ_TOOLS.has(message.toolName)) return { target: path, edit: false }
	if (message.toolName === 'read_workspace_item' && params.type === 'flow' && path) {
		return { target: path, edit: false }
	}
	return undefined
}

// Thinking between two calls stays inside the group; visible text ends it. The live
// streaming message is never absorbed, or the reasoning in progress would be hidden.
function isSilentAssistant(message: DisplayMessage): boolean {
	return message.role === 'assistant' && !message.streaming && message.content.trim() === ''
}

export function groupToolRuns(messages: DisplayMessage[]): ChatItem[] {
	const items: ChatItem[] = []
	let i = 0
	while (i < messages.length) {
		const first = membership(messages[i])
		if (!first) {
			items.push({ kind: 'message', message: messages[i], index: i })
			i++
			continue
		}
		// Extend over same-target calls and silent assistant messages; `end` stops at the
		// last call so trailing silent messages stay outside.
		let end = i
		let j = i + 1
		while (j < messages.length) {
			const m = membership(messages[j])
			if (m) {
				if (m.target !== first.target) break
				end = j
			} else if (!isSilentAssistant(messages[j])) {
				break
			}
			j++
		}
		const run = messages.slice(i, end + 1)
		const calls = run.filter((m) => m.role === 'tool')
		const hasEdit = calls.some((m) => membership(m)?.edit)
		if (calls.length >= 2 && hasEdit) {
			items.push({
				kind: 'group',
				key: (messages[i] as ToolDisplayMessage).tool_call_id,
				target: first.target,
				entries: run.map((message, k) => ({ message, index: i + k }))
			})
			i = end + 1
		} else {
			items.push({ kind: 'message', message: messages[i], index: i })
			i++
		}
	}
	return items
}

export function isFlowEditCall(message: ToolDisplayMessage): boolean {
	return FLOW_EDIT_TOOLS.has(message.toolName ?? '')
}
