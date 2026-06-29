import { derived, type Readable } from 'svelte/store'
import { workspaceStore, userWorkspaces, type UserWorkspace } from '$lib/stores'

export function workspaceRootId(id: string | undefined, all: UserWorkspace[]): string | undefined {
	if (!id) return undefined
	let cur = all.find((w) => w.id === id)
	if (!cur) return id
	while (cur?.parent_workspace_id) {
		const parentId = cur.parent_workspace_id
		const parent = all.find((w) => w.id === parentId)
		if (!parent) return parentId
		cur = parent
	}
	return cur?.id ?? id
}

export const currentWorkspaceRootId: Readable<string | undefined> = derived(
	[workspaceStore, userWorkspaces],
	([ws, all]) => workspaceRootId(ws ?? undefined, all)
)
