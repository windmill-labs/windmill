import { describe, expect, it } from 'vitest'
import { billedTokens } from './chat/tokenUsage'
import { estimateCost, priceSpend, resolveModelPrice } from './modelPricing'

describe('resolveModelPrice', () => {
	it('resolves the same model across the routes that decorate its id', () => {
		const direct = resolveModelPrice('anthropic', 'claude-opus-5', undefined)
		expect(direct?.price.input).toBe(5)
		// A gateway prefix, a dot-versioned id, a date suffix and a variant suffix
		// must all land on the same entry — a miss here silently under-reports cost.
		for (const id of [
			'anthropic/claude-opus-5',
			'anthropic/claude-opus-4.8',
			'claude-opus-4-8-20260101',
			'anthropic/claude-opus-5:thinking'
		]) {
			expect(resolveModelPrice('openrouter', id, undefined)?.price.input).toBe(5)
		}
	})

	it('does not let a version-digit entry claim a longer version', () => {
		expect(resolveModelPrice('openai', 'gpt-4.1', undefined)?.price.input).toBe(2)
		expect(resolveModelPrice('openai', 'gpt-4-1106-preview', undefined)?.price.input).not.toBe(2)
	})

	it('prices flat-rate Gemini Flash while leaving the tiered Pro alone', () => {
		expect(resolveModelPrice('googleai', 'gemini-2.5-flash', undefined)?.price.input).toBe(0.3)
		expect(resolveModelPrice('googleai', 'gemini-2.5-flash-lite', undefined)?.price.input).toBe(0.1)
		expect(resolveModelPrice('googleai', 'gemini-3.5-flash', undefined)?.price.output).toBe(9)
		// Pro charges roughly double above a 200k prompt, which a per-model rate cannot
		// express, so it must stay unpriced rather than be estimated at the low tier.
		expect(resolveModelPrice('googleai', 'gemini-2.5-pro', undefined)).toBeUndefined()
		expect(resolveModelPrice('googleai', 'gemini-3.1-pro', undefined)).toBeUndefined()
		// Promotional rates carry an end date a timeless table cannot represent.
		expect(resolveModelPrice('googleai', 'gemini-3.7-flash', undefined)).toBeUndefined()
	})

	it('reports an unknown model as unpriced rather than guessing', () => {
		expect(resolveModelPrice('customai', 'some-in-house-model', undefined)).toBeUndefined()
	})

	it('does not let another model inherit a price through a shared prefix', () => {
		// A sub-model (`-pro`) or a newer revision (`gpt-5.6` → `gpt-5-6`) is a
		// different model at a different rate; inheriting `gpt-5`'s would be off by
		// an order of magnitude, and silently so.
		expect(resolveModelPrice('openai', 'gpt-5', undefined)?.price.input).toBe(1.25)
		expect(resolveModelPrice('openai', 'gpt-5-mini', undefined)?.price.input).toBe(0.25)
		expect(resolveModelPrice('openai', 'gpt-5-pro', undefined)).toBeUndefined()
		expect(resolveModelPrice('openai', 'gpt-5.6', undefined)).toBeUndefined()
		expect(resolveModelPrice('googleai', 'gemini-3.1', undefined)).toBeUndefined()
		// A revision carrying a variant has to be caught by the matcher, not by an
		// explicit entry: `gpt-5.4-mini` cannot match the `gpt-5.4` one (the `-mini`
		// makes it a sub-model), so nothing but the guard stops it reaching `gpt-5`.
		expect(resolveModelPrice('openai', 'gpt-5.4-mini', undefined)).toBeUndefined()
		expect(resolveModelPrice('openai', 'gpt-5.5-pro', undefined)).toBeUndefined()
	})

	it('still resolves the route decorations that name the same model', () => {
		// Dates, Bedrock's -v1 and floating aliases are ways of spelling one model,
		// not sub-models. `claude-3-5-haiku-latest` is a shipped picker default, so
		// unpricing it would silently disable cost tracking out of the box.
		expect(resolveModelPrice('anthropic', 'claude-opus-4-5-20251101', undefined)?.price.input).toBe(5)
		// The revision guard must not swallow a date, which is digits too.
		expect(resolveModelPrice('openai', 'gpt-5-2026-01-01', undefined)?.price.input).toBe(1.25)
		expect(
			resolveModelPrice('bedrock', 'anthropic.claude-sonnet-4-6-20250101-v1:0', undefined)?.price
				.input
		).toBe(3)
		expect(resolveModelPrice('anthropic', 'claude-3-5-haiku-latest', undefined)?.price.input).toBe(
			0.8
		)
		// …while a genuine sub-model stays unpriced, including one hiding behind a
		// decoration.
		expect(resolveModelPrice('openai', 'gpt-5-pro', undefined)).toBeUndefined()
		expect(resolveModelPrice('openai', 'gpt-5-preview-pro', undefined)).toBeUndefined()
		// A family fallback must not price a model the table deliberately left out,
		// nor the floating alias pointing at it.
		expect(resolveModelPrice('anthropic', 'claude-sonnet-5', undefined)).toBeUndefined()
		expect(
			resolveModelPrice('openrouter', '~anthropic/claude-sonnet-latest', undefined)
		).toBeUndefined()
	})

	it('prefers a workspace override, keeping the model’s own cache ratios', () => {
		const resolved = resolveModelPrice('anthropic', 'claude-opus-5', {
			'anthropic:claude-opus-5': { input: 2, output: 8 }
		})
		expect(resolved?.source).toBe('override')
		expect(resolved?.price.input).toBe(2)
		// Anthropic reads a cached prefix at a tenth and writes at 1.25x.
		expect(resolved?.price.cacheRead).toBeCloseTo(0.2)
		expect(resolved?.price.cacheWrite).toBeCloseTo(2.5)
	})

	it('applies the overridden model’s own cache discount, not Anthropic’s', () => {
		// gpt-4o discounts a cached read by half, not by a tenth — an override that
		// only states input/output must not silently inherit the Anthropic ratio.
		const resolved = resolveModelPrice('openai', 'gpt-4o', {
			'openai:gpt-4o': { input: 2, output: 8 }
		})
		expect(resolved?.price.cacheRead).toBeCloseTo(1)
	})

	it('bills an unpriced model’s cached tokens at its input rate', () => {
		// Gemini Pro is deliberately unpriced, so there is no ratio to inherit. Falling
		// back to Anthropic's tenth would invent a discount the provider may not give;
		// the admin states the cache rates explicitly or pays full input.
		const resolved = resolveModelPrice('googleai', 'gemini-2.5-pro', {
			'googleai:gemini-2.5-pro': { input: 2, output: 8 }
		})
		expect(resolved?.price.cacheRead).toBe(2)
		expect(resolved?.price.cacheWrite).toBe(2)

		const stated = resolveModelPrice('googleai', 'gemini-2.5-pro', {
			'googleai:gemini-2.5-pro': { input: 2, output: 8, cache_read: 0.5, cache_write: 1 }
		})
		expect(stated?.price.cacheRead).toBe(0.5)
		expect(stated?.price.cacheWrite).toBe(1)
	})

	it('ignores an override whose rates could not be a price', () => {
		for (const bad of [{ input: -1, output: 8 }, { input: 1e9, output: 8 }]) {
			const resolved = resolveModelPrice('anthropic', 'claude-opus-5', {
				'anthropic:claude-opus-5': bad
			})
			expect(resolved?.source).toBe('builtin')
		}
	})
})

