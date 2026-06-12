import type {
	ChatCompletionChunk,
	ChatCompletionMessageFunctionToolCall,
	ChatCompletionMessageParam
} from 'openai/resources/index.mjs'
import { describe, expect, it } from 'vitest'
import {
	buildAssistantTextMessage,
	buildAssistantToolCallMessage,
	getReasoningContentDelta
} from './chat/openaiReasoning'
import { parseFimCompletionChoice } from './fim'
import { requiresMaxCompletionTokens } from './modelConfig'
import { supportsAutocomplete } from './utils'

type AssistantMessageWithReasoning = ChatCompletionMessageParam & {
	role: 'assistant'
	content?: string
	reasoning_content?: string
	tool_calls?: ChatCompletionMessageFunctionToolCall[]
}

describe('modelConfig', () => {
	it('flags Fable 5 model IDs via includes matching', () => {
		expect(requiresMaxCompletionTokens('claude-fable-5')).toBe(true)
		expect(requiresMaxCompletionTokens('claude-fable-5@20260611')).toBe(true)
		expect(requiresMaxCompletionTokens('claude-fable-5/thinking')).toBe(true)
		expect(requiresMaxCompletionTokens('anthropic/claude-fable-5')).toBe(true)
	})

	it('flags Opus 4.7 model IDs via includes matching', () => {
		expect(requiresMaxCompletionTokens('claude-opus-4-7')).toBe(true)
		expect(requiresMaxCompletionTokens('claude-opus-4-7@20260416')).toBe(true)
		expect(requiresMaxCompletionTokens('claude-opus-4-7/thinking')).toBe(true)
		expect(requiresMaxCompletionTokens('anthropic/claude-opus-4-7')).toBe(true)
	})

	it('flags Opus 4.8 model IDs via includes matching', () => {
		expect(requiresMaxCompletionTokens('claude-opus-4-8')).toBe(true)
		expect(requiresMaxCompletionTokens('claude-opus-4-8@20260416')).toBe(true)
		expect(requiresMaxCompletionTokens('claude-opus-4-8/thinking')).toBe(true)
		expect(requiresMaxCompletionTokens('anthropic/claude-opus-4-8')).toBe(true)
	})

	it('flags gpt-5+ and o-series reasoning models via prefix matching', () => {
		expect(requiresMaxCompletionTokens('gpt-5')).toBe(true)
		expect(requiresMaxCompletionTokens('gpt-5.5')).toBe(true)
		expect(requiresMaxCompletionTokens('gpt-5-mini')).toBe(true)
		expect(requiresMaxCompletionTokens('o1')).toBe(true)
		expect(requiresMaxCompletionTokens('o3')).toBe(true)
		expect(requiresMaxCompletionTokens('o4-mini')).toBe(true)
		// provider-prefixed identifiers (e.g. OpenRouter) match on the bare model id
		expect(requiresMaxCompletionTokens('openai/gpt-5')).toBe(true)
		expect(requiresMaxCompletionTokens('openai/o3')).toBe(true)
	})

	it('does not require max_completion_tokens for non-reasoning models that merely share a prefix', () => {
		// gpt-4o starts with "gpt-" but not "gpt-5"; the "o" is mid-string, not a prefix
		expect(requiresMaxCompletionTokens('gpt-4o')).toBe(false)
		expect(requiresMaxCompletionTokens('gpt-4o-mini')).toBe(false)
		// the provider prefix "openai/" must not be mistaken for an o-series model
		expect(requiresMaxCompletionTokens('openai/gpt-4o')).toBe(false)
		// the o-series match requires a digit after "o", so non-OpenAI ids that
		// start with "o" (Mistral open-* family, OpenRouter optimus-*/openchat-*)
		// do not require max_completion_tokens
		expect(requiresMaxCompletionTokens('open-mistral-7b')).toBe(false)
		expect(requiresMaxCompletionTokens('open-mixtral-8x7b')).toBe(false)
		expect(requiresMaxCompletionTokens('open-mistral-nemo-2407')).toBe(false)
		expect(requiresMaxCompletionTokens('optimus-alpha')).toBe(false)
		expect(requiresMaxCompletionTokens('openchat/openchat-7b')).toBe(false)
	})
})

describe('fim autocomplete', () => {
	it('allows DeepSeek v4 pro and Codestral autocomplete models', () => {
		expect(supportsAutocomplete('codestral-latest')).toBe(true)
		expect(supportsAutocomplete('Codestral-2501')).toBe(true)
		expect(supportsAutocomplete('codestral-embed')).toBe(false)
		expect(supportsAutocomplete('deepseek-v4-pro')).toBe(true)
		expect(supportsAutocomplete('deepseek-chat')).toBe(false)
	})

	it('parses chat-shaped native FIM responses', () => {
		expect(
			parseFimCompletionChoice(
				{
					choices: [
						{
							message: { content: 'cache[key] = factory()' },
							finish_reason: 'stop'
						}
					]
				},
				'mistral'
			)
		).toEqual({ content: 'cache[key] = factory()', finish_reason: 'stop' })
	})

	it('parses DeepSeek native FIM completion responses', () => {
		expect(
			parseFimCompletionChoice(
				{
					choices: [
						{
							text: 'items?.length ?? 0',
							finish_reason: 'stop'
						}
					]
				},
				'deepseek'
			)
		).toEqual({ content: 'items?.length ?? 0', finish_reason: 'stop' })
	})
})

describe('openaiReasoning', () => {
	it('reads provider-specific reasoning_content deltas', () => {
		expect(
			getReasoningContentDelta({
				reasoning_content: 'thinking'
			} as ChatCompletionChunk.Choice.Delta & { reasoning_content: string })
		).toBe('thinking')
	})

	it('preserves DeepSeek reasoning_content on assistant tool-call messages', () => {
		const toolCalls: ChatCompletionMessageFunctionToolCall[] = [
			{
				id: 'call_1',
				type: 'function',
				function: {
					name: 'lookup',
					arguments: '{"query":"docs"}'
				}
			}
		]

		const assistantMessage = buildAssistantToolCallMessage({
			content: 'I will look that up.',
			reasoning: {
				hasReasoningContent: true,
				reasoningContent: 'First, I need a lookup.'
			},
			toolCalls
		}) as AssistantMessageWithReasoning

		expect(assistantMessage).toMatchObject({
			role: 'assistant',
			content: 'I will look that up.',
			reasoning_content: 'First, I need a lookup.',
			tool_calls: [
				{
					id: 'call_1',
					type: 'function',
					function: {
						name: 'lookup',
						arguments: '{"query":"docs"}'
					}
				}
			]
		})
	})

	it('does not preserve reasoning_content on text-only assistant messages', () => {
		expect(buildAssistantTextMessage('done')).toEqual({
			role: 'assistant',
			content: 'done'
		})
	})

	it('keeps empty reasoning_content when the provider emitted the field', () => {
		const assistantMessage = buildAssistantToolCallMessage({
			content: '',
			reasoning: {
				hasReasoningContent: true,
				reasoningContent: ''
			},
			toolCalls: []
		}) as AssistantMessageWithReasoning

		expect(assistantMessage).toMatchObject({
			role: 'assistant',
			content: '',
			reasoning_content: '',
			tool_calls: []
		})
	})
})
