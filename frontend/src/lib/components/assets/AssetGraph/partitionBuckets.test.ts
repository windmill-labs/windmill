import { describe, expect, it } from 'vitest'
import {
	bucketFor,
	bucketFromInputValue,
	inputValueFromBucket,
	partitionInputType,
	recentBuckets,
	usesCalendarPicker
} from './partitionBuckets'
import type { PartitionSpec } from './parsePipelineAnnotations'

const spec = (kind: PartitionSpec['kind'], extra: Partial<PartitionSpec> = {}): PartitionSpec =>
	({ kind, ...extra }) as PartitionSpec

// A fixed UTC instant: 2026-07-05 14:37Z (a Sunday — ISO week 27 of 2026).
// Explicit UTC so the assertions are independent of the test runner's TZ, and
// they exercise the same UTC default the backend uses when `spec.tz` is absent.
const at = new Date(Date.UTC(2026, 6, 5, 14, 37, 0))

describe('bucketFor mirrors backend default_format (UTC)', () => {
	it('daily -> %Y-%m-%d', () => {
		expect(bucketFor(spec('daily'), at)).toBe('2026-07-05')
	})
	it('hourly -> %Y-%m-%dT%H', () => {
		expect(bucketFor(spec('hourly'), at)).toBe('2026-07-05T14')
	})
	it('monthly -> %Y-%m', () => {
		expect(bucketFor(spec('monthly'), at)).toBe('2026-07')
	})
	it('weekly -> ISO %G-W%V', () => {
		expect(bucketFor(spec('weekly'), at)).toBe('2026-W27')
	})
})

describe('bucketFor honours spec.tz', () => {
	// 2026-07-05 02:30Z is still 2026-07-04 in America/New_York (UTC-4 in July).
	const nearMidnight = new Date(Date.UTC(2026, 6, 5, 2, 30, 0))
	it('shifts the day boundary by the producer tz', () => {
		expect(bucketFor(spec('daily'), nearMidnight)).toBe('2026-07-05') // UTC default
		expect(bucketFor(spec('daily', { tz: 'America/New_York' }), nearMidnight)).toBe('2026-07-04')
	})
	it('shifts the hour bucket by the producer tz', () => {
		// 02:30Z -> 22 (previous day) in New York.
		expect(bucketFor(spec('hourly', { tz: 'America/New_York' }), nearMidnight)).toBe(
			'2026-07-04T22'
		)
	})
})

describe('partitionInputType', () => {
	it('maps each calendar kind to its native input', () => {
		expect(partitionInputType(spec('daily'))).toBe('date')
		expect(partitionInputType(spec('hourly'))).toBe('datetime-local')
		expect(partitionInputType(spec('weekly'))).toBe('week')
		expect(partitionInputType(spec('monthly'))).toBe('month')
	})
	it('falls back to text for dynamic and custom-format specs', () => {
		expect(partitionInputType(spec('dynamic', { key: '$.tenant' }))).toBe('text')
		expect(partitionInputType(spec('daily', { format: '%Y/%m/%d' }))).toBe('text')
		expect(usesCalendarPicker(spec('dynamic', { key: '$.tenant' }))).toBe(false)
		expect(usesCalendarPicker(spec('daily', { format: '%Y/%m/%d' }))).toBe(false)
	})
})

describe('native input <-> bucket round-trip', () => {
	it('hourly truncates the datetime-local minutes and restores :00', () => {
		expect(bucketFromInputValue(spec('hourly'), '2026-07-05T14:37')).toBe('2026-07-05T14')
		expect(inputValueFromBucket(spec('hourly'), '2026-07-05T14')).toBe('2026-07-05T14:00')
	})
	it('non-hourly kinds pass through unchanged', () => {
		expect(bucketFromInputValue(spec('daily'), '2026-07-05')).toBe('2026-07-05')
		expect(inputValueFromBucket(spec('weekly'), '2026-W27')).toBe('2026-W27')
	})
	it('empty stays empty', () => {
		expect(bucketFromInputValue(spec('hourly'), '')).toBe('')
		expect(inputValueFromBucket(spec('daily'), '')).toBe('')
	})
})

describe('recentBuckets (UTC)', () => {
	it('walks back day by day, most recent first', () => {
		expect(recentBuckets(spec('daily'), at, 3)).toEqual(['2026-07-05', '2026-07-04', '2026-07-03'])
	})
	it('walks back hour by hour', () => {
		expect(recentBuckets(spec('hourly'), at, 3)).toEqual([
			'2026-07-05T14',
			'2026-07-05T13',
			'2026-07-05T12'
		])
	})
	it('walks back month by month across a year boundary without day roll-over', () => {
		const jan31 = new Date(Date.UTC(2026, 0, 31, 0, 0, 0))
		expect(recentBuckets(spec('monthly'), jan31, 3)).toEqual(['2026-01', '2025-12', '2025-11'])
	})
	it('walks back week by week', () => {
		expect(recentBuckets(spec('weekly'), at, 3)).toEqual(['2026-W27', '2026-W26', '2026-W25'])
	})
})
