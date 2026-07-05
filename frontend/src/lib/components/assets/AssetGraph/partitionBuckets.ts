// Client-side partition-bucket math for the run form's partition picker.
// Produces the same canonical bucket strings the backend renders in
// `windmill-common/src/partition_ee.rs` (`resolve_time_partition` /
// `default_format`): the instant is localized to the spec's timezone
// (`spec.tz`, defaulting to UTC — NOT the browser's zone) and then formatted.
// Getting the zone right matters — a browser in a non-UTC zone would otherwise
// seed a default bucket, and compare against materialized rows, off by a
// day/hour near every boundary and always for an explicit `tz=` spec.
//
//   daily   %Y-%m-%d      -> 2026-07-05
//   hourly  %Y-%m-%dT%H   -> 2026-07-05T14
//   weekly  %G-W%V (ISO)  -> 2026-W27
//   monthly %Y-%m         -> 2026-07
//
// Pure module (no Svelte runes) so the mapping is unit-testable.

import type { PartitionSpec } from './parsePipelineAnnotations'

export type PartitionInputType = 'date' | 'month' | 'week' | 'datetime-local' | 'text'

// A spec carrying a custom strftime `format` can't be reproduced by the native
// date pickers (arbitrary strftime), and `dynamic` partitions are a free-form
// key extracted from the payload — both fall back to a plain text input.
export function usesCalendarPicker(spec: PartitionSpec): boolean {
	return spec.kind !== 'dynamic' && !spec.format
}

export function partitionInputType(spec: PartitionSpec): PartitionInputType {
	if (!usesCalendarPicker(spec)) return 'text'
	switch (spec.kind) {
		case 'monthly':
			return 'month'
		case 'weekly':
			return 'week'
		case 'hourly':
			return 'datetime-local'
		default:
			return 'date'
	}
}

function pad(n: number): string {
	return String(n).padStart(2, '0')
}

// Re-express `at` as a Date whose UTC fields equal the wall-clock in `tz`, so
// all downstream field reads / calendar arithmetic can use the UTC getters and
// stay in the producer's zone. `hourCycle: 'h23'` keeps hours 00–23.
function zonedAsUtc(at: Date, tz: string): Date {
	const parts = new Intl.DateTimeFormat('en-US', {
		timeZone: tz,
		year: 'numeric',
		month: '2-digit',
		day: '2-digit',
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit',
		hourCycle: 'h23'
	}).formatToParts(at)
	const g = (t: string) => Number(parts.find((p) => p.type === t)?.value)
	return new Date(
		Date.UTC(g('year'), g('month') - 1, g('day'), g('hour'), g('minute'), g('second'))
	)
}

// ISO 8601 week-numbering year + week (chrono's %G / %V) of a UTC-substituted
// date. Standard "nearest Thursday" algorithm, all in UTC.
function isoWeekOf(d: Date): { isoYear: number; week: number } {
	const date = new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate()))
	const dayNum = (date.getUTCDay() + 6) % 7 // Mon=0 … Sun=6
	date.setUTCDate(date.getUTCDate() - dayNum + 3) // Thursday of this week
	const isoYear = date.getUTCFullYear()
	const firstThursday = new Date(Date.UTC(isoYear, 0, 4))
	const fdNum = (firstThursday.getUTCDay() + 6) % 7
	firstThursday.setUTCDate(firstThursday.getUTCDate() - fdNum + 3)
	const week = 1 + Math.round((date.getTime() - firstThursday.getTime()) / (7 * 86400000))
	return { isoYear, week }
}

// Render a UTC-substituted date into its canonical bucket for the cadence.
function fmtBucket(kind: PartitionSpec['kind'], d: Date): string {
	const y = d.getUTCFullYear()
	const m = d.getUTCMonth() + 1
	const day = d.getUTCDate()
	const h = d.getUTCHours()
	switch (kind) {
		case 'hourly':
			return `${y}-${pad(m)}-${pad(day)}T${pad(h)}`
		case 'monthly':
			return `${y}-${pad(m)}`
		case 'weekly': {
			const { isoYear, week } = isoWeekOf(d)
			return `${isoYear}-W${pad(week)}`
		}
		default:
			return `${y}-${pad(m)}-${pad(day)}`
	}
}

export function bucketFor(spec: PartitionSpec, at: Date): string {
	return fmtBucket(spec.kind, zonedAsUtc(at, spec.tz ?? 'UTC'))
}

// Native input value -> canonical bucket. Only hourly differs: datetime-local
// carries a minute component the hourly bucket drops. The picked wall-clock is
// taken verbatim as the bucket (the user picks in the producer's frame), so no
// timezone conversion happens here.
export function bucketFromInputValue(spec: PartitionSpec, inputValue: string): string {
	if (!inputValue) return ''
	if (spec.kind === 'hourly') {
		const m = inputValue.match(/^(\d{4}-\d{2}-\d{2}T\d{2})/)
		return m ? m[1] : inputValue
	}
	return inputValue
}

// Canonical bucket -> native input value. Only hourly differs: datetime-local
// needs a minute component the bucket omits.
export function inputValueFromBucket(spec: PartitionSpec, bucket: string): string {
	if (!bucket) return ''
	if (spec.kind === 'hourly') {
		return /T\d{2}$/.test(bucket) ? `${bucket}:00` : bucket
	}
	return bucket
}

// The last `count` buckets ending at (and including) `now`, most-recent first,
// localized to `spec.tz`. Arithmetic walks calendar units in the zoned frame,
// so it's exact across DST (no ±1 drift). Undefined for non-calendar specs
// (the caller guards on `usesCalendarPicker`).
export function recentBuckets(spec: PartitionSpec, now: Date, count: number): string[] {
	const base = zonedAsUtc(now, spec.tz ?? 'UTC')
	const out: string[] = []
	for (let i = 0; i < count; i++) {
		const d = new Date(base)
		switch (spec.kind) {
			case 'hourly':
				d.setUTCHours(d.getUTCHours() - i)
				break
			case 'weekly':
				d.setUTCDate(d.getUTCDate() - 7 * i)
				break
			case 'monthly':
				// Normalize to the 1st first so subtracting months can't roll over
				// a short target month (e.g. Mar 31 − 1mo → Mar 3).
				d.setUTCDate(1)
				d.setUTCMonth(d.getUTCMonth() - i)
				break
			default:
				d.setUTCDate(d.getUTCDate() - i)
		}
		out.push(fmtBucket(spec.kind, d))
	}
	return out
}

// How many recent buckets the "missing partitions" hint scans, per kind — a
// window that reads as "recent" for each cadence without flooding the hint.
export function recentWindow(kind: PartitionSpec['kind']): number {
	switch (kind) {
		case 'hourly':
			return 24
		case 'weekly':
			return 8
		case 'monthly':
			return 6
		default:
			return 14
	}
}
