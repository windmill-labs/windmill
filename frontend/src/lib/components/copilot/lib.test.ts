import type {
	ChatCompletionChunk,
	ChatCompletionMessageFunctionToolCall,
	ChatCompletionMessageParam
} from 'openai/resources/index.mjs'
import { describe, expect, it } from 'vitest'
import {
	buildAssistantTextMessage,
	buildAssistantToolCallMessage,
	getReasoningContentDelta,
	splitContentDelta
} from './chat/openaiReasoning'
import { parseFimCompletionChoice } from './fim'
import { getDefaultChatTemperature, modelDisallowsSamplingParams } from './modelConfig'
import { supportsAutocomplete } from './utils'

type AssistantMessageWithReasoning = ChatCompletionMessageParam & {
	role: 'assistant'
	content?: string
	reasoning_content?: string
	tool_calls?: ChatCompletionMessageFunctionToolCall[]
}

describe('modelConfig', () => {
	it('flags Opus 4.7 model IDs via includes matching', () => {
		expect(modelDisallowsSamplingParams('claude-opus-4-7')).toBe(true)
		expect(modelDisallowsSamplingParams('claude-opus-4-7@20260416')).toBe(true)
		expect(modelDisallowsSamplingParams('claude-opus-4-7/thinking')).toBe(true)
		expect(modelDisallowsSamplingParams('anthropic/claude-opus-4-7')).toBe(true)
	})

	it('flags Opus 4.8 model IDs via includes matching', () => {
		expect(modelDisallowsSamplingParams('claude-opus-4-8')).toBe(true)
		expect(modelDisallowsSamplingParams('claude-opus-4-8@20260416')).toBe(true)
		expect(modelDisallowsSamplingParams('claude-opus-4-8/thinking')).toBe(true)
		expect(modelDisallowsSamplingParams('anthropic/claude-opus-4-8')).toBe(true)
	})

	it('omits deterministic temperature for Anthropic Opus 4.7 chat requests', () => {
		expect(
			getDefaultChatTemperature({ provider: 'anthropic', model: 'claude-opus-4-7' })
		).toBeUndefined()
	})

	it('omits deterministic temperature for non-anthropic providers carrying Opus 4.7 models', () => {
		expect(
			getDefaultChatTemperature({ provider: 'openrouter', model: 'anthropic/claude-opus-4-7' })
		).toBeUndefined()
	})

	it('keeps deterministic temperature for older Anthropic models', () => {
		expect(getDefaultChatTemperature({ provider: 'anthropic', model: 'claude-sonnet-4-6' })).toBe(0)
	})

	it('flags gpt-5+ and o-series reasoning models via prefix matching', () => {
		expect(modelDisallowsSamplingParams('gpt-5')).toBe(true)
		expect(modelDisallowsSamplingParams('gpt-5.5')).toBe(true)
		expect(modelDisallowsSamplingParams('gpt-5-mini')).toBe(true)
		expect(modelDisallowsSamplingParams('o1')).toBe(true)
		expect(modelDisallowsSamplingParams('o3')).toBe(true)
		expect(modelDisallowsSamplingParams('o4-mini')).toBe(true)
		// provider-prefixed identifiers (e.g. OpenRouter) match on the bare model id
		expect(modelDisallowsSamplingParams('openai/gpt-5')).toBe(true)
		expect(modelDisallowsSamplingParams('openai/o3')).toBe(true)
	})

	it('keeps sampling params for non-reasoning models that merely share a prefix', () => {
		// gpt-4o starts with "gpt-" but not "gpt-5"; the "o" is mid-string, not a prefix
		expect(modelDisallowsSamplingParams('gpt-4o')).toBe(false)
		expect(modelDisallowsSamplingParams('gpt-4o-mini')).toBe(false)
		// the provider prefix "openai/" must not be mistaken for an o-series model
		expect(modelDisallowsSamplingParams('openai/gpt-4o')).toBe(false)
		// the o-series match requires a digit after "o", so non-OpenAI ids that
		// start with "o" (Mistral open-* family, OpenRouter optimus-*/openchat-*)
		// keep their deterministic temperature
		expect(modelDisallowsSamplingParams('open-mistral-7b')).toBe(false)
		expect(modelDisallowsSamplingParams('open-mixtral-8x7b')).toBe(false)
		expect(modelDisallowsSamplingParams('open-mistral-nemo-2407')).toBe(false)
		expect(modelDisallowsSamplingParams('optimus-alpha')).toBe(false)
		expect(modelDisallowsSamplingParams('openchat/openchat-7b')).toBe(false)
	})

	it('keeps deterministic temperature for Mistral open-* models', () => {
		expect(getDefaultChatTemperature({ provider: 'mistral', model: 'open-mixtral-8x7b' })).toBe(0)
	})

	it('omits deterministic temperature for gpt-5.5 routed through the customai gateway', () => {
		expect(getDefaultChatTemperature({ provider: 'customai', model: 'gpt-5.5' })).toBeUndefined()
	})

	it('omits deterministic temperature for o-series models on the customai gateway', () => {
		expect(getDefaultChatTemperature({ provider: 'customai', model: 'o3' })).toBeUndefined()
	})

	it('keeps deterministic temperature for gpt-4o on the customai gateway', () => {
		expect(getDefaultChatTemperature({ provider: 'customai', model: 'gpt-4o' })).toBe(0)
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

	it('omits reasoning_content for Mistral, which rejects it on input messages', () => {
		const assistantMessage = buildAssistantToolCallMessage({
			content: '',
			reasoning: {
				hasReasoningContent: true,
				reasoningContent: 'thinking trace'
			},
			toolCalls: [],
			provider: 'mistral'
		}) as AssistantMessageWithReasoning

		expect(assistantMessage.reasoning_content).toBeUndefined()
		// The content key is still present so the message stays well-formed.
		expect(assistantMessage).toMatchObject({ role: 'assistant', content: '', tool_calls: [] })
	})
})

describe('splitContentDelta', () => {
	it('passes plain string deltas through as answer text', () => {
		expect(splitContentDelta('hello')).toEqual({ reasoning: '', text: 'hello' })
		expect(splitContentDelta(null)).toEqual({ reasoning: '', text: '' })
		expect(splitContentDelta(undefined)).toEqual({ reasoning: '', text: '' })
	})

	it('routes Mistral thinking parts to reasoning and text parts to the answer', () => {
		expect(
			splitContentDelta([
				{ type: 'thinking', thinking: [{ type: 'text', text: 'step 1. ' }], closed: true },
				{ type: 'thinking', thinking: [{ type: 'text', text: 'step 2.' }] },
				{ type: 'text', text: '42' }
			])
		).toEqual({ reasoning: 'step 1. step 2.', text: '42' })
	})

	it('ignores malformed parts', () => {
		expect(splitContentDelta([{ type: 'thinking' }, { type: 'text' }, 'junk', null])).toEqual({
			reasoning: '',
			text: ''
		})
	})
})
