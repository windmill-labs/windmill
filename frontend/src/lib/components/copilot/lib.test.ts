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
import {
	getKnownModelContextWindow,
	getModelContextWindow,
	modelSupportsVision,
	requiresMaxCompletionTokens
} from './modelConfig'
import { supportsAutocomplete } from './utils'

type AssistantMessageWithReasoning = ChatCompletionMessageParam & {
	role: 'assistant'
	content?: string
	reasoning_content?: string
	tool_calls?: ChatCompletionMessageFunctionToolCall[]
}

describe('modelConfig', () => {
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

describe('modelSupportsVision', () => {
	// Each ships in a provider's `defaultModels`, so a user can select it and try to
	// attach. Verdicts are from provider API docs, not model marketing: the two
	// diverge (DeepSeek V4 and o3-mini both have vision their APIs never exposed).
	it.each([
		['openai', 'o3-mini'],
		['openai', 'o1-mini'],
		['mistral', 'codestral-latest'],
		['deepseek', 'deepseek-v4-pro'],
		['deepseek', 'deepseek-chat'],
		['deepseek', 'deepseek-reasoner'],
		['groq', 'llama-3.3-70b-versatile'],
		['groq', 'llama-3.1-8b-instant'],
		['azure_foundry', 'DeepSeek-R1'],
		['azure_foundry', 'Llama-3.3-70B-Instruct'],
		['azure_foundry', 'Phi-4'],
		['azure_foundry', 'Mistral-Large-2411'],
		['openrouter', 'meta-llama/llama-3.2-3b-instruct:free'],
		['togetherai', 'meta-llama/Llama-3.3-70B-Instruct-Turbo']
	])('refuses images on text-only %s/%s', (provider, model) => {
		expect(modelSupportsVision(provider as any, model)).toBe(false)
	})

	it.each([
		['anthropic', 'claude-sonnet-4-6'],
		['googleai', 'gemini-2.5-flash'],
		['googleai', 'gemini-2.5-flash-lite'],
		['googleai', 'gemini-3.1-pro-preview'],
		['openai', 'gpt-4o'],
		['openai', 'gpt-5'],
		['openai', 'o3'],
		['openai', 'o4-mini'],
		['aws_bedrock', 'global.anthropic.claude-haiku-4-5-20251001-v1:0']
	])('allows images on %s/%s', (provider, model) => {
		expect(modelSupportsVision(provider as any, model)).toBe(true)
	})

	// The reason this is an exact-match set. Each of these WOULD be wrongly blocked
	// by a substring of an entry above, and each takes images via its API.
	it.each([
		['azure_foundry', 'Mistral-Large-3'], // substring of 'mistral-large-2411'
		['azure_foundry', 'Phi-4-multimodal-instruct'], // substring of 'phi-4'
		['openrouter', 'meta-llama/llama-3.2-90b-vision-instruct'], // 'llama-3.2-...'
		['groq', 'meta-llama/llama-4-scout-17b-16e-instruct'],
		['groq', 'qwen/qwen3.6-27b']
	])('does not let a text-only id shadow the vision model %s/%s', (provider, model) => {
		expect(modelSupportsVision(provider as any, model)).toBe(true)
	})

	// Permissive by design: a wrong "no" blocks a working model with no override,
	// while a wrong "yes" costs one turn and the failure path recovers it.
	it('allows unknown and custom models', () => {
		expect(modelSupportsVision('customai' as any, 'some-internal-vlm')).toBe(true)
		expect(modelSupportsVision('deepseek' as any, 'deepseek-v9-sees-everything')).toBe(true)
		expect(modelSupportsVision(undefined, undefined)).toBe(true)
	})

	// The reason entries are keyed by provider, not id alone: an id proves nothing
	// about a different endpoint. A Custom AI deployment serving a vision model
	// under a colliding name must not inherit another provider's verdict.
	it("does not apply one provider's text-only verdict to another provider's model", () => {
		expect(modelSupportsVision('customai' as any, 'deepseek-chat')).toBe(true)
		expect(modelSupportsVision('customai' as any, 'llama-3.3-70b-versatile')).toBe(true)
	})
})
