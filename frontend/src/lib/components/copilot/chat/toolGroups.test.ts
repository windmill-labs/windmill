import { describe, expect, it } from 'vitest'
import type { DisplayMessage } from './shared'
import { groupToolRuns } from './toolGroups'

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
		item.kind === 'group' ? item.entries.map((e) => e.index) : item.index
	)
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
		).toEqual([0, [1, 2, 3, 4], 5, 6])
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
		).toEqual([[0, 1], 2, 3, 4])
	})

	it('never folds a call waiting for confirmation, or a run of reads alone', () => {
		const flow = { path: 'f/a/flow' }
		expect(
			shape([
				tool('patch_flow_json', flow),
				tool('patch_flow_json', flow, { needsConfirmation: true, isLoading: true }),
				tool('read_flow_module_code', flow),
				tool('read_flow_module_code', flow)
			])
		).toEqual([0, 1, 2, 3])
	})
})
