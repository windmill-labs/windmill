import { describe, it, expect } from 'vitest'
import { accentStylesheet } from './accentColor'

describe('accentStylesheet', () => {
	it('refuses anything but #rrggbb, since the value lands in a stylesheet', () => {
		for (const bad of ['#000;}body{display:none', '#fff', 'red', '1f9d55', 123, null]) {
			expect(accentStylesheet(bad)).toBeUndefined()
		}
	})

	it('overrides every accent token with an in-range r g b triplet in each theme', () => {
		// A saturated color exercises the gamut clamp, a grey the zero-chroma path.
		for (const color of ['#ff0000', '#808080']) {
			const rules = accentStylesheet(color)!.split('\n')
			expect(rules).toHaveLength(3)
			for (const rule of rules) {
				const triplets = [...rule.matchAll(/--color-[a-z-]+: (\d+) (\d+) (\d+);/g)]
				expect(triplets).toHaveLength(10)
				for (const [, ...channels] of triplets) {
					for (const c of channels) expect(Number(c)).toBeLessThanOrEqual(255)
				}
			}
		}
	})
})
