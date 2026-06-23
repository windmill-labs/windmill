import { derived, type Readable } from 'svelte/store'
import { workspaceStore, userWorkspaces, type UserWorkspace } from '$lib/stores'

// Immutable family id of a workspace, or undefined when unknown — the workspace
// isn't in the user's list, or its family_id is absent (e.g. the superadmin
// workspace list, which doesn't carry it). Sessions are stored in a per-family
// IndexedDB keyed by this id, so "unknown family" means "don't open a DB":
// fail-safe, since a wrong/missing family yields no sessions rather than another
// family's sessions.
export function familyOfWorkspace(
	id: string | undefined,
	all: UserWorkspace[]
): string | undefined {
	if (!id) return undefined
	return all.find((w) => w.id === id)?.family_id ?? undefined
}

// Family id of the active workspace — the key of the session DB currently in
// view. Switching to another workspace in the same fork family yields the same
// id (same DB, no reload); switching to an unrelated workspace yields a
// different id (a different DB, which cannot contain this family's sessions).
// Recomputes on workspace switch or workspace-list refresh.
export const currentFamilyId: Readable<string | undefined> = derived(
	[workspaceStore, userWorkspaces],
	([ws, all]) => familyOfWorkspace(ws ?? undefined, all)
)
