import { describe, expect, it } from 'vitest'
import { anthropicModelDisallowsSamplingParams, getDefaultChatTemperature } from './modelConfig'

describe('modelConfig', () => {
	it('flags Anthropic Opus 4.7 model IDs as sampling-incompatible', () => {
		expect(anthropicModelDisallowsSamplingParams('claude-opus-4-7')).toBe(true)
		expect(anthropicModelDisallowsSamplingParams('claude-opus-4-7@20260416')).toBe(true)
		expect(anthropicModelDisallowsSamplingParams('claude-opus-4-7/thinking')).toBe(true)
	})

	it('omits deterministic temperature for Anthropic Opus 4.7 chat requests', () => {
		expect(
			getDefaultChatTemperature({ provider: 'anthropic', model: 'claude-opus-4-7' })
		).toBeUndefined()
	})

	it('keeps deterministic temperature for older Anthropic models', () => {
		expect(getDefaultChatTemperature({ provider: 'anthropic', model: 'claude-sonnet-4-6' })).toBe(0)
	})
})
