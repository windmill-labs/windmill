import { describe, expect, it } from 'vitest'
import { describeCron, describeSchedule } from './describeCron'

describe('describeCron', () => {
	it.each([
		['*/30 * * * * *', 'Every 30 seconds', 'Every 30s'],
		['0 * * * * *', 'Every minute', 'Every minute'],
		['0 */15 * * * *', 'Every 15 minutes', 'Every 15 min'],
		['0 0 * * * *', 'Every hour', 'Hourly'],
		['0 30 * * * *', 'Every hour at :30', 'Hourly at :30'],
		['0 0 */6 * * *', 'Every 6 hours', 'Every 6h'],
		['0 0 4 * * *', 'Every day at 4:00', 'Daily, 4:00'],
		// five fields are padded at the end, like the schedule editor saves them
		['0 0 12 * *', 'Every day at 12:00', 'Daily, 12:00'],
		['0 30 9 * * 1-5', 'Every weekday at 9:30', 'Weekdays, 9:30'],
		['0 0 9 * * MON', 'Every Monday at 9:00', 'Mondays, 9:00'],
		['0 0 18 * * 1,3,5', 'Every Monday, Wednesday and Friday at 18:00', 'Mon/Wed/Fri, 18:00'],
		['0 0 8 1 * *', 'Every month on the 1st at 8:00', 'Monthly on the 1st, 8:00'],
		['0 0 8 22 * *', 'Every month on the 22nd at 8:00', 'Monthly on the 22nd, 8:00']
	])('%s → %s / %s', (cron, sentence, compact) => {
		expect(describeCron(cron)).toBe(sentence)
		expect(describeCron(cron, { compact: true })).toBe(compact)
	})

	it.each(['0 0 8 1 1 *', '15 0 4 * * *', '0 0 4-6 * * *', 'not a cron'])(
		'leaves %s to be shown as is',
		(cron) => {
			expect(describeCron(cron)).toBeUndefined()
			expect(describeCron(cron, { compact: true })).toBeUndefined()
		}
	)
})

describe('describeSchedule', () => {
	it('names the zone of a wall-clock time only', () => {
		expect(describeSchedule('0 0 12 * * *', 'UTC')).toBe('Every day at 12:00 UTC')
		expect(describeSchedule('0 0 12 * * *', 'UTC', { compact: true })).toBe('Daily, 12:00 UTC')
		expect(describeSchedule('0 0 * * * *', 'UTC')).toBe('Every hour')
		expect(describeSchedule('0 30 * * * *', 'UTC', { compact: true })).toBe('Hourly at :30')
	})
})
