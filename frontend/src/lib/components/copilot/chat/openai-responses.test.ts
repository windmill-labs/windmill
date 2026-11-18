import { describe, expect, it, vi } from 'vitest'
import {
	buildPromptCacheKey,
	getOpenAIResponsesCompletion,
	openAIWebSearchDetails,
	parseOpenAIResponsesCompletion,
	toResponsesContent
} from './openai-responses'
import { OutputTokenLimitError } from './outputTokenLimit'

const mocks = vi.hoisted(() => ({
	getProviderAndCompletionConfig: vi.fn(),
	applyReasoningToConfig: vi.fn(),
	modelSupportsWebSearch: vi.fn()
}))

// openai-responses.ts pulls in the chat client/registry layer at import time; the
// helper under test is pure, so stub those side-effecting modules away.
vi.mock('../lib', () => ({
	createOpenAIProxyClient: vi.fn(),
	getAiProxyBaseURL: vi.fn(),
	getProviderAndCompletionConfig: mocks.getProviderAndCompletionConfig,
	modelSupportsWebSearch: mocks.modelSupportsWebSearch,
	workspaceAIClients: {}
}))

vi.mock('../reasoningRegistry', () => ({
	applyReasoningToConfig: mocks.applyReasoningToConfig
}))

vi.mock('./shared', () => ({
	processToolCall: vi.fn(),
	appendPendingToolImages: vi.fn(),
	queuedToolStatus: vi.fn()
}))

describe('toResponsesContent', () => {
	it('passes a plain string through unchanged', () => {
		expect(toResponsesContent('hello')).toBe('hello')
	})

	it('maps text parts to input_text and image_url parts to input_image (string url)', () => {
		const out = toResponsesContent([
			{ type: 'text', text: 'describe this' },
			{ type: 'image_url', image_url: { url: 'data:image/png;base64,ZZZZ' } }
		]) as any[]

		expect(out).toEqual([
			{ type: 'input_text', text: 'describe this' },
			{ type: 'input_image', image_url: 'data:image/png;base64,ZZZZ' }
		])
	})
})

describe('buildPromptCacheKey', () => {
	it('composes workspace, provider, model and surface', () => {
		expect(buildPromptCacheKey('chat', { provider: 'openai', model: 'gpt-5.6' }, 'admins')).toBe(
			'admins:openai:gpt-5.6:chat'
		)
	})

	// Over 64 characters OpenAI rejects the key outright, and an Azure deployment name
	// is user-chosen, so the model segment can be arbitrarily long.
	it('stays within the provider length bound for a long deployment name', () => {
		const key = buildPromptCacheKey(
			'chat',
			{ provider: 'azure_openai', model: 'our-very-long-production-deployment-name-for-gpt-5-6' },
			'some-customer-workspace'
		)

		expect(key.length).toBeLessThanOrEqual(64)
		expect(key.startsWith('some-customer-workspace:azure_openai:')).toBe(true)
	})

	// A max-length workspace on the longest provider name leaves no room for the model
	// or surface, so only the digest can keep those keys apart.
	it('keeps distinct models and surfaces apart when the workspace fills the bound', () => {
		const workspace = 'w'.repeat(50)
		const keys = [
			buildPromptCacheKey('chat', { provider: 'azure_openai', model: 'gpt-5.6' }, workspace),
			buildPromptCacheKey('chat', { provider: 'azure_openai', model: 'gpt-5.1' }, workspace),
			buildPromptCacheKey('script', { provider: 'azure_openai', model: 'gpt-5.6' }, workspace)
		]

		for (const key of keys) {
			expect(key.length).toBeLessThanOrEqual(64)
		}
		expect(new Set(keys).size).toBe(3)
		// Stable, or every turn would land on a different cache.
		expect(keys[0]).toBe(
			buildPromptCacheKey('chat', { provider: 'azure_openai', model: 'gpt-5.6' }, workspace)
		)
	})
})

describe('getOpenAIResponsesCompletion prompt cache key', () => {
	function stubClient() {
		const stream = vi.fn().mockReturnValue({})
		return { client: { responses: { stream } } as any, stream }
	}

	function stubConfig() {
		mocks.getProviderAndCompletionConfig.mockReturnValue({
			provider: 'openai',
			config: { model: 'gpt-5.6' }
		})
		// The reasoning layer rebuilds the config object; keep it a pass-through so the
		// assertion is about the body this module produces.
		mocks.applyReasoningToConfig.mockImplementation((config) => config)
	}

	// `gpt-5.6` and later only match a cached prefix reliably when the request carries
	// the routing key, so dropping it from the body silently forfeits the cache.
	it('puts the caller key on the request body', async () => {
		stubConfig()
		const { client, stream } = stubClient()

		await getOpenAIResponsesCompletion([], new AbortController(), undefined, {
			openaiClient: client,
			promptCacheKey: 'admins:openai:gpt-5.6:chat'
		})

		expect(stream.mock.calls[0][0]).toEqual(
			expect.objectContaining({ prompt_cache_key: 'admins:openai:gpt-5.6:chat' })
		)
	})

	it('omits the field entirely when the caller withdrew the key', async () => {
		stubConfig()
		const { client, stream } = stubClient()

		await getOpenAIResponsesCompletion([], new AbortController(), undefined, {
			openaiClient: client,
			promptCacheKey: undefined
		})

		expect(stream.mock.calls[0][0]).not.toHaveProperty('prompt_cache_key')
	})
})

