import { base } from '$lib/base'

/**
 * The workspace a share from `workspaceId` lands in: the topmost ancestor the user still
 * belongs to. A session often runs in a fork, whose members are its creator alone, so a link
 * minted there would reach nobody — and it would be deleted with the fork.
 */
export function shareWorkspaceId(
	workspaceId: string,
	workspaces: { id: string; parent_workspace_id?: string | null }[]
): string {
	let current = workspaceId
	const seen = new Set([current])
	for (;;) {
		const parent = workspaces.find((w) => w.id === current)?.parent_workspace_id
		if (!parent || seen.has(parent) || !workspaces.some((w) => w.id === parent)) return current
		seen.add(parent)
		current = parent
	}
}

export function sharedArtifactUrl(workspaceId: string, shareId: string): string {
	return `${window.location.origin}${base}/shared_artifacts/${encodeURIComponent(
		shareId
	)}?workspace=${encodeURIComponent(workspaceId)}`
}

/** "30 days", "12 hours": the retention window in the largest whole unit it fills. */
export function formatRetention(secs: number): string {
	const units: [string, number][] = [
		['day', 86400],
		['hour', 3600],
		['minute', 60]
	]
	for (const [unit, size] of units) {
		if (secs >= size) {
			const n = Math.floor(secs / size)
			return `${n} ${unit}${n === 1 ? '' : 's'}`
		}
	}
	return `${secs} second${secs === 1 ? '' : 's'}`
}
