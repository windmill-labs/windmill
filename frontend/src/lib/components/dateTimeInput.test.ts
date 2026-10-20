import { expect, it } from 'vitest'
import { parseDateTime } from './dateTimeInput'

it('reads a zone-less naive value as its digits, outside UTC too', () => {
	const tz = process.env.TZ
	process.env.TZ = 'Europe/Paris'
	try {
		expect(parseDateTime('2027-09-09T10:00', 'naive').getUTCHours()).toBe(10)
		expect(parseDateTime('2027-09-09T10:00', 'local').getUTCHours()).toBe(8)
	} finally {
		if (tz === undefined) delete process.env.TZ
		else process.env.TZ = tz
	}
})
