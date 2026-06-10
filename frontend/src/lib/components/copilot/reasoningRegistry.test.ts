import { describe, expect, it } from 'vitest'
import {
	applyReasoningToConfig,
	getReasoningCapability,
	REASONING_OFF,
	resolveEffectiveReasoning,
	setDynamicReasoningCapability,
	stripLegacyThinkingSuffix,
	supportsReasoning
} from './reasoningRegistry'

describe('stripLegacyThinkingSuffix', () => {
	it('removes the deprecated /thinking suffix', () => {
		expect(stripLegacyThinkingSuffix('claude-sonnet-4-6/thinking')).toBe('claude-sonnet-4-6')
	})
	it('leaves non-suffixed models untouched', () => {
		expect(stripLegacyThinkingSuffix('claude-sonnet-4-6')).toBe('claude-sonnet-4-6')
		expect(stripLegacyThinkingSuffix('anthropic/claude-opus-4-8')).toBe('anthropic/claude-opus-4-8')
	})
})

describe('supportsReasoning (static registry)', () => {
	it('flags reasoning-capable Anthropic models', () => {
		expect(supportsReasoning('anthropic', 'claude-sonnet-4-6')).toBe(true)
		expect(supportsReasoning('anthropic', 'claude-opus-4-8')).toBe(true)
		// legacy /thinking suffix still resolves to the capable base model
		expect(supportsReasoning('anthropic', 'claude-sonnet-4-6/thinking')).toBe(true)
	})
	it('excludes non-reasoning Anthropic models', () => {
		expect(supportsReasoning('anthropic', 'claude-3-5-haiku-latest')).toBe(false)
	})
	it('exposes the model-specific Anthropic effort ladder', () => {
		// Sonnet 4.6 / Opus 4.6: max but no xhigh
		expect(getReasoningCapability('anthropic', 'claude-sonnet-4-6').levels).toEqual([
			'low',
			'medium',
			'high',
			'max'
		])
		// Opus 4.7/4.8 / Fable: adds xhigh
		expect(getReasoningCapability('anthropic', 'claude-opus-4-8').levels).toEqual([
			'low',
			'medium',
			'high',
			'xhigh',
			'max'
		])
	})
	it('flags OpenAI reasoning families, not gpt-4o', () => {
		expect(supportsReasoning('openai', 'gpt-5')).toBe(true)
		expect(supportsReasoning('openai', 'o3')).toBe(true)
		expect(supportsReasoning('openai', 'gpt-4o')).toBe(false)
	})
	it('returns no levels for providers without a registry entry', () => {
		expect(getReasoningCapability('mistral', 'codestral-latest')).toEqual({
			supported: false,
			levels: []
		})
	})
})

describe('dynamic capability overlay', () => {
	it('overrides the static registry for a specific model', () => {
		setDynamicReasoningCapability('openrouter', 'some/custom-model', {
			supported: true,
			levels: ['low', 'high']
		})
		expect(getReasoningCapability('openrouter', 'some/custom-model')).toEqual({
			supported: true,
			levels: ['low', 'high']
		})
	})
})

describe('resolveEffectiveReasoning', () => {
	it('defaults capable models to high when unset', () => {
		expect(resolveEffectiveReasoning({ provider: 'anthropic', model: 'claude-sonnet-4-6' })).toBe(
			'high'
		)
	})
	it('returns undefined for non-capable models', () => {
		expect(
			resolveEffectiveReasoning({ provider: 'anthropic', model: 'claude-3-5-haiku-latest' })
		).toBeUndefined()
	})
	it('honours an explicit level', () => {
		expect(
			resolveEffectiveReasoning({
				provider: 'openai',
				model: 'o3',
				reasoning: 'low'
			})
		).toBe('low')
	})
	it('treats the off sentinel as no reasoning', () => {
		expect(
			resolveEffectiveReasoning({
				provider: 'anthropic',
				model: 'claude-sonnet-4-6',
				reasoning: REASONING_OFF
			})
		).toBeUndefined()
	})
})

describe('applyReasoningToConfig', () => {
	it('is a no-op when no effort is provided', () => {
		const config = { model: 'm', max_tokens: 10 }
		expect(applyReasoningToConfig(config, 'completions', undefined)).toBe(config)
	})
	it('adds reasoning_effort on the completions path', () => {
		expect(applyReasoningToConfig({ model: 'm' }, 'completions', 'high')).toMatchObject({
			model: 'm',
			reasoning_effort: 'high'
		})
	})
	it('nests effort under reasoning on the responses path', () => {
		expect(applyReasoningToConfig({ model: 'm' }, 'responses', 'medium')).toMatchObject({
			reasoning: { effort: 'medium' }
		})
	})
	it('adds adaptive thinking + output_config and strips sampling params for Anthropic', () => {
		const out = applyReasoningToConfig(
			{ model: 'm', max_tokens: 10, temperature: 0 },
			'anthropic',
			'high'
		) as Record<string, any>
		expect(out.output_config).toEqual({ effort: 'high' })
		expect(out.thinking).toEqual({ type: 'adaptive', display: 'summarized' })
		expect(out.temperature).toBeUndefined()
		expect(out.max_tokens).toBe(10)
	})
})
