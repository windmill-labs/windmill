import { describe, expect, it } from 'vitest'
import {
	applyReasoningToConfig,
	getReasoningCapability,
	REASONING_OFF,
	resolveEffectiveReasoning,
	resolveRequestReasoning,
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
		// Opus 4.5 predates adaptive thinking — offering effort would hard-400.
		expect(supportsReasoning('anthropic', 'claude-opus-4-5')).toBe(false)
		expect(supportsReasoning('aws_bedrock', 'eu.anthropic.claude-opus-4-5-20251101-v1:0')).toBe(
			false
		)
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
	it('flags Claude models served through Bedrock, with the Anthropic ladder', () => {
		expect(supportsReasoning('aws_bedrock', 'us.anthropic.claude-opus-4-6-v1')).toBe(true)
		expect(supportsReasoning('aws_bedrock', 'anthropic.claude-sonnet-4-6-v1:0')).toBe(true)
		expect(supportsReasoning('aws_bedrock', 'amazon.nova-pro-v1:0')).toBe(false)
		expect(getReasoningCapability('aws_bedrock', 'us.anthropic.claude-opus-4-8-v1').levels).toEqual(
			['low', 'medium', 'high', 'xhigh', 'max']
		)
	})
	it('exposes the model-specific Gemini ladder', () => {
		// Gemini 3+ Flash / Flash-Lite accept minimal; Pro does not.
		expect(getReasoningCapability('googleai', 'gemini-3-flash-preview').levels).toEqual([
			'minimal',
			'low',
			'medium',
			'high'
		])
		expect(getReasoningCapability('googleai', 'gemini-3.1-flash-lite').levels).toEqual([
			'minimal',
			'low',
			'medium',
			'high'
		])
		expect(getReasoningCapability('googleai', 'gemini-3.1-pro-preview').levels).toEqual([
			'low',
			'medium',
			'high'
		])
		// 2.5 uses budget tiers — no minimal, even on flash.
		expect(getReasoningCapability('googleai', 'gemini-2.5-flash').levels).toEqual([
			'low',
			'medium',
			'high'
		])
	})
	it('flags DeepSeek models with the two effective levels, excluding the chat alias', () => {
		expect(supportsReasoning('deepseek', 'deepseek-v4-flash')).toBe(true)
		expect(supportsReasoning('deepseek', 'deepseek-v4-pro')).toBe(true)
		expect(supportsReasoning('deepseek', 'deepseek-reasoner')).toBe(true)
		// `deepseek-chat` is the documented non-thinking mode — no knob.
		expect(supportsReasoning('deepseek', 'deepseek-chat')).toBe(false)
		// Only high/max are real; low/medium/xhigh are server-side aliases.
		expect(getReasoningCapability('deepseek', 'deepseek-v4-flash').levels).toEqual(['high', 'max'])
	})
	it('flags OpenAI reasoning families, not gpt-4o', () => {
		expect(supportsReasoning('openai', 'gpt-5')).toBe(true)
		expect(supportsReasoning('openai', 'o3')).toBe(true)
		expect(supportsReasoning('openai', 'gpt-4o')).toBe(false)
	})
	it('exposes the model-specific OpenAI ladder', () => {
		// gpt-5 has minimal; gpt-5.1+ dropped it; only gpt-5.5 adds xhigh.
		expect(getReasoningCapability('openai', 'gpt-5').levels).toEqual([
			'minimal',
			'low',
			'medium',
			'high'
		])
		expect(getReasoningCapability('openai', 'gpt-5.1').levels).toEqual(['low', 'medium', 'high'])
		expect(getReasoningCapability('openai', 'gpt-5.5').levels).toEqual([
			'low',
			'medium',
			'high',
			'xhigh'
		])
		expect(getReasoningCapability('openai', 'o3').levels).toEqual(['low', 'medium', 'high'])
	})
	it('flags Mistral models verified to accept reasoning_effort, with the on/off ladder', () => {
		expect(supportsReasoning('mistral', 'mistral-medium-3-5')).toBe(true)
		expect(supportsReasoning('mistral', 'mistral-medium-latest')).toBe(true)
		expect(supportsReasoning('mistral', 'mistral-small-latest')).toBe(true)
		// These reject the param ("reasoning_effort is not enabled for this model").
		expect(supportsReasoning('mistral', 'mistral-large-latest')).toBe(false)
		expect(supportsReasoning('mistral', 'magistral-medium-latest')).toBe(false)
		expect(supportsReasoning('mistral', 'ministral-8b-latest')).toBe(false)
		// 'high' is the only accepted effort token; off = omit the field.
		expect(getReasoningCapability('mistral', 'mistral-medium-3-5').levels).toEqual(['high'])
		expect(getReasoningCapability('mistral', 'mistral-medium-3-5').canDisable).toBe(true)
	})
	it('returns no levels for providers without a registry entry', () => {
		expect(getReasoningCapability('mistral', 'codestral-latest')).toEqual({
			supported: false,
			levels: [],
			canDisable: false
		})
	})
	it('only offers off where the model can truly disable thinking', () => {
		// Gemini Pro enforces a thinking floor — no off option.
		expect(getReasoningCapability('googleai', 'gemini-2.5-pro').canDisable).toBe(false)
		expect(getReasoningCapability('googleai', 'gemini-3.1-pro-preview').canDisable).toBe(false)
		// Flash / Flash-Lite can fully disable (budget 0 / minimal).
		expect(getReasoningCapability('googleai', 'gemini-2.5-flash').canDisable).toBe(true)
		expect(getReasoningCapability('googleai', 'gemini-3-flash-preview').canDisable).toBe(true)
		// Claude doesn't think unless asked; DeepSeek has a disable param.
		expect(getReasoningCapability('anthropic', 'claude-sonnet-4-6').canDisable).toBe(true)
		expect(getReasoningCapability('deepseek', 'deepseek-v4-flash').canDisable).toBe(true)
		// Fable thinking is always on — explicit disable 400s, omission is a no-op.
		expect(getReasoningCapability('anthropic', 'claude-fable-5').canDisable).toBe(false)
		expect(
			getReasoningCapability('aws_bedrock', 'eu.anthropic.claude-fable-5-v1:0').canDisable
		).toBe(false)
		// ...but the effort ladder is fully available on Fable.
		expect(getReasoningCapability('anthropic', 'claude-fable-5').levels).toEqual([
			'low',
			'medium',
			'high',
			'xhigh',
			'max'
		])
		// gpt-5.1+ accept effort 'none'; gpt-5 and o-series reason at medium
		// by default and reject 'none' — omission isn't off.
		expect(getReasoningCapability('openai', 'gpt-5.1').canDisable).toBe(true)
		expect(getReasoningCapability('openai', 'gpt-5.5').canDisable).toBe(true)
		expect(getReasoningCapability('openai', 'gpt-5').canDisable).toBe(false)
		expect(getReasoningCapability('openai', 'o3').canDisable).toBe(false)
		// OpenRouter's 'none' effort disables reasoning, but only where the
		// underlying model can actually stop thinking — same family scoping
		// as the levels.
		expect(getReasoningCapability('openrouter', 'anthropic/claude-sonnet-4.6').canDisable).toBe(
			true
		)
		expect(getReasoningCapability('openrouter', 'google/gemini-2.5-flash').canDisable).toBe(true)
		expect(getReasoningCapability('openrouter', 'deepseek/deepseek-v4-flash').canDisable).toBe(true)
		expect(getReasoningCapability('openrouter', 'openai/gpt-5.1').canDisable).toBe(true)
		expect(getReasoningCapability('openrouter', 'google/gemini-2.5-pro').canDisable).toBe(false)
		expect(getReasoningCapability('openrouter', 'google/gemini-3.1-pro-preview').canDisable).toBe(
			false
		)
		expect(getReasoningCapability('openrouter', 'openai/o3').canDisable).toBe(false)
		expect(getReasoningCapability('openrouter', 'openai/gpt-5-mini').canDisable).toBe(false)
		expect(getReasoningCapability('openrouter', 'x-ai/grok-4').canDisable).toBe(false)
		expect(getReasoningCapability('openrouter', 'deepseek/deepseek-r1').canDisable).toBe(false)
	})
	it('forwards an explicit off as effort none through OpenRouter', () => {
		expect(
			resolveRequestReasoning({
				provider: 'openrouter',
				model: 'google/gemini-2.5-flash',
				reasoning: REASONING_OFF
			})
		).toBe('none')
	})
	it('flags deepseek v4 models served through OpenRouter', () => {
		expect(supportsReasoning('openrouter', 'deepseek/deepseek-v4-flash')).toBe(true)
		expect(supportsReasoning('openrouter', 'deepseek/deepseek-r1')).toBe(true)
	})
	it('scopes the OpenRouter ladder to the underlying model family', () => {
		// Anthropic: all five levels are distinct budget ratios.
		expect(getReasoningCapability('openrouter', 'anthropic/claude-sonnet-4.6').levels).toEqual([
			'minimal',
			'low',
			'medium',
			'high',
			'xhigh'
		])
		// Gemini: thinkingLevel mapping; xhigh is clamped to high, so not offered.
		expect(getReasoningCapability('openrouter', 'google/gemini-3-flash-preview').levels).toEqual([
			'minimal',
			'low',
			'medium',
			'high'
		])
		expect(getReasoningCapability('openrouter', 'google/gemini-2.5-flash').levels).toEqual([
			'low',
			'medium',
			'high'
		])
		// OpenAI: passed through verbatim — same per-model scoping as direct.
		expect(getReasoningCapability('openrouter', 'openai/gpt-5-mini').levels).toEqual([
			'minimal',
			'low',
			'medium',
			'high'
		])
		expect(getReasoningCapability('openrouter', 'openai/gpt-5.5').levels).toEqual([
			'low',
			'medium',
			'high',
			'xhigh'
		])
		// DeepSeek: low/medium alias high and xhigh aliases max server-side.
		expect(getReasoningCapability('openrouter', 'deepseek/deepseek-v4-flash').levels).toEqual([
			'high',
			'xhigh'
		])
		// Unknown families keep the conservative common denominator.
		expect(getReasoningCapability('openrouter', 'x-ai/grok-4').levels).toEqual([
			'low',
			'medium',
			'high'
		])
	})
})

