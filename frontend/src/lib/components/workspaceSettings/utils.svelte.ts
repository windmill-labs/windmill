import { isCloudHosted } from '$lib/cloud'

import { superadmin } from '$lib/stores'
import { derived } from 'svelte/store'

export let isCustomInstanceDbEnabled = derived(
	[superadmin],
	([superadmin_]) => superadmin_ && !isCloudHosted()
)

// Postgres caps identifiers at 63 bytes; the backend rejects longer db names.
const MAX_INSTANCE_DB_NAME_LEN = 63

// Builds a default instance database name scoped to the workspace (e.g. `dt_myworkspace`),
// appending `_1`, `_2`... until an unused name is found. Workspace ids may contain hyphens,
// which are not valid in unquoted postgres identifiers, so they are replaced with underscores.
// The result is truncated to keep it within the postgres identifier length limit.
export function getUnusedInstanceDbName(
	prefix: string,
	workspaceId: string,
	usedNames: Iterable<string>
): string {
	const used = new Set(usedNames)
	const base = `${prefix}_${workspaceId.toLowerCase().replace(/-/g, '_')}`.slice(
		0,
		MAX_INSTANCE_DB_NAME_LEN
	)
	if (!used.has(base)) return base
	let i = 1
	let candidate: string
	do {
		const suffix = `_${i}`
		candidate = base.slice(0, MAX_INSTANCE_DB_NAME_LEN - suffix.length) + suffix
		i++
	} while (used.has(candidate))
	return candidate
}
