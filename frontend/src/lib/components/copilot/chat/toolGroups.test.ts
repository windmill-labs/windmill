import { describe, expect, it } from 'vitest'
import type { DisplayMessage } from './shared'
import { groupHeader, groupToolRuns, type ToolGroup } from './toolGroups'

let nextId = 0
function tool(
	toolName: string,
	parameters: Record<string, unknown> = {},
	extra = {}
): DisplayMessage {
	return {
		role: 'tool',
		tool_call_id: `call_${nextId++}`,
		content: toolName,
		toolName,
		parameters,
		...extra
	}
}
function assistant(content: string, extra = {}): DisplayMessage {
	return { role: 'assistant', content, ...extra } as DisplayMessage
}

function shape(messages: DisplayMessage[]) {
	return groupToolRuns(messages).map((item) =>
		item.kind === 'group' ? { [item.groupKind]: item.entries.map((e) => e.index) } : item.index
	)
}

function header(messages: DisplayMessage[]) {
	const group = groupToolRuns(messages).find((item) => item.kind === 'group') as ToolGroup
	const { prefix, label } = groupHeader(group, false)
	return prefix ? `${prefix} ${label}` : label
}

describe('groupToolRuns', () => {
	it('folds edits and reads of one flow, with thinking in between, into one group', () => {
		const flow = { path: 'f/a/flow' }
		expect(
			shape([
				assistant('Let me edit the flow.'),
				tool('read_flow_module_code', flow),
				tool('patch_flow_json', flow),
				assistant('', { reasoning: 'next step' }),
				tool('set_flow_module_code', flow),
				assistant('', { reasoning: 'done' }),
				assistant('All steps updated.')
			])
		).toEqual([0, { edit: [1, 2, 3, 4] }, 5, 6])
	})

	it('splits on a different flow, on visible text, and leaves a lone edit ungrouped', () => {
		const a = { path: 'f/a/flow' }
		const b = { path: 'f/b/flow' }
		expect(
			shape([
				tool('patch_flow_json', a),
				tool('patch_flow_json', a),
				tool('patch_flow_json', b),
				assistant('Now the other one.'),
				tool('patch_flow_json', b)
			])
		).toEqual([{ edit: [0, 1] }, 2, 3, 4])
	})

	it('never folds a call waiting for confirmation, and folds reads without an edit as exploring', () => {
		const flow = { path: 'f/a/flow' }
		expect(
			shape([
				tool('patch_flow_json', flow),
				tool('patch_flow_json', flow, { needsConfirmation: true, isLoading: true }),
				tool('read_flow_module_code', flow),
				tool('read_flow_module_code', flow)
			])
		).toEqual([0, 1, { explore: [2, 3] }])
		expect(
			shape([
				tool('patch_flow_json', flow),
				tool('patch_flow_json', flow, { declinedByUser: true, error: 'Cancelled by user' }),
				tool('patch_flow_json', flow)
			])
		).toEqual([0, 1, 2])
	})

	it('leaves a flow read that prepares an edit to the edit group, not the lookups before it', () => {
		const flow = { type: 'flow', path: 'f/a/flow' }
		expect(
			shape([
				tool('search_workspace'),
				tool('list_workspace_items'),
				tool('read_workspace_item', flow),
				tool('patch_flow_json', flow)
			])
		).toEqual([{ explore: [0, 1] }, { edit: [2, 3] }])
	})

	it('folds consecutive lookups of any tool, split by a write or a row with its own card', () => {
		expect(
			shape([
				tool('search_workspace'),
				assistant('', { reasoning: 'look closer' }),
				tool('read_workspace_item', { type: 'script', path: 'f/a/s' }),
				tool('get_run', {}, { inspectedRun: { jobId: 'x' } }),
				tool('list_runs'),
				tool('search_docs'),
				tool('write_script', { path: 'f/a/s' }),
				tool('search_workspace')
			])
		).toEqual([{ explore: [0, 1, 2] }, 3, { explore: [4, 5] }, 6, 7])
	})

	it('names what a group did', () => {
		const flow = { path: 'f/a/flow' }
		expect(
			header([
				tool('read_flow_module_code', flow),
				tool('patch_flow_json', flow),
				tool('patch_flow_json', flow)
			])
		).toBe('Edited f/a/flow · 2 changes')
		// A save rejected as a conflict reports it in `result`, not `error`: not a change.
		expect(
			header([
				tool('patch_flow_json', flow),
				tool('patch_flow_json', flow, { result: 'Conflict' }),
				tool('set_flow_module_code', flow, { result: 'Save failed' })
			])
		).toBe('Edited f/a/flow · 1 change')
		expect(
			header([
				tool('search_workspace'),
				tool('read_workspace_item'),
				tool('search_workspace'),
				tool('read_workspace_item'),
				tool('list_runs')
			])
		).toBe('Search workspace 2 times, read 2 workspace item, list runs')
		const mcp = (t: string) => tool('call_mcp_read_tool', { server: 'u/admin/github', tool: t })
		expect(header([mcp('list_issues'), mcp('get_issue'), mcp('get_issue')])).toBe(
			'github list issues, get issue 2 times'
		)
	})
})
