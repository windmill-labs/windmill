import { describe, it, expect } from 'vitest'
import { byPopularity } from './pickerPopularity'

const order = (names: string[], hub: Record<string, number>, local: Record<string, number> = {}) =>
	[...names].sort(byPopularity(hub, local))

describe('byPopularity', () => {
	it('ranks hub picks above local usage', () => {
		expect(order(['slack', 'stripe'], { slack: 1 }, { stripe: 40 })).toEqual(['slack', 'stripe'])
	})

	it('breaks a hub tie on local usage', () => {
		expect(order(['slack', 'stripe'], { slack: 5, stripe: 5 }, { stripe: 2 })).toEqual([
			'stripe',
			'slack'
		])
	})

	it('falls back to alphabetical for everything neither signal ranks', () => {
		expect(order(['stripe', 'ably', 'github'], { github: 3 })).toEqual(['github', 'ably', 'stripe'])
	})

	it('orders on local usage alone when the hub ranks nothing', () => {
		expect(order(['stripe', 'ably', 'github'], {}, { stripe: 1 })).toEqual([
			'stripe',
			'ably',
			'github'
		])
	})
})
