import { describe, expect, it } from 'vitest'

import { hostBoundToolInputs, summarizeAgentBrain } from './agentResourceUtils'

describe('summarizeAgentBrain', () => {
	it('returns only set fields, in brain-key order, formatted', () => {
		const rows = summarizeAgentBrain({
			provider: { kind: 'openai', model: 'gpt-4o', resource: '$res:f/x/openai' } as any,
			system_prompt: 'You are helpful',
			temperature: 0.7,
			streaming: true,
			max_iterations: 10
		})
		expect(rows).toEqual([
			{ label: 'Model', value: 'openai · gpt-4o' },
			{ label: 'System prompt', value: 'You are helpful' },
			{ label: 'Streaming', value: 'on' },
			{ label: 'Temperature', value: '0.7' },
			{ label: 'Max iterations', value: '10' }
		])
	})

	it('skips empty/undefined fields', () => {
		expect(summarizeAgentBrain({ system_prompt: '', provider: undefined as any })).toEqual([])
		expect(summarizeAgentBrain(undefined)).toEqual([])
	})

	it('summarizes structured fields compactly', () => {
		const rows = summarizeAgentBrain({
			memory: { type: 'auto', context_length: 20 } as any,
			output_schema: { type: 'object' } as any
		})
		expect(rows).toEqual([
			{ label: 'Memory', value: 'auto' },
			{ label: 'Output schema', value: 'configured' }
		])
	})
})

describe('hostBoundToolInputs', () => {
	it('surfaces only javascript-wired inputs of FlowModule tools', () => {
		const tools = [
			{
				id: 'search',
				summary: 'search_db',
				value: {
					tool_type: 'flowmodule',
					type: 'script',
					input_transforms: {
						tenant: { type: 'javascript', expr: 'flow_input.tenant' },
						query: { type: 'ai' },
						limit: { type: 'static', value: 10 }
					}
				}
			},
			// MCP tool: no input_transforms, must be skipped entirely.
			{ id: 'mcp', value: { tool_type: 'mcp', resource_path: 'u/x/mcp' } }
		]
		expect(hostBoundToolInputs(tools as any)).toEqual([
			{
				toolId: 'search',
				label: 'search_db',
				keys: [{ key: 'tenant', resourceExpr: 'flow_input.tenant' }]
			}
		])
	})

	it('omits tools with no host-bound inputs and handles empty input', () => {
		const tools = [{ id: 't', summary: 's', value: { input_transforms: { a: { type: 'ai' } } } }]
		expect(hostBoundToolInputs(tools as any)).toEqual([])
		expect(hostBoundToolInputs(undefined)).toEqual([])
	})
})
