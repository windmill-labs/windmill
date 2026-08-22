import { describe, expect, test } from 'vitest'
import type { Tick } from 'chart.js'
import { timeTicksWithDate } from './timeTicks'

describe('timeTicksWithDate', () => {
	test('dates the leftmost tick and each day boundary, not the ticks in between', () => {
		const year = new Date().getFullYear()
		const ticks = [
			new Date(year, 7, 21, 20, 0),
			new Date(year, 7, 21, 22, 0),
			new Date(year, 7, 22, 0, 0),
			new Date(year, 7, 22, 2, 0)
		].map((d) => ({ value: d.getTime() }) as Tick)

		expect(ticks.map((t, i) => timeTicksWithDate.callback(t.value, i, ticks))).toEqual([
			'Aug 21 8PM',
			'10PM',
			'Aug 22',
			'2AM'
		])
	})
})
