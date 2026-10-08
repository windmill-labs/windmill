import { describe, expect, it } from 'vitest'
import { describeCron, describeSchedule } from './describeCron'

describe('describeCron', () => {
	it.each([
		['*/30 * * * * *', 'Every 30s'],
		['0 * * * * *', 'Every minute'],
		['0 */15 * * * *', 'Every 15 min'],
		['0 0 * * * *', 'Hourly'],
		['0 30 * * * *', 'Hourly at :30'],
		['0 0 */6 * * *', 'Every 6h'],
		['0 0 4 * * *', 'Daily, 4:00'],
		// five fields are padded at the end, like the schedule editor saves them
		['0 0 12 * *', 'Daily, 12:00'],
		['0 30 9 * * 1-5', 'Weekdays, 9:30'],
		['0 0 9 * * MON', 'Mondays, 9:00'],
		['0 0 18 * * 1,3,5', 'Mon/Wed/Fri, 18:00'],
		['0 0 8 1 * *', 'Monthly on the 1st, 8:00'],
		['0 0 8 22 * *', 'Monthly on the 22nd, 8:00']
	])('%s → %s', (cron, text) => {
		expect(describeCron(cron)).toBe(text)
	})

	it.each(['0 0 8 1 1 *', '15 0 4 * * *', '0 0 4-6 * * *', 'not a cron'])(
		'leaves %s to be shown as is',
		(cron) => {
			expect(describeCron(cron)).toBeUndefined()
		}
	)
})

describe('describeSchedule', () => {
	it('names the zone of a wall-clock time only', () => {
		expect(describeSchedule('0 0 12 * * *', 'UTC')).toBe('Daily, 12:00 UTC')
		expect(describeSchedule('0 0 * * * *', 'UTC')).toBe('Hourly')
	})
})
