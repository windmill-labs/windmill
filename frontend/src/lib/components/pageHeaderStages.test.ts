import { describe, expect, it } from 'vitest'
import { BAR_STAGES, fitBar, type BarMeasure } from './pageHeaderStages'

// Each stage 50px narrower than the last, so the bar width alone says which one fits.
const trail = BAR_STAGES.map((_, i) => 600 - i * 50)
const base: BarMeasure = {
	barWidth: 2000,
	chrome: 0,
	trail,
	summary: BAR_STAGES.map(() => 0),
	actions: [300, 200, 100],
	folds: 2,
	natural: { family: 0, fork: 0, summary: 0, name: 0 }
}

describe('fitBar', () => {
	it('picks the first stage that fits beside the full row of actions', () => {
		expect(fitBar({ ...base, barWidth: 900 }).stage).toBe(0)
		expect(fitBar({ ...base, barWidth: 800 })).toMatchObject({ stage: 2, fold: 0 })
	})

	it('folds the actions only once the breadcrumb is at its last stage, and keeps it there', () => {
		// 350 + 300 overflows a 600px bar; 350 + 200 fits. Stage 0 would fit beside the folded row
		// too, but the parts a narrowing bar dropped do not come back when the buttons fold.
		expect(fitBar({ ...base, barWidth: 600 })).toMatchObject({ stage: 5, fold: 1 })
		expect(fitBar({ ...base, barWidth: 400 })).toMatchObject({ stage: 5, fold: 2 })
	})

	it('tries a level it has not seen at the width of the next one folded further', () => {
		// Only the second fold has been measured: the levels before it are assumed no wider, so the
		// bar tries the full row first and goes on to the first fold once it has seen it overflow.
		const unseen = { ...base, barWidth: 600, actions: [undefined, undefined, 100] }
		expect(fitBar(unseen).fold).toBe(0)
		expect(fitBar({ ...unseen, actions: [300, undefined, 100] }).fold).toBe(1)
	})

	it("takes the squeeze from the path's name before the summary", () => {
		const fit = fitBar({
			...base,
			barWidth: 360,
			actions: [0],
			folds: 0,
			trail: trail.map((w) => w - 400),
			summary: BAR_STAGES.map(() => 100),
			natural: { family: 0, fork: 0, summary: 300, name: 300 }
		})
		// Stage 0 at its floors is 300px; whole, the name and the summary add 140 and 100, so 180
		// has to go: all 140 the name has above its floor, then 40 from the summary.
		expect(fit).toMatchObject({ stage: 0, caps: { name: 160, summary: 260 }, scopesGive: false })
	})

	it('keeps the summary and drops the path when only one of them fits', () => {
		const natural = { family: 0, fork: 0, summary: 300, name: 300 }
		// Stage 2 is the first without the path: the name is no longer a part the bar sizes.
		const fit = fitBar({ ...base, barWidth: 520, natural, actions: [0], folds: 0 })
		expect(fit.stage).toBe(2)
		expect(fit.caps.name).toBeUndefined()
	})
})
