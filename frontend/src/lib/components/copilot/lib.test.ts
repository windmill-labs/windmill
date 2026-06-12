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
import {
	getDefaultChatTemperature,
	getKnownModelContextWindow,
	getModelContextWindow,
	modelDisallowsSamplingParams
} from './modelConfig'
import { supportsAutocomplete } from './utils'

type AssistantMessageWithReasoning = ChatCompletionMessageParam & {
	role: 'assistant'
	content?: string
	reasoning_content?: string
	tool_calls?: ChatCompletionMessageFunctionToolCall[]
}

describe('modelConfig', () => {
	it('flags Fable 5 model IDs via includes matching', () => {
		expect(modelDisallowsSamplingParams('claude-fable-5')).toBe(true)
		expect(modelDisallowsSamplingParams('claude-fable-5@20260611')).toBe(true)
		expect(modelDisallowsSamplingParams('claude-fable-5/thinking')).toBe(true)
		expect(modelDisallowsSamplingParams('anthropic/claude-fable-5')).toBe(true)
	})

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

	it('omits deterministic temperature for Anthropic Fable 5 chat requests', () => {
		expect(
			getDefaultChatTemperature({ provider: 'anthropic', model: 'claude-fable-5' })
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
})

describe('model context windows', () => {
	it('maps Sonnet/Opus 4.6+ Claude models to the 1M window', () => {
		expect(getKnownModelContextWindow('claude-sonnet-4-6')).toBe(1000000)
		expect(getKnownModelContextWindow('claude-opus-4-6')).toBe(1000000)
		expect(getKnownModelContextWindow('claude-opus-4-8')).toBe(1000000)
		expect(getKnownModelContextWindow('anthropic.claude-sonnet-4-6-v1:0')).toBe(1000000)
	})

	it('keeps Haiku and older Claude models at 200K', () => {
		expect(getKnownModelContextWindow('claude-haiku-4-5')).toBe(200000)
		expect(getKnownModelContextWindow('global.anthropic.claude-haiku-4-5-20251001-v1:0')).toBe(
			200000
		)
		expect(getKnownModelContextWindow('claude-3-5-sonnet-latest')).toBe(200000)
		expect(getKnownModelContextWindow('claude-sonnet-4-5-20250929')).toBe(200000)
		expect(getKnownModelContextWindow('claude-opus-4-1')).toBe(200000)
		// date-suffixed base ids without a minor version: the date must not be
		// captured as the version
		expect(getKnownModelContextWindow('claude-sonnet-4-20250514')).toBe(200000)
		expect(getKnownModelContextWindow('anthropic.claude-sonnet-4-20250514-v1:0')).toBe(200000)
	})

	it('keeps base GPT-5 models at 400K while GPT-5.4+ get the 1M window', () => {
		expect(getKnownModelContextWindow('gpt-5')).toBe(400000)
		expect(getKnownModelContextWindow('gpt-5-mini')).toBe(400000)
		expect(getKnownModelContextWindow('gpt-5.2')).toBe(400000)
		expect(getKnownModelContextWindow('gpt-5.4')).toBe(1000000)
		expect(getKnownModelContextWindow('gpt-5.5')).toBe(1000000)
	})

	it('maps recent Gemini and DeepSeek models to the 1M window', () => {
		expect(getKnownModelContextWindow('gemini-3.1-pro')).toBe(1000000)
		expect(getKnownModelContextWindow('gemini-3-flash')).toBe(1000000)
		expect(getKnownModelContextWindow('gemini-2.5-flash')).toBe(1000000)
		expect(getKnownModelContextWindow('deepseek-v4-pro')).toBe(1000000)
		expect(getKnownModelContextWindow('deepseek-chat')).toBe(1000000)
		expect(getKnownModelContextWindow('deepseek-reasoner')).toBe(1000000)
	})

	it('returns undefined for unrecognized models, 128K via the defaulting wrapper', () => {
		expect(getKnownModelContextWindow('some-custom-model')).toBeUndefined()
		expect(getModelContextWindow('some-custom-model')).toBe(128000)
	})
})
