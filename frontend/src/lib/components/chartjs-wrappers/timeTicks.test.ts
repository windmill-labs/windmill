import { describe, expect, test } from 'vitest'
import type { Tick } from 'chart.js'
import { timeTicksWithDate } from './timeTicks'

const year = new Date().getFullYear()

function labels(dates: Date[]): string[] {
	const ticks = dates.map((d) => ({ value: d.getTime() }) as Tick)
	const { callback } = timeTicksWithDate(dates[0], dates[dates.length - 1])
	return ticks.map((t, i) => callback(t.value, i, ticks))
}

describe('timeTicksWithDate', () => {
	test('dates the leftmost tick and each day boundary, not the ticks in between', () => {
		expect(
			labels([
				new Date(year, 7, 21, 20, 0),
				new Date(year, 7, 21, 22, 0),
				new Date(year, 7, 22, 0, 0),
				new Date(year, 7, 22, 2, 0)
			])
		).toEqual(['Aug 21 8PM', '10PM', 'Aug 22', '2AM'])
	})

	test('sub-hour ticks keep the same AM/PM casing as the hourly ones', () => {
		expect(labels([new Date(year, 7, 21, 23, 45), new Date(year, 7, 22, 0, 15)])).toEqual([
			'Aug 21 11:45 PM',
			'Aug 22 12:15 AM'
		])
	})

	test('major ticks are enabled only once the axis spans more than one day', () => {
		const within = timeTicksWithDate(new Date(year, 7, 21, 6, 0), new Date(year, 7, 21, 23, 0))
		const across = timeTicksWithDate(new Date(year, 7, 21, 23, 0), new Date(year, 7, 22, 1, 0))

		// Majors keep day boundaries through autoSkip, but drop tick 0 — the only dated tick an
		// axis inside a single day has.
		expect(within.major.enabled).toBe(false)
		expect(across.major.enabled).toBe(true)
	})
})
