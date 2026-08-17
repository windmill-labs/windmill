import { describe, it, expect, vi } from 'vitest'

// Mock the component wrapper so importing the .svelte module doesn't pull in the
// full render-time dependency graph.
vi.mock('./NodeWrapper.svelte', () => ({ default: {} }))

import { agentActionMatchesTool, computeAIToolNodes } from './AIToolNode.svelte'

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

	it('picks up a tool call that arrives without moving any node', () => {
		// Run status lives outside node data, so the memo has to key on the agent's actions
		// itself. Two calls occupy one row, i.e. identical positions, so a memo keyed only on
		// the nodes would serve the stale single-tool result forever.
		const node = aiAgentNode('agent', [
			{ id: 'tool_a', summary: 'my_tool', value: { tool_type: 'flowmodule', type: 'script' } }
		])
		const stateWith = (n: number) =>
			({
				agent: {
					type: 'InProgress',
					agent_actions: Array.from({ length: n }, (_, i) => ({
						type: 'tool_call',
						function_name: 'my_tool',
						module_id: 'tool_a',
						job_id: `j${i}`
					}))
				}
			}) as any

		expect(computeAIToolNodes([node], eventHandlers, false, stateWith(1)).toolNodes.length).toBe(1)
		expect(computeAIToolNodes([node], eventHandlers, false, stateWith(2)).toolNodes.length).toBe(2)
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

	// The editor keeps one node per declared tool, so each kind of action has to find its way back
	// to the right one. Only the flowmodule join is reachable in a local run — web search needs a
	// provider that emits it server-side, MCP needs a live server — so pin all four here.
	describe('agentActionMatchesTool', () => {
		const flowTool = { moduleId: 'tool_a', type: 'script' }
		const searchTool = { moduleId: 'tool_s', type: 'websearch' }
		const mcpTool = { moduleId: 'tool_m', type: 'mcp', resourcePath: 'u/admin/mcp' }

		it('binds a flow module call to the tool it names', () => {
			const call = { type: 'tool_call', module_id: 'tool_a' } as any
			expect(agentActionMatchesTool(call, flowTool)).toBe(true)
			expect(agentActionMatchesTool(call, { moduleId: 'tool_b', type: 'script' })).toBe(false)
			expect(agentActionMatchesTool(call, searchTool)).toBe(false)
		})

		it('binds a web search to the declared web search tool', () => {
			const search = { type: 'web_search' } as any
			expect(agentActionMatchesTool(search, searchTool)).toBe(true)
			expect(agentActionMatchesTool(search, flowTool)).toBe(false)
		})

		it('binds an MCP call by server, not by function name', () => {
			// One MCP node stands for a whole server and many function names.
			const call = {
				type: 'mcp_tool_call',
				function_name: 'anything',
				resource_path: 'u/admin/mcp'
			} as any
			expect(agentActionMatchesTool(call, mcpTool)).toBe(true)
			expect(
				agentActionMatchesTool(call, {
					moduleId: 'tool_m2',
					type: 'mcp',
					resourcePath: 'u/admin/other'
				})
			).toBe(false)
		})

		it("binds the agent's own replies to no tool at all", () => {
			const message = { type: 'message' } as any
			for (const tool of [flowTool, searchTool, mcpTool]) {
				expect(agentActionMatchesTool(message, tool)).toBe(false)
			}
		})
	})
})
