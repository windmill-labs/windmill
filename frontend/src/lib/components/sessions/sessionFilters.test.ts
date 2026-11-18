import { describe, it, expect } from 'vitest'
import { dateBucket } from './sessionFilters'

// Buckets are anchored on local midnight, so the boundaries are the part that
// can silently drift: a timestamp must land in exactly one bucket, and the one
// the label claims.
describe('dateBucket', () => {
	const now = new Date(2026, 7, 19, 14, 30).getTime()
	const startOfToday = new Date(2026, 7, 19, 0, 0, 0, 0).getTime()
	const day = 24 * 60 * 60 * 1000

	it('splits today from yesterday at local midnight', () => {
		expect(dateBucket(startOfToday, now).key).toBe('today')
		expect(dateBucket(startOfToday - 1, now).key).toBe('yesterday')
	})

	it('closes each window on its own boundary', () => {
		expect(dateBucket(startOfToday - day, now).key).toBe('yesterday')
		expect(dateBucket(startOfToday - day - 1, now).key).toBe('week')
		expect(dateBucket(startOfToday - 7 * day, now).key).toBe('week')
		expect(dateBucket(startOfToday - 7 * day - 1, now).key).toBe('month')
		expect(dateBucket(startOfToday - 30 * day, now).key).toBe('month')
		expect(dateBucket(startOfToday - 30 * day - 1, now).key).toBe('older')
	})

	// A local day is 23h long across the spring-forward transition, so boundaries
	// derived by subtracting a fixed 24h land an hour before midnight and pull the
	// first hour of a day into the older bucket.
	it('keeps boundaries on local midnight across a DST change', () => {
		const original = process.env.TZ
		process.env.TZ = 'Europe/Paris'
		try {
			// Paris springs forward at 02:00 on 2026-03-29, so from the 30th a naive
			// -24h lands at 23:00 on the 28th instead of midnight on the 29th.
			const dstNow = new Date(2026, 2, 30, 12, 0).getTime()
			const startOfYesterday = new Date(2026, 2, 29, 0, 0, 0, 0).getTime()
			expect(dateBucket(startOfYesterday, dstNow).key).toBe('yesterday')
			expect(dateBucket(startOfYesterday - 1, dstNow).key).toBe('week')
		} finally {
			process.env.TZ = original
		}
	})

	it('ranks buckets newest-first', () => {
		const ranks = [
			dateBucket(now, now),
			dateBucket(startOfToday - 1, now),
			dateBucket(startOfToday - 2 * day, now),
			dateBucket(startOfToday - 10 * day, now),
			dateBucket(startOfToday - 100 * day, now)
		].map((b) => b.rank)
		expect(ranks).toEqual([0, 1, 2, 3, 4])
	})
})
