import { describe, expect, it } from 'vitest'
import {
	anthropicUsageToChatTokenUsage,
	billedTokens,
	openAICompletionsUsageToChatTokenUsage
} from './tokenUsage'

// The two providers report cache tokens under opposite conventions — Anthropic's
// input_tokens excludes them, OpenAI's includes them. Both are normalized so that
// `prompt` is the whole input, which is what makes `prompt - cached` the uncached
// share. Getting this backwards double-counts (or loses) the cached prefix, which
// is most of a long chat's input.
describe('billedTokens', () => {
	it('derives uncached input under the Anthropic convention', () => {
		const usage = anthropicUsageToChatTokenUsage({
			input_tokens: 1000,
			output_tokens: 200,
			cache_creation_input_tokens: 300,
			cache_read_input_tokens: 5000
		})
		expect(usage.prompt).toBe(6300)
		expect(billedTokens(usage)).toEqual({
			input: 1000,
			cacheRead: 5000,
			cacheWrite: 300,
			output: 200
		})
	})

	it('derives uncached input under the OpenAI convention', () => {
		const usage = openAICompletionsUsageToChatTokenUsage({
			prompt_tokens: 6000,
			completion_tokens: 200,
			prompt_tokens_details: { cached_tokens: 5000 }
		})
		expect(usage.prompt).toBe(6000)
		expect(billedTokens(usage)).toEqual({
			input: 1000,
			cacheRead: 5000,
			cacheWrite: 0,
			output: 200
		})
	})

	// OpenRouter extends the OpenAI shape with cache-creation tokens, counted
	// inside prompt_tokens like the reads beside them. Missing the field bills
	// them as uncached input.
	it('splits out OpenRouter cache-creation tokens', () => {
		const usage = openAICompletionsUsageToChatTokenUsage({
			prompt_tokens: 6300,
			completion_tokens: 200,
			prompt_tokens_details: { cached_tokens: 5000, cache_write_tokens: 300 }
		})
		expect(billedTokens(usage)).toEqual({
			input: 1000,
			cacheRead: 5000,
			cacheWrite: 300,
			output: 200
		})
	})
})
