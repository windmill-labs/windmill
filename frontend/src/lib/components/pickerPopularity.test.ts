import { describe, it, expect } from 'vitest'
import { alphabetical, byPopularity } from './pickerPopularity'

const order = (names: string[], hub: Record<string, number>, local: Record<string, number> = {}) =>
	[...names].sort(byPopularity(hub, local))

describe('byPopularity', () => {
	// The tier that stops a filling-up hub from squeezing the workspace's own stack out of
	// the ordering: a global pick count grows without bound, a local one does not.
	it('leads with what the workspace uses, whatever the hub says', () => {
		expect(order(['slack', 'stripe'], { slack: 900 }, { stripe: 1 })).toEqual(['stripe', 'slack'])
	})

	it('ranks the used types among themselves by hub picks', () => {
		expect(order(['slack', 'stripe'], { slack: 900 }, { slack: 1, stripe: 1 })).toEqual([
			'slack',
			'stripe'
		])
	})

	it('breaks a hub tie on how much the workspace uses it', () => {
		expect(order(['slack', 'stripe'], { slack: 5, stripe: 5 }, { slack: 1, stripe: 2 })).toEqual([
			'stripe',
			'slack'
		])
	})

	it('ranks the unused types by hub picks, below every used one', () => {
		expect(order(['ably', 'github', 'stripe'], { ably: 900, github: 5 }, { stripe: 1 })).toEqual([
			'stripe',
			'ably',
			'github'
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

	// The lists render before either signal lands, and one of them arrives in a server-side
	// HashMap's iteration order, so the resting comparator has to sort rather than no-op.
	it('leaves an alphabetical order with no signal at all', () => {
		expect(['stripe', 'ably', 'github'].sort(alphabetical)).toEqual(['ably', 'github', 'stripe'])
	})
})
