import { describe, expect, it } from 'vitest'
import type { DisplayMessage } from './shared'
import { groupHeader, groupToolRuns, type GroupState, type ToolGroup } from './toolGroups'

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

function header(messages: DisplayMessage[], state: GroupState = 'settled') {
	const group = groupToolRuns(messages).find((item) => item.kind === 'group') as ToolGroup
	const { prefix, label } = groupHeader(group, state)
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
		const sources = JSON.stringify({ sources: [{ url: 'https://example.com', title: 'x' }] })
		expect(
			shape([
				tool('search_docs'),
				tool('call_mcp_read_tool', { server: 'u/a/s', tool: 'search' }, { result: sources }),
				tool('patch_flow_json', flow, { heldForFolderInstructions: true }),
				tool('patch_flow_json', flow)
			])
		).toEqual([0, 1, 2, 3])
	})

	it('folds the calls queued behind another call into one waiting row', () => {
		const sql = (extra = {}) =>
			tool('exec_datatable_sql', { datatable_name: 'main', sql: 'select 1' }, extra)
		const queued = { isQueued: true, content: 'Execute SQL on "main"' }
		const batch = [
			sql({ needsConfirmation: true, isLoading: true }),
			sql(queued),
			tool('search_workspace', {}, { ...queued, content: 'Search workspace' })
		]
		expect(shape(batch)).toEqual([0, { waiting: [1, 2] }])
		expect(header(batch)).toBe('2 more calls waiting: Execute SQL on "main", Search workspace')
		// The call next in line shows as itself, before anything runs and between two calls.
		expect(shape([assistant(''), sql(queued), sql(queued)])).toEqual([0, 1, { waiting: [2] }])
		expect(shape([sql(), sql(queued), sql(queued)])).toEqual([0, 1, { waiting: [2] }])
		// Queued edits of one flow keep their edit group.
		const flow = { path: 'f/a/flow' }
		expect(
			shape([
				tool('search_workspace', {}, { isLoading: true }),
				tool('patch_flow_json', flow, queued),
				tool('patch_flow_json', flow, queued)
			])
		).toEqual([0, { edit: [1, 2] }])
		// A waiting row stops before queued calls that form their own group.
		expect(
			shape([
				sql({ needsConfirmation: true, isLoading: true }),
				sql(queued),
				tool('patch_flow_json', flow, queued),
				tool('patch_flow_json', flow, queued)
			])
		).toEqual([0, { waiting: [1] }, { edit: [2, 3] }])
	})

	it('keeps an edit whose arguments are still streaming in the group it follows', () => {
		const flow = { path: 'f/a/flow' }
		expect(
			shape([
				tool('patch_flow_json', flow),
				tool('patch_flow_json', '{"path":"f/a/flow","old_str' as never, {
					isStreamingArguments: true
				}),
				tool('set_flow_module_code', '{"pa' as never, { isStreamingArguments: true })
			])
		).toEqual([{ edit: [0, 1, 2] }])
		expect(
			shape([
				tool('patch_flow_json', flow),
				tool('patch_flow_json', '{"path":"f/b/flow","old_str' as never, {
					isStreamingArguments: true
				})
			])
		).toEqual([0, 1])
	})

	it('lets a call with no arguments yet join a group but not start one', () => {
		const queued = (toolName: string) =>
			tool(toolName, {}, { parameters: undefined, isQueued: true })
		expect(shape([queued('delete_app_file'), queued('delete_app_runnable')])).toEqual([
			0,
			{ waiting: [1] }
		])
		expect(shape([tool('patch_app_file', { path: 'f/a/x' }), queued('delete_app_file')])).toEqual([
			{ edit: [0, 1] }
		])
		// Arguments without a path are a call the tool will reject, not an edit of the open
		// flow; only flow-mode tools, which never take a path, share the open flow.
		expect(
			shape([tool('write_app_file', { file_path: '/a' }), tool('patch_app_file', {})])
		).toEqual([0, 1])
		expect(
			shape([
				tool('set_module_code', { moduleId: 'a' }),
				tool('patch_flow_json', { old_string: 'a', new_string: 'b' }),
				tool('set_module_code', { moduleId: 'b' })
			])
		).toEqual([{ edit: [0, 1, 2] }])
		// A global-only flow tool always takes a path, so without one its item is unknown.
		expect(shape([tool('write_flow', { summary: 'x' }), tool('set_flow_module_code', {})])).toEqual(
			[0, 1]
		)
	})

	it('folds edits and reads of one app, apart from a flow at the same path', () => {
		const app = { path: 'f/a/x', file_path: '/src/App.tsx' }
		expect(
			shape([
				tool('read_app_file', app),
				tool('patch_app_file', app),
				tool('write_app_runnable', { path: 'f/a/x', key: 'fetch' }),
				tool('patch_flow_json', { path: 'f/a/x' }),
				tool('patch_flow_json', { path: 'f/a/x' })
			])
		).toEqual([{ edit: [0, 1, 2] }, { edit: [3, 4] }])
	})

	it('folds app edits that carry a file diff, but not a script edit with one', () => {
		const app = { path: 'f/a/x' }
		const diff = { codeDiff: { before: 'a', after: 'b' } }
		expect(
			shape([
				tool('init_app', app),
				tool('write_app_file', { ...app, file_path: '/App.tsx' }, diff),
				tool('delete_app_runnable', { ...app, key: 'greet' }, diff),
				tool('write_script', { path: 'f/a/s' }, diff)
			])
		).toEqual([{ edit: [0, 1, 2] }, 3])
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
		// Before any call runs, the header says what will happen, not what did.
		expect(
			header(
				[
					tool('patch_flow_json', flow, { isQueued: true }),
					tool('patch_flow_json', flow, { isQueued: true })
				],
				'queued'
			)
		).toBe('Edit f/a/flow · 2 changes')
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
		expect(header([tool('search_mcp_tools'), mcp('list_issues'), mcp('get_issue')])).toBe(
			'github search mcp tools, list issues, get issue'
		)
		const other = tool('call_mcp_read_tool', { server: 'f/team/github', tool: 'get_issue' })
		expect(header([mcp('list_issues'), other])).toBe('List issues, get issue')
	})
})
