/**
 * The workspace an artifact made in `workspaceId` belongs to: the topmost ancestor the user still
 * belongs to. A session often runs in a fork, whose members are its creator alone, so an artifact
 * shared there would reach nobody — and it would go with the fork.
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

/** The page that shows an artifact full screen, which is also its share link. */
export function artifactUrl(workspaceId: string, artifactId: string): string {
	return `${window.location.origin}${artifactPath(workspaceId, artifactId)}`
}

export function artifactPath(workspaceId: string, artifactId: string): string {
	return `/artifacts/${encodeURIComponent(artifactId)}?workspace=${encodeURIComponent(workspaceId)}`
}

/** Who can open an artifact, as the artifacts page filters on it. */
export type Audience = 'only_you' | 'everyone' | 'specific'

/** The group every member of a workspace is in. */
export const EVERYONE = 'g/all'

/**
 * Who can open an artifact besides its owner, in the words its row and chip use: "Only you",
 * "Everyone in acme", "data-team and bob", "data-team, bob and 3 others". `extraPerms` is the
 * access list of the server's copy, undefined for an artifact never shared. `owner` names
 * someone else's artifact, which an admin sees unshared: "Only alice".
 */
export function whoCanSee(
	extraPerms: Record<string, boolean> | undefined,
	workspaceName: string,
	owner?: string
): { audience: Audience; label: string } {
	const principals = Object.keys(extraPerms ?? {})
	if (principals.length === 0) {
		return { audience: 'only_you', label: owner ? `Only ${owner}` : 'Only you' }
	}
	if (principals.includes(EVERYONE)) {
		return { audience: 'everyone', label: `Everyone in ${workspaceName}` }
	}
	// Groups before users, as a group usually says more about who reads it than a name does.
	const names = principals
		.sort((a, b) => Number(a.startsWith('u/')) - Number(b.startsWith('u/')) || a.localeCompare(b))
		.map((p) => p.replace(/^[ug]\//, ''))
	return { audience: 'specific', label: listNames(names) }
}

function listNames(names: string[]): string {
	if (names.length <= 3) return names.join(', ').replace(/, ([^,]*)$/, ' and $1')
	return `${names.slice(0, 2).join(', ')} and ${names.length - 2} others`
}
