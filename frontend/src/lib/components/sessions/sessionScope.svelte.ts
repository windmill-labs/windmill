import { derived, type Readable } from 'svelte/store'
import { workspaceStore, userWorkspaces, type UserWorkspace } from '$lib/stores'
import { findWorkspaceDescendants } from '$lib/utils/workspaceHierarchy'

// Walk up parent_workspace_id chain to find the root of the fork family
// containing `id`. Falls back to the workspace itself if it has no parent
// (or its parent isn't in the user's list).
function findFamilyRoot(id: string, all: UserWorkspace[]): UserWorkspace | undefined {
	let current = all.find((w) => w.id === id)
	while (current?.parent_workspace_id) {
		const parent = all.find((w) => w.id === current!.parent_workspace_id)
		if (!parent) break
		current = parent
	}
	return current
}

// Set of workspace ids a session must belong to for the user to see it in
// the sidebar list. The whole fork family is visible from any node: when
// the user is inside fork A whose root is R, sessions belonging to R or
// any sibling fork of A are listed too. Recomputes when the user switches
// workspace or when the workspace list refreshes.
export const visibleWorkspaceIds: Readable<Set<string>> = derived(
	[workspaceStore, userWorkspaces],
	([ws, all]) => {
		if (!ws) return new Set<string>()
		const root = findFamilyRoot(ws, all) ?? ({ id: ws } as UserWorkspace)
		const ids = new Set<string>([root.id])
		for (const d of findWorkspaceDescendants(root.id, all)) ids.add(d.id)
		return ids
	}
)
