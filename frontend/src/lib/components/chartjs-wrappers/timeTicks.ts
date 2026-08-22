import type { Tick } from 'chart.js'
import { format, isSameDay, startOfDay } from 'date-fns'

const SECOND = 1000
const MINUTE = 60 * SECOND
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

function dateFormat(time: number): string {
	return new Date(time).getFullYear() === new Date().getFullYear() ? 'MMM d' : 'MMM d, yyyy'
}

function clockFormat(spacing: number): string {
	if (spacing < SECOND) return 'h:mm:ss.SSS aaaa'
	if (spacing < MINUTE) return 'h:mm:ss aaaa'
	if (spacing < HOUR) return 'h:mm aaaa'
	return 'ha'
}

/**
 * Ticks for a chart.js time axis that stay unambiguous about the day: sub-day ticks are bare
 * clock times ("6PM"), except the leftmost one and the first tick of each day, which spell out
 * the date. Without it a range that never reaches a day boundary carries no date at all.
 */
export const timeTicksWithDate = {
	maxRotation: 0,
	minRotation: 0,
	// The dates ride on the day-boundary ticks, so autoSkip must keep those.
	major: { enabled: true },
	callback(value: number | string, index: number, ticks: Tick[]): string {
		const time = Number(value)
		// chart.js also calls this with a lone tick to size a sample label, hence the fallback.
		const spacing = ticks.length > 1 ? Math.abs(ticks[1].value - ticks[0].value) : HOUR
		if (spacing >= 27 * DAY) return format(time, 'MMM yyyy')
		const date = format(time, dateFormat(time))
		if (spacing >= DAY) return date
		const clock = format(time, clockFormat(spacing))
		if (index > 0 && isSameDay(ticks[index - 1].value, time)) return clock
		return time === startOfDay(time).getTime() ? date : `${date} ${clock}`
	}
}
