import { describe, expect, it } from 'vitest'
import { getDefaultChatTemperature, modelDisallowsSamplingParams } from './modelConfig'

describe('modelConfig', () => {
	it('flags Opus 4.7 model IDs via includes matching', () => {
		expect(modelDisallowsSamplingParams('claude-opus-4-7')).toBe(true)
		expect(modelDisallowsSamplingParams('claude-opus-4-7@20260416')).toBe(true)
		expect(modelDisallowsSamplingParams('claude-opus-4-7/thinking')).toBe(true)
		expect(modelDisallowsSamplingParams('anthropic/claude-opus-4-7')).toBe(true)
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
})
