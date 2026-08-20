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
 * Calendar-relative bucket for a timestamp, counted from local midnight so
 * "Today" means today's date rather than the last 24 hours. `rank` orders the
 * buckets newest-first.
 */
export function dateBucket(
	ts: number,
	now: number
): { key: string; label: string; rank: number } {
	const midnight = new Date(now)
	midnight.setHours(0, 0, 0, 0)
	const startOfToday = midnight.getTime()
	const day = 24 * 60 * 60 * 1000
	if (ts >= startOfToday) return { key: 'today', label: 'Today', rank: 0 }
	if (ts >= startOfToday - day) return { key: 'yesterday', label: 'Yesterday', rank: 1 }
	if (ts >= startOfToday - 7 * day) return { key: 'week', label: 'Last 7 days', rank: 2 }
	if (ts >= startOfToday - 30 * day) return { key: 'month', label: 'Last 30 days', rank: 3 }
	return { key: 'older', label: 'Older', rank: 4 }
}