describe('estimateCost', () => {
	it('bills each token class at its own rate', () => {
		const cost = estimateCost(
			{ input: 1_000_000, cacheRead: 1_000_000, cacheWrite: 1_000_000, output: 1_000_000 },
			{ input: 5, output: 25, cacheRead: 0.5, cacheWrite: 6.25 }
		)
		expect(cost).toBeCloseTo(5 + 0.5 + 6.25 + 25)
	})

	it('charges a cached prefix less than an uncached one', () => {
		const usage = {
			prompt: 100_000,
			completion: 0,
			total: 100_000,
			cacheRead: 90_000,
			cacheWrite: 0
		}
		const uncached = { ...usage, cacheRead: 0 }
		const price = { input: 5, output: 25, cacheRead: 0.5, cacheWrite: 6.25 }
		expect(estimateCost(billedTokens(usage), price)).toBeLessThan(
			estimateCost(billedTokens(uncached), price)
		)
	})
})

describe('priceSpend', () => {
	it('prefers a provider-reported cost over the estimate', () => {
		const priced = priceSpend(
			[
				{
					provider: 'openrouter',
					model: 'anthropic/claude-opus-5',
					tokens: { input: 1_000_000, cacheRead: 0, cacheWrite: 0, output: 0 },
					reportedCostUsd: 0.42
				}
			],
			undefined
		)
		expect(priced.total).toBe(0.42)
		expect(priced.hasReported).toBe(true)
	})

	it('flags an unpriced model instead of counting it as free', () => {
		const priced = priceSpend(
			[
				{
					provider: 'customai',
					model: 'some-in-house-model',
					tokens: { input: 1_000_000, cacheRead: 0, cacheWrite: 0, output: 0 }
				}
			],
			undefined
		)
		expect(priced.hasUnpriced).toBe(true)
		expect(priced.rows[0].cost).toBeUndefined()
	})
})
