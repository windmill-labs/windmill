import { describe, expect, it } from 'vitest'
import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'
import { hasValidToolCallArguments, sanitizeToolCallArguments } from './toolCallArguments'

describe('hasValidToolCallArguments', () => {
	it('accepts empty or valid JSON arguments', () => {
		expect(hasValidToolCallArguments(undefined)).toBe(true)
		expect(hasValidToolCallArguments('')).toBe(true)
		expect(hasValidToolCallArguments('{}')).toBe(true)
		expect(hasValidToolCallArguments('{"path": "u/admin/app", "content": "x"}')).toBe(true)
	})

	it('rejects arguments truncated mid-stream', () => {
		expect(hasValidToolCallArguments('{"path": "u/admin/app", "old_string": "setMess')).toBe(false)
		expect(hasValidToolCallArguments('{"path": "u/admin/app"')).toBe(false)
	})
})

describe('sanitizeToolCallArguments', () => {
	const poisoned: ChatCompletionMessageParam = {
		role: 'assistant',
		tool_calls: [
			{
				id: 'call_bad',
				type: 'function',
				function: { name: 'patch_app_file', arguments: '{"path": "u/x", "old_string": "trunc' }
			},
			{
				id: 'call_ok',
				type: 'function',
				function: { name: 'read_file', arguments: '{"file": "a.txt"}' }
			}
		]
	}

	it('replaces only unparseable arguments with {}', () => {
		const [sanitized] = sanitizeToolCallArguments([poisoned]) as any[]
		expect(sanitized.tool_calls[0].function.arguments).toBe('{}')
		expect(sanitized.tool_calls[1].function.arguments).toBe('{"file": "a.txt"}')
		// The stored history object is not mutated
		expect((poisoned as any).tool_calls[0].function.arguments).toContain('trunc')
	})

	it('returns untouched messages by reference', () => {
		const user: ChatCompletionMessageParam = { role: 'user', content: 'hi' }
		const validAssistant: ChatCompletionMessageParam = {
			role: 'assistant',
			tool_calls: [{ id: 'c1', type: 'function', function: { name: 'read_file', arguments: '{}' } }]
		}
		const result = sanitizeToolCallArguments([user, validAssistant])
		expect(result[0]).toBe(user)
		expect(result[1]).toBe(validAssistant)
	})
})
