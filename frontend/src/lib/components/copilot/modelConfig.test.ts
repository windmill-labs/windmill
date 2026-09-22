import { describe, expect, it } from 'vitest'
import {
	getConfiguredModelContextWindow,
	getEffectiveModelContextWindow,
	usesAnthropicMessagesApi
} from './modelConfig'

describe('workspace context window overrides', () => {
	const overrides = { 'customai:qwen-local': 32_768, 'anthropic:claude-sonnet-5': 200_000 }

	it('wins over the built-in table and the assumed window for its exact provider:model', () => {
		expect(getEffectiveModelContextWindow('customai', 'qwen-local', overrides)).toBe(32_768)
		expect(getConfiguredModelContextWindow('customai', 'qwen-local', overrides)).toBe(32_768)
		expect(getEffectiveModelContextWindow('anthropic', 'claude-sonnet-5', overrides)).toBe(200_000)
	})

	it('does not apply to the same model id served by another provider', () => {
		expect(getEffectiveModelContextWindow('openrouter', 'claude-sonnet-5', overrides)).toBe(
			1_000_000
		)
		expect(getConfiguredModelContextWindow('openai', 'qwen-local', overrides)).toBeUndefined()
		expect(getEffectiveModelContextWindow('openai', 'qwen-local', overrides)).toBe(128_000)
	})
})

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
