/** Two-letter initials; `john.doe` → `JD`, `alice` → `AL`, no username → `?`. */
export function userInitials(username: string | null | undefined): string {
	if (!username) return '?'
	const parts = username.split(/[._\-\s]+/).filter(Boolean)
	if (parts.length >= 2) {
		return (parts[0][0] + parts[1][0]).toUpperCase()
	}
	return username.slice(0, 2).toUpperCase()
}

// Deterministic, so a user's circle has the same color wherever it is drawn.
const PALETTE = [
	'bg-blue-500',
	'bg-emerald-500',
	'bg-amber-500',
	'bg-rose-500',
	'bg-violet-500',
	'bg-cyan-500'
]
export function userColorClass(username: string | null | undefined): string {
	const name = username ?? ''
	let hash = 0
	for (let i = 0; i < name.length; i++) hash = (hash * 31 + name.charCodeAt(i)) >>> 0
	return PALETTE[hash % PALETTE.length]
}
