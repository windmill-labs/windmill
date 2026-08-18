import { describe, expect, it } from 'vitest'
import { usesAnthropicMessagesApi } from './modelConfig'

describe('usesAnthropicMessagesApi', () => {
	it('routes the native Anthropic provider through the Messages API', () => {
		expect(usesAnthropicMessagesApi('anthropic', 'claude-sonnet-5')).toBe(true)
	})

	it('routes Azure Foundry Claude deployments through the Messages API', () => {
		expect(usesAnthropicMessagesApi('azure_foundry', 'claude-sonnet-5')).toBe(true)
		expect(usesAnthropicMessagesApi('azure_foundry', 'Claude-Opus-4-8')).toBe(true)
	})

	it('keeps other Azure Foundry models on the OpenAI-compatible path', () => {
		expect(usesAnthropicMessagesApi('azure_foundry', 'gpt-4o')).toBe(false)
		expect(usesAnthropicMessagesApi('azure_foundry', 'DeepSeek-R1')).toBe(false)
	})

	it('does not affect other providers', () => {
		expect(usesAnthropicMessagesApi('openai', 'gpt-4o')).toBe(false)
		expect(usesAnthropicMessagesApi('azure_openai', 'gpt-4o')).toBe(false)
	})
})
