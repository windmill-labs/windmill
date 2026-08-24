// Filter and grouping options for the session list, shared by the expanded
// picker header and the collapsed rail's menu so both offer the same choices.

export const LAST_ACTIVITY_OPTIONS: { days: number; label: string; hint: string }[] = [
	{ days: 0, label: 'Any time', hint: 'Any' },
	{ days: 7, label: 'Last 7 days', hint: '7d' },
	{ days: 30, label: 'Last 30 days', hint: '30d' },
	{ days: 90, label: 'Last 90 days', hint: '90d' }
]

export type GroupBy = 'none' | 'date' | 'fork'

export const GROUP_BY_OPTIONS: { value: GroupBy; label: string; hint: string }[] = [
	{ value: 'none', label: 'None', hint: 'None' },
	{ value: 'date', label: 'Date', hint: 'Date' },
	{ value: 'fork', label: 'Workspace fork', hint: 'Fork' }
]

/**
 * Local midnight `daysAgo` calendar days before `now`. Stepping the date field
 * rather than subtracting 24h keeps the boundary on midnight across a DST
 * change, where a local day is 23 or 25 hours long.
 */
function startOfDayBefore(now: number, daysAgo: number): number {
	const d = new Date(now)
	d.setDate(d.getDate() - daysAgo)
	d.setHours(0, 0, 0, 0)
	return d.getTime()
}

/**
 * Calendar-relative bucket for a timestamp, counted from local midnight so
 * "Today" means today's date rather than the last 24 hours. `rank` orders the
 * buckets newest-first.
 */
export function dateBucket(ts: number, now: number): { key: string; label: string; rank: number } {
	if (ts >= startOfDayBefore(now, 0)) return { key: 'today', label: 'Today', rank: 0 }
	if (ts >= startOfDayBefore(now, 1)) return { key: 'yesterday', label: 'Yesterday', rank: 1 }
	if (ts >= startOfDayBefore(now, 7)) return { key: 'week', label: 'Last 7 days', rank: 2 }
	if (ts >= startOfDayBefore(now, 30)) return { key: 'month', label: 'Last 30 days', rank: 3 }
	return { key: 'older', label: 'Older', rank: 4 }
}
