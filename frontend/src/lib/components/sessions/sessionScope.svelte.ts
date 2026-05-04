import { derived, type Readable } from 'svelte/store'
import { workspaceStore, userWorkspaces } from '$lib/stores'
import { findWorkspaceDescendants } from '$lib/utils/workspaceHierarchy'

// Set of workspace ids a session must belong to for the user to see it in
// the sidebar list. Currently: the active workspace plus all of its forks
// (descendants). Recomputes when the user switches workspace or when the
// workspace list refreshes.
export const visibleWorkspaceIds: Readable<Set<string>> = derived(
	[workspaceStore, userWorkspaces],
	([ws, all]) => {
		if (!ws) return new Set<string>()
		const ids = new Set<string>([ws])
		for (const d of findWorkspaceDescendants(ws, all)) ids.add(d.id)
		return ids
	}
)
