import { describe, expect, it } from 'vitest'
import { appendRevealed, applyStreamEvent, emptyTurnState, type TurnStep } from './turnTranscript'
import type { StreamEvent } from '$lib/components/chat/utils'

function start(): TurnStep {
	return { rows: [], state: emptyTurnState('conv') }
}

let n = 0
const nextId = () => `row-${++n}`

function apply(step: TurnStep, events: StreamEvent[]): TurnStep {
	return events.reduce((acc, event) => applyStreamEvent(acc, event, nextId), step)
}

describe('turn transcript', () => {
	it('keeps thinking that arrives in the same chunk as the tool call it led to', () => {
		// The regression: reasoning revealed and a tool call applied back to back, which is
		// how one SSE chunk delivers "thought about it, then called the tool".
		let step = appendRevealed(start(), 'reasoning', 'Checking the issue first.', nextId)
		step = apply(step, [{ kind: 'tool_call', callId: 'c1', name: 'mcp_linear_get_issue' }])

		expect(step.rows.map((r) => r.message_type)).toEqual(['assistant', 'tool'])
		expect(step.rows[0].reasoning).toBe('Checking the issue first.')
		expect(step.rows[0].streaming).toBe(false)
	})

	it('starts a new answer row after a tool call instead of extending the one before it', () => {
		let step = appendRevealed(start(), 'answer', 'Let me look. ', nextId)
		step = apply(step, [
			{ kind: 'tool_call', callId: 'c1', name: 'get_time' },
			{ kind: 'tool_result', callId: 'c1', name: 'get_time', result: '{}', success: true }
		])
		step = appendRevealed(step, 'answer', 'It is noon.', nextId)

		expect(step.rows.map((r) => r.content)).toEqual([
			'Let me look. ',
			'Used get_time tool',
			'It is noon.'
		])
	})

	it('lands a tool call and its result on one row', () => {
		const step = apply(start(), [
			{ kind: 'tool_call', callId: 'c1', name: 'get_time' },
			{ kind: 'tool_arguments', callId: 'c1', name: 'get_time', arguments: '{"tz":"UTC"}' },
			{ kind: 'tool_execution', callId: 'c1', name: 'get_time' },
			{ kind: 'tool_result', callId: 'c1', name: 'get_time', result: '{"now":1}', success: true }
		])

		const tools = step.rows.filter((r) => r.message_type === 'tool')
		expect(tools).toHaveLength(1)
		expect(tools[0].tool_arguments).toBe('{"tz":"UTC"}')
		expect(tools[0].tool_result).toBe('{"now":1}')
		expect(tools[0].loading).toBe(false)
	})

	it('marks a failed tool call on its row', () => {
		const step = apply(start(), [
			{ kind: 'tool_call', callId: 'c1', name: 'get_time' },
			{ kind: 'tool_result', callId: 'c1', name: 'get_time', result: 'boom', success: false }
		])

		const tool = step.rows.find((r) => r.message_type === 'tool')
		expect(tool?.success).toBe(false)
		expect(tool?.content).toBe('Failed to use get_time tool')
	})

	it('grows one answer row as text is revealed', () => {
		let step = appendRevealed(start(), 'answer', 'Hel', nextId)
		step = appendRevealed(step, 'answer', 'lo', nextId)

		expect(step.rows).toHaveLength(1)
		expect(step.rows[0].content).toBe('Hello')
		expect(step.rows[0].streaming).toBe(true)
	})
})
