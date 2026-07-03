import { describe, expect, it, vi } from 'vitest'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import { convertOpenAIToAnthropicMessages } from './anthropic'

// anthropic.ts pulls in the chat client/registry layer at import time; the
// converter under test is pure, so stub those side-effecting modules away.
vi.mock('../lib', () => ({
	getProviderAndCompletionConfig: vi.fn(),
	workspaceAIClients: {}
}))

vi.mock('../reasoningRegistry', () => ({
	applyReasoningToConfig: vi.fn()
}))

vi.mock('./shared', () => ({
	processToolCall: vi.fn()
}))

describe('convertOpenAIToAnthropicMessages', () => {
	it('replays a captured assistant turn verbatim, skips the standalone text copy, and leaves the turn untouched', () => {
		const anthropicContent = [
			{ type: 'thinking', thinking: 'first', signature: 'sig-1' },
			{
				type: 'server_tool_use',
				id: 'srv_1',
				name: 'web_search',
				input: { query: 'nist password length' }
			},
			{
				type: 'web_search_tool_result',
				tool_use_id: 'srv_1',
				content: [{ type: 'web_search_result', title: 'NIST', url: 'https://nist.gov' }]
			},
			{ type: 'thinking', thinking: 'second', signature: 'sig-2' },
			{ type: 'tool_use', id: 'tool_1', name: 'list_resources', input: {} }
		]
		// Snapshot to assert the stored content is never mutated by the converter.
		const anthropicContentSnapshot = JSON.parse(JSON.stringify(anthropicContent))

		const messages: ChatCompletionMessageParam[] = [
			{ role: 'user', content: 'find the nist length then list resources' },
			// Standalone text the streamer emits before the tool-call message.
			{ role: 'assistant', content: 'Let me search the web.' },
			{
				role: 'assistant',
				tool_calls: [
					{
						id: 'tool_1',
						type: 'function',
						function: { name: 'list_resources', arguments: '{}' }
					}
				],
				_anthropicContent: anthropicContent
			} as any,
			{ role: 'tool', tool_call_id: 'tool_1', content: 'resource A, resource B' }
		]

		const { messages: out } = convertOpenAIToAnthropicMessages(messages)

		// user + the verbatim assistant turn + the tool result; the standalone text is dropped.
		expect(out).toHaveLength(3)
		expect(out[0]).toEqual({ role: 'user', content: 'find the nist length then list resources' })

		// Assistant turn replayed in original order, no reordering or dropped blocks.
		expect(out[1].role).toBe('assistant')
		expect((out[1].content as any[]).map((b) => b.type)).toEqual([
			'thinking',
			'server_tool_use',
			'web_search_tool_result',
			'thinking',
			'tool_use'
		])
		// The verbatim turn must stay byte-identical — no cache_control injected into it,
		// or a thinking-block signature would no longer validate.
		expect(out[1].content).toEqual(anthropicContentSnapshot)
		expect((out[1].content as any[]).some((b) => 'cache_control' in b)).toBe(false)
		expect(anthropicContent).toEqual(anthropicContentSnapshot)

		// The cache breakpoint lands on the trailing tool result, not the assistant turn.
		expect(out[2]).toEqual({
			role: 'user',
			content: [
				{
					type: 'tool_result',
					tool_use_id: 'tool_1',
					content: 'resource A, resource B',
					cache_control: { type: 'ephemeral' }
				}
			]
		})
	})

	it('converts a plain text assistant turn (no tools) and caches the trailing text block', () => {
		const messages: ChatCompletionMessageParam[] = [
			{ role: 'user', content: 'hello' },
			{ role: 'assistant', content: 'hi there' }
		]

		const { messages: out } = convertOpenAIToAnthropicMessages(messages)

		expect(out).toHaveLength(2)
		expect(out[0]).toEqual({ role: 'user', content: 'hello' })
		expect(out[1].role).toBe('assistant')
		expect(out[1].content).toEqual([
			{ type: 'text', text: 'hi there', cache_control: { type: 'ephemeral' } }
		])
	})

	it('falls back to _anthropicThinkingBlocks for turns persisted before _anthropicContent', () => {
		const thinkingBlocks = [{ type: 'thinking', thinking: 'reasoning', signature: 'sig-old' }]

		const messages: ChatCompletionMessageParam[] = [
			{ role: 'user', content: 'do a thing' },
			// The standalone text persisted alongside an old-style turn must NOT be skipped
			// (the fallback reconstruction relies on it for the assistant text).
			{ role: 'assistant', content: 'working on it' },
			{
				role: 'assistant',
				tool_calls: [
					{
						id: 'tool_old',
						type: 'function',
						function: { name: 'list_resources', arguments: '{}' }
					}
				],
				_anthropicThinkingBlocks: thinkingBlocks
			} as any
		]

		const { messages: out } = convertOpenAIToAnthropicMessages(messages)

		expect(out).toHaveLength(3)
		expect(out[1]).toEqual({ role: 'assistant', content: 'working on it' })
		const content = out[2].content as any[]
		// Thinking block re-injected first, then the tool_use.
		expect(content.map((b) => b.type)).toEqual(['thinking', 'tool_use'])
		expect(content[0]).toEqual(thinkingBlocks[0])
		expect(content[1]).toMatchObject({ type: 'tool_use', id: 'tool_old', name: 'list_resources' })
	})

	it('caches a trailing tool result even when the prior turn used no captured content', () => {
		const messages: ChatCompletionMessageParam[] = [
			{ role: 'user', content: 'q' },
			{
				role: 'assistant',
				tool_calls: [
					{ id: 't1', type: 'function', function: { name: 'list_resources', arguments: '{}' } }
				]
			} as any,
			{ role: 'tool', tool_call_id: 't1', content: 'done' }
		]

		const { messages: out } = convertOpenAIToAnthropicMessages(messages)

		const last = out[out.length - 1]
		expect(last.role).toBe('user')
		expect((last.content as any[])[0]).toMatchObject({
			type: 'tool_result',
			tool_use_id: 't1',
			cache_control: { type: 'ephemeral' }
		})
	})
})
