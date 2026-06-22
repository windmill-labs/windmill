import { isCloudHosted } from '$lib/cloud'

import { superadmin } from '$lib/stores'
import { derived } from 'svelte/store'

export let isCustomInstanceDbEnabled = derived(
	[superadmin],
	([superadmin_]) => superadmin_ && !isCloudHosted()
)

// Builds a default instance database name scoped to the workspace (e.g. `dt_myworkspace`),
// appending `_1`, `_2`... until an unused name is found. Workspace ids may contain hyphens,
// which are not valid in unquoted postgres identifiers, so they are replaced with underscores.
export function getUnusedInstanceDbName(
	prefix: string,
	workspaceId: string,
	usedNames: Iterable<string>
): string {
	const used = new Set(usedNames)
	const base = `${prefix}_${workspaceId.toLowerCase().replace(/-/g, '_')}`
	if (!used.has(base)) return base
	let i = 1
	while (used.has(`${base}_${i}`)) i++
	return `${base}_${i}`
}
