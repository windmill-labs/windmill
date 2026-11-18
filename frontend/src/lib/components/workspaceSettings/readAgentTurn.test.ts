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
