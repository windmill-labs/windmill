import { describe, it, expect } from 'vitest'
import { parseStreamDeltas } from './utils'

describe('parseStreamDeltas', () => {
	it('accumulates answer content from token_delta', () => {
		const stream = [
			JSON.stringify({ type: 'token_delta', content: 'Hello ' }),
			JSON.stringify({ type: 'token_delta', content: 'world' })
		].join('\n')
		const { content, reasoning, type } = parseStreamDeltas(stream)
		expect(content).toBe('Hello world')
		expect(reasoning).toBe('')
		expect(type).toBe('message')
	})

	it('accumulates reasoning separately from answer content', () => {
		const stream = [
			JSON.stringify({ type: 'reasoning_token_delta', content: 'let me ' }),
			JSON.stringify({ type: 'reasoning_token_delta', content: 'think' }),
			JSON.stringify({ type: 'token_delta', content: '42' })
		].join('\n')
		const { content, reasoning, type } = parseStreamDeltas(stream)
		expect(reasoning).toBe('let me think')
		expect(content).toBe('42')
		expect(type).toBe('message')
	})

	it('reports tool_result with success flag', () => {
		const stream = JSON.stringify({
			type: 'tool_result',
			function_name: 'lookup',
			success: true
		})
		const { type, content, success } = parseStreamDeltas(stream)
		expect(type).toBe('tool_result')
		expect(content).toBe('Used lookup tool')
		expect(success).toBe(true)
	})

	it('reports a failed tool_result', () => {
		const stream = JSON.stringify({
			type: 'tool_result',
			function_name: 'lookup',
			success: false
		})
		const { type, content, success } = parseStreamDeltas(stream)
		expect(type).toBe('tool_result')
		expect(content).toBe('Failed to use lookup tool')
		expect(success).toBe(false)
	})
})
