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

	it('reports an unknown model as unpriced rather than guessing', () => {
		expect(resolveModelPrice('customai', 'some-in-house-model', undefined)).toBeUndefined()
	})

	it('prefers a workspace override, defaulting its cache rates off its own input rate', () => {
		const resolved = resolveModelPrice('anthropic', 'claude-opus-5', {
			'anthropic:claude-opus-5': { input: 2, output: 8 }
		})
		expect(resolved?.source).toBe('override')
		expect(resolved?.price.input).toBe(2)
		expect(resolved?.price.cacheRead).toBeCloseTo(0.2)
		expect(resolved?.price.cacheWrite).toBeCloseTo(2.5)
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
