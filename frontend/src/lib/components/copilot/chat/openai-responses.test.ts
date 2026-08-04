import { describe, expect, it, vi } from 'vitest'
import {
	getOpenAIResponsesCompletion,
	openAIWebSearchDetails,
	toResponsesContent
} from './openai-responses'

const mocks = vi.hoisted(() => ({
	getProviderAndCompletionConfig: vi.fn(),
	applyReasoningToConfig: vi.fn()
}))

// openai-responses.ts pulls in the chat client/registry layer at import time; the
// helper under test is pure, so stub those side-effecting modules away.
vi.mock('../lib', () => ({
	createOpenAIProxyClient: vi.fn(),
	getAiProxyBaseURL: vi.fn(),
	getProviderAndCompletionConfig: mocks.getProviderAndCompletionConfig,
	providerSupportsWebSearch: vi.fn(),
	workspaceAIClients: {}
}))

vi.mock('../reasoningRegistry', () => ({
	applyReasoningToConfig: mocks.applyReasoningToConfig
}))

vi.mock('./shared', () => ({
	processToolCall: vi.fn(),
	appendPendingToolImages: vi.fn()
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
