import { describe, it, expect } from 'vitest'
import { formatAutoDuration, linearLayout, logRange, logTicks } from './durationAxis'

const SECOND = 1_000
const MINUTE = 60 * SECOND
const HOUR = 60 * MINUTE

function autoLogLabels(minMs: number, maxMs: number, heightPx = 140) {
	const { min, max } = logRange('auto', minMs, maxMs)
	return logTicks('auto', min, max, heightPx).map(formatAutoDuration)
}

describe('duration axis', () => {
	it('labels a wide log axis on the 1-10-100 steps of each unit', () => {
		expect(autoLogLabels(27, 76 * MINUTE)).toEqual(['100ms', '1s', '10s', '1m', '10m', '1h'])
	})

	it('keeps the unit boundaries when a wide log axis is too short for every step', () => {
		expect(autoLogLabels(27, 4 * HOUR, 120)).toEqual(['100ms', '1s', '1m', '1h'])
	})

	it('fills a narrow log range with the 2 and 5 steps, and widens one with no round duration', () => {
		expect(autoLogLabels(20 * SECOND, 3 * MINUTE)).toEqual(['20s', '30s', '1m', '2m'])
		expect(autoLogLabels(300, 300).length).toBeGreaterThanOrEqual(2)
	})

	it('picks the linear unit from the longest run', () => {
		expect(linearLayout('auto', 900, 140).unit).toBe('ms')
		expect(linearLayout('auto', 119 * SECOND, 140).unit).toBe('s')
		expect(linearLayout('auto', 76 * MINUTE, 140)).toEqual({
			unit: 'min',
			stepMs: 30 * MINUTE,
			maxMs: 90 * MINUTE
		})
		expect(linearLayout('auto', 121 * MINUTE, 140).unit).toBe('h')
	})
})
