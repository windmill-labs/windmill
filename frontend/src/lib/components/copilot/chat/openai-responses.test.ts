import { describe, expect, it, vi } from 'vitest'
import {
	buildPromptCacheKey,
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