describe('openAIWebSearchDetails', () => {
	it('drops the call id DeepSeek lists among its queries', () => {
		expect(
			openAIWebSearchDetails({ action: { queries: ['windmill', 'ws_call_id=call_00_x'] } }).query
		).toBe('windmill')
	})

	it('prefers the queries array over the deprecated singular query', () => {
		expect(
			openAIWebSearchDetails({ action: { type: 'search', queries: ['a', 'b'], query: 'old' } })
		).toEqual({ query: 'a, b', sources: undefined })
	})

	it('falls back to the singular query when queries is absent', () => {
		expect(openAIWebSearchDetails({ action: { type: 'search', query: 'solo' } }).query).toBe('solo')
	})

	it('keeps only url-shaped sources and returns undefined query when neither field is usable', () => {
		expect(
			openAIWebSearchDetails({
				action: { type: 'search', queries: [], sources: [{ url: 'https://a.dev' }, { nope: 1 }] }
			})
		).toEqual({ query: undefined, sources: [{ url: 'https://a.dev' }] })
	})
})

describe('parseOpenAIResponsesCompletion output token limit', () => {
	it('fails a response the stream reports incomplete at max_output_tokens', async () => {
		const handlers: Record<string, (event: any) => void> = {}
		// The SDK's final snapshot of an incomplete response still reads in_progress;
		// only the response.incomplete event carries the reason.
		const runner = {
			on: (name: string, fn: (event: any) => void) => {
				handlers[name] = fn
			},
			done: async () => {
				handlers['response.incomplete']?.({
					type: 'response.incomplete',
					response: { incomplete_details: { reason: 'max_output_tokens' } }
				})
			},
			finalResponse: async () => ({ status: 'in_progress', output: [] })
		}

		const parsed = parseOpenAIResponsesCompletion(
			runner as any,
			{ onNewToken: vi.fn(), onMessageEnd: vi.fn(), setToolStatus: vi.fn() } as any,
			[],
			[],
			[],
			{}
		)

		await expect(parsed).rejects.toBeInstanceOf(OutputTokenLimitError)
	})
})

// DeepSeek refuses a function tool named like one of its search tools (`open_page` is the
// chat's own), and refuses a tool-call turn whose reasoning is not passed back.
describe('DeepSeek web search on the Responses API', () => {
	it('renames the reserved tools and replays the reasoning of a tool-call turn', async () => {
		mocks.getProviderAndCompletionConfig.mockReturnValue({
			provider: 'deepseek',
			config: {
				model: 'deepseek-v4-pro',
				tools: ['open_page', 'list_runs'].map((name) => ({
					type: 'function',
					function: { name, parameters: {} }
				}))
			}
		})
		mocks.applyReasoningToConfig.mockImplementation((config) => config)
		mocks.modelSupportsWebSearch.mockReturnValue(true)
		const stream = vi.fn().mockReturnValue({})

		await getOpenAIResponsesCompletion(
			[
				{ role: 'user', content: 'open runs' },
				{
					role: 'assistant',
					content: '',
					reasoning_content: 'I open it.',
					tool_calls: [
						{ id: 'c1', type: 'function', function: { name: 'open_page', arguments: '{}' } }
					]
				} as any,
				{ role: 'tool', tool_call_id: 'c1', content: 'ok' }
			],
			new AbortController(),
			undefined,
			{ openaiClient: { responses: { stream } } as any, webSearch: true }
		)

		const body = stream.mock.calls[0][0]
		expect(body.tools.map((t: any) => t.name ?? t.type)).toEqual([
			'wm_open_page',
			'list_runs',
			'web_search'
		])
		expect(body.input.slice(1, 3)).toEqual([
			{ type: 'reasoning', summary: [], content: [{ type: 'reasoning_text', text: 'I open it.' }] },
			expect.objectContaining({ type: 'function_call', name: 'wm_open_page' })
		])
	})

	it('maps a renamed call back and keeps text and reasoning on the tool-call turn', async () => {
		const handlers: Record<string, (event: any) => void> = {}
		const runner = {
			on: (name: string, fn: (event: any) => void) => {
				handlers[name] = fn
			},
			done: async () => {
				handlers['response.reasoning_text.delta']({ delta: 'I open it.' })
				handlers['response.output_text.delta']({ delta: 'Opening.' })
				handlers['response.output_item.added']({
					item: { type: 'function_call', id: 'fc1', name: 'wm_open_page', call_id: 'c1' }
				})
				handlers['response.function_call_arguments.done']({ item_id: 'fc1', arguments: '{}' })
			},
			finalResponse: async () => ({ status: 'completed', output: [] })
		}
		const messages: any[] = []

		await parseOpenAIResponsesCompletion(
			runner as any,
			{ onNewToken: vi.fn(), onMessageEnd: vi.fn(), setToolStatus: vi.fn() } as any,
			messages,
			[],
			[{ def: { type: 'function', function: { name: 'open_page' } } } as any],
			{},
			{ provider: 'deepseek' }
		)

		expect(messages[0]).toEqual({
			role: 'assistant',
			content: 'Opening.',
			reasoning_content: 'I open it.',
			tool_calls: [
				{ id: 'fc1', type: 'function', function: { name: 'open_page', arguments: '{}' } }
			]
		})
	})
})