describe('Azure AI Foundry reasoning follows the model family', () => {
	it('treats Foundry Claude deployments like the Anthropic provider', () => {
		// Live-verified: Foundry Claude accepts the adaptive-thinking effort ladder.
		expect(supportsReasoning('azure_foundry', 'claude-sonnet-5')).toBe(true)
		expect(supportsReasoning('azure_foundry', 'claude-opus-4-8')).toBe(true)
		expect(getReasoningCapability('azure_foundry', 'claude-opus-4-8').levels).toEqual([
			'low',
			'medium',
			'high',
			'xhigh',
			'max'
		])
		// Off is achieved by omission (Foundry rejects effort 'none'), like Anthropic.
		expect(getReasoningCapability('azure_foundry', 'claude-sonnet-5').canDisable).toBe(true)
		expect(
			resolveRequestReasoning({
				provider: 'azure_foundry',
				model: 'claude-sonnet-5',
				reasoning: REASONING_OFF
			})
		).toBeUndefined()
	})

	it('treats Foundry OpenAI deployments like the OpenAI provider', () => {
		expect(supportsReasoning('azure_foundry', 'gpt-5.1')).toBe(true)
		expect(supportsReasoning('azure_foundry', 'gpt-4o')).toBe(false)
		expect(supportsReasoning('azure_foundry', 'DeepSeek-R1')).toBe(false)
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

describe('resolveRequestReasoning', () => {
	it('matches resolveEffectiveReasoning for levels and defaults', () => {
		expect(resolveRequestReasoning({ provider: 'googleai', model: 'gemini-2.5-pro' })).toBe('high')
		expect(resolveRequestReasoning({ provider: 'openai', model: 'o3', reasoning: 'low' })).toBe(
			'low'
		)
	})
	it('forwards an explicit off as the provider disable token on reasoning-by-default providers', () => {
		expect(
			resolveRequestReasoning({
				provider: 'googleai',
				model: 'gemini-2.5-pro',
				reasoning: REASONING_OFF
			})
		).toBe('none')
	})
	it('forwards an explicit off for DeepSeek (thinks by default)', () => {
		expect(
			resolveRequestReasoning({
				provider: 'deepseek',
				model: 'deepseek-v4-flash',
				reasoning: REASONING_OFF
			})
		).toBe('none')
	})
	it('forwards off as effort none on gpt-5.1+, but not on older OpenAI reasoning models', () => {
		expect(
			resolveRequestReasoning({ provider: 'openai', model: 'gpt-5.1', reasoning: REASONING_OFF })
		).toBe('none')
		expect(
			resolveRequestReasoning({ provider: 'openai', model: 'o3', reasoning: REASONING_OFF })
		).toBeUndefined()
	})
	it('keeps off as undefined for providers without default-on reasoning', () => {
		expect(
			resolveRequestReasoning({
				provider: 'anthropic',
				model: 'claude-sonnet-4-6',
				reasoning: REASONING_OFF
			})
		).toBeUndefined()
	})
	it('never sends a disable token for non-capable models', () => {
		expect(
			resolveRequestReasoning({
				provider: 'googleai',
				model: 'gemini-2.0-flash',
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
	it('adds reasoning_effort on the deepseek path for a level', () => {
		expect(applyReasoningToConfig({ model: 'm' }, 'deepseek', 'max')).toMatchObject({
			reasoning_effort: 'max'
		})
	})
	it('translates the deepseek off sentinel to the thinking-disabled param', () => {
		const out = applyReasoningToConfig({ model: 'm' }, 'deepseek', 'none') as Record<string, any>
		expect(out.thinking).toEqual({ type: 'disabled' })
		expect(out.reasoning_effort).toBeUndefined()
	})
	it('strips sampling params on the mistral path when reasoning is on', () => {
		const out = applyReasoningToConfig(
			{ model: 'm', temperature: 0, max_tokens: 10 },
			'mistral',
			'high'
		) as Record<string, any>
		expect(out.reasoning_effort).toBe('high')
		expect(out.temperature).toBeUndefined()
		expect(out.max_tokens).toBe(10)
		// No effort -> config untouched, temperature kept.
		expect(
			applyReasoningToConfig({ model: 'm', temperature: 0 }, 'mistral', undefined)
		).toMatchObject({ temperature: 0 })
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
