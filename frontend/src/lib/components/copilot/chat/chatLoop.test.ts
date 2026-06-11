import { describe, expect, it, vi } from 'vitest'
import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'

// truncateToToolPairedPrefix is pure, but importing chatLoop.ts pulls in the
// completion seams (which transitively load monaco). Stub them out so the unit
// under test loads in the node test env.
vi.mock('monaco-editor', () => ({ Selection: class {} }))
vi.mock('../lib', () => ({ getCompletion: vi.fn(), parseOpenAICompletion: vi.fn() }))
vi.mock('./anthropic', () => ({
	getAnthropicCompletion: vi.fn(),
	parseAnthropicCompletion: vi.fn()
}))
vi.mock('./openai-responses', () => ({
	getOpenAIResponsesCompletion: vi.fn(),
	parseOpenAIResponsesCompletion: vi.fn()
}))

import { truncateToToolPairedPrefix } from './chatLoop'

// Builders for the message shapes the chat loop accumulates.
const assistant = (content: string): ChatCompletionMessageParam => ({ role: 'assistant', content })
const assistantTools = (...ids: string[]): ChatCompletionMessageParam => ({
	role: 'assistant',
	content: '',
	tool_calls: ids.map((id) => ({
		id,
		type: 'function',
		function: { name: 'do_thing', arguments: '{}' }
	}))
})
const tool = (id: string): ChatCompletionMessageParam => ({
	role: 'tool',
	tool_call_id: id,
	content: 'result'
})
const user = (content: string): ChatCompletionMessageParam => ({ role: 'user', content })

describe('truncateToToolPairedPrefix', () => {
	it('returns an empty array unchanged', () => {
		expect(truncateToToolPairedPrefix([])).toEqual([])
	})

	it('drops a trailing dangling tool_call (aborted before the result)', () => {
		const msgs = [assistantTools('a')]
		expect(truncateToToolPairedPrefix(msgs)).toEqual([])
	})

	it('drops a partially-answered batch entirely (A answered, B missing)', () => {
		const msgs = [assistantTools('a', 'b'), tool('a')]
		expect(truncateToToolPairedPrefix(msgs)).toEqual([])
	})

	it('keeps text + a completed round-trip, then drops the dangling tail', () => {
		const msgs = [
			assistant('let me check'),
			assistantTools('a'),
			tool('a'),
			assistant('more'),
			assistantTools('b') // dangling
		]
		expect(truncateToToolPairedPrefix(msgs)).toEqual([
			assistant('let me check'),
			assistantTools('a'),
			tool('a'),
			assistant('more')
		])
	})

	it('treats a user message as a valid boundary only when no tool calls are pending', () => {
		const ok = [assistantTools('a'), tool('a'), user('next')]
		expect(truncateToToolPairedPrefix(ok)).toEqual(ok)

		const dangling = [assistantTools('a'), user('next')]
		expect(truncateToToolPairedPrefix(dangling)).toEqual([])
	})

	it('leaves a valid full conversation unchanged (no loss on the normal path)', () => {
		const msgs = [
			assistant('thinking'),
			assistantTools('a', 'b'),
			tool('a'),
			tool('b'),
			assistant('done')
		]
		expect(truncateToToolPairedPrefix(msgs)).toEqual(msgs)
	})
})
