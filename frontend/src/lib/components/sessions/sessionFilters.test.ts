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
