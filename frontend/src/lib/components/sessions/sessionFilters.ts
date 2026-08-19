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
