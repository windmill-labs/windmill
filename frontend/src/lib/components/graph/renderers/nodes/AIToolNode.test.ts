import { describe, it, expect, vi } from 'vitest'

// Mock the component wrapper so importing the .svelte module doesn't pull in the
// full render-time dependency graph.
vi.mock('./NodeWrapper.svelte', () => ({ default: {} }))

import { computeAIToolNodes } from './AIToolNode.svelte'

const eventHandlers = {} as any

function aiAgentNode(id: string, tools: any[]): any {
	return {
		id,
		type: 'module',
		position: { x: 0, y: 0 },
		data: { module: { id, value: { type: 'aiagent', tools } } }
	}
}

describe('computeAIToolNodes', () => {
	it('does not flag duplicate names when the same tool is called multiple times at runtime', () => {
		// One statically-defined tool that the agent called twice. The runtime
		// agent_actions therefore carry the same function_name twice — this is
		// expected and must not surface as a `nameError` (which renders as Failure).
		const node = aiAgentNode('agent', [
			{ id: 'tool_a', summary: 'my_tool', value: { tool_type: 'flowmodule', type: 'script' } }
		])
		const flowModuleStates = {
			agent: {
				type: 'Success',
				agent_actions: [
					{ type: 'tool_call', function_name: 'my_tool', module_id: 'tool_a', job_id: 'j1' },
					{ type: 'tool_call', function_name: 'my_tool', module_id: 'tool_a', job_id: 'j2' }
				]
			}
		} as any

		const { toolNodes } = computeAIToolNodes([node], eventHandlers, false, flowModuleStates)

		expect(toolNodes.length).toBe(2)
		for (const n of toolNodes) {
			expect((n.data as any).nameError).toBeUndefined()
		}
	})

	it('does not flag any node in a mixed run where one tool repeats (reporter scenario)', () => {
		// Repo-intel run: query_stored called 3x plus two single calls, all succeeded.
		// Before the fix the three query_stored nodes rendered red (Failure) purely
		// from the duplicate-name check, while the unique tools stayed green.
		const node = aiAgentNode('chat', [
			{ id: 'q', summary: 'query_stored', value: { tool_type: 'flowmodule', type: 'script' } },
			{ id: 'h', summary: 'hybrid_search', value: { tool_type: 'flowmodule', type: 'script' } },
			{
				id: 't',
				summary: 'trace_outbound_calls',
				value: { tool_type: 'flowmodule', type: 'script' }
			}
		])
		const call = (name: string, job: string) => ({
			type: 'tool_call',
			function_name: name,
			module_id: name[0],
			job_id: job
		})
		const flowModuleStates = {
			chat: {
				type: 'Success',
				agent_actions: [
					call('query_stored', 'j1'),
					call('query_stored', 'j2'),
					call('query_stored', 'j3'),
					call('hybrid_search', 'j4'),
					call('trace_outbound_calls', 'j5')
				]
			}
		} as any

		const { toolNodes } = computeAIToolNodes([node], eventHandlers, false, flowModuleStates)

		expect(toolNodes.length).toBe(5)
		for (const n of toolNodes) {
			expect((n.data as any).nameError).toBeUndefined()
		}
	})

	it('still flags genuinely duplicate tool names in the editor (static tool set)', () => {
		const node = aiAgentNode('agent2', [
			{ id: 't1', summary: 'dup', value: { tool_type: 'flowmodule', type: 'script' } },
			{ id: 't2', summary: 'dup', value: { tool_type: 'flowmodule', type: 'script' } }
		])

		const { toolNodes } = computeAIToolNodes([node], eventHandlers, true, undefined)

		const toolCallNodes = toolNodes.filter((n) => n.type === 'aiTool')
		expect(toolCallNodes.length).toBe(2)
		for (const n of toolCallNodes) {
			expect((n.data as any).nameError).toBe('Duplicate tool name')
		}
	})
})
