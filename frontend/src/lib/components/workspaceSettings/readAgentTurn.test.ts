import { describe, expect, it } from 'vitest'
import { readAgentTurn } from './deployToHubSession.svelte'

const call = (id: string, name: string, args: unknown) => ({
	id,
	type: 'function',
	function: { name, arguments: JSON.stringify(args) }
})

describe('readAgentTurn', () => {
	it('reads the tools this turn called and its answer', () => {
		const turn = readAgentTurn({
			output: 'Found it.',
			messages: [
				{ role: 'user', content: 'Where is my order?' },
				{ role: 'assistant', tool_calls: [call('a', 'get_order', { id: 1 })] },
				{ role: 'tool', tool_call_id: 'a', content: '{"status":"shipped"}' },
				{ role: 'assistant', content: 'Found it.' }
			]
		})
		expect(turn.answer).toBe('Found it.')
		expect(turn.tool_calls).toEqual([
			{ name: 'get_order', args: { id: 1 }, result: '{"status":"shipped"}' }
		])
	})

	it('leaves out the earlier turns a run with memory returns', () => {
		const turn = readAgentTurn({
			output: 'Refunded.',
			messages: [
				{ role: 'user', content: 'Where is my order?' },
				{ role: 'assistant', tool_calls: [call('a', 'get_order', { id: 1 })] },
				{ role: 'tool', tool_call_id: 'a', content: 'shipped' },
				{ role: 'assistant', content: 'Found it.' },
				{ role: 'user', content: 'Refund it.' },
				{ role: 'assistant', tool_calls: [call('b', 'refund_order', { id: 1 })] },
				{ role: 'tool', tool_call_id: 'b', content: 'ok' },
				{ role: 'assistant', content: 'Refunded.' }
			]
		})
		expect(turn.tool_calls.map((c) => c.name)).toEqual(['refund_order'])
	})
})

describe('readAgentTurn with background tools', () => {
	it('keeps a background call whose result came back as a tagged user message', () => {
		const turn = readAgentTurn({
			output: 'Rendered.',
			messages: [
				{ role: 'user', content: 'Render it.' },
				{ role: 'assistant', tool_calls: [call('a', 'render', {})] },
				{ role: 'tool', tool_call_id: 'a', content: 'Started in the background.' },
				{
					role: 'user',
					content: 'Background tool `render` (call a, job j) finished. Result:\nok',
					agent_action: { type: 'tool_call', job_id: 'j', function_name: 'render', module_id: 'm' }
				},
				{ role: 'assistant', content: 'Rendered.' }
			]
		})
		expect(turn.tool_calls.map((c) => c.name)).toEqual(['render'])
	})
})

describe('readAgentTurn with calls that are not tools of the agent', () => {
	it('does not show the structured answer as a tool call', () => {
		const turn = readAgentTurn({
			output: { city: 'Paris' },
			messages: [
				{ role: 'user', content: 'Where?' },
				{ role: 'assistant', tool_calls: [call('s', 'structured_output', { city: 'Paris' })] },
				{ role: 'tool', tool_call_id: 's', content: 'Successfully ran structured_output tool' }
			]
		})
		expect(turn.tool_calls).toEqual([])
		expect(JSON.parse(turn.answer)).toEqual({ city: 'Paris' })
	})

	it('shows a web search, in the order it happened', () => {
		const turn = readAgentTurn({
			output: 'Found.',
			messages: [
				{ role: 'user', content: 'Look it up, then check the order.' },
				{
					role: 'tool',
					content: 'Used websearch tool successfully',
					agent_action: { type: 'web_search' }
				},
				{ role: 'assistant', tool_calls: [call('a', 'get_order', { id: 1 })] },
				{ role: 'tool', tool_call_id: 'a', content: 'shipped' },
				{ role: 'assistant', content: 'Found.' }
			]
		})
		expect(turn.tool_calls.map((c) => c.name)).toEqual(['web_search', 'get_order'])
	})
})
