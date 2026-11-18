import { BROWSER } from 'esm-env'
import { writable } from 'svelte/store'
import type { UserWorkspace } from '$lib/stores'
import { findWorkspaceAncestors } from './workspaceHierarchy'

const STORAGE_KEY = 'recentWorkspaces'
const MAX_REMEMBERED = 20

/** Above this many workspaces a list gets a search field and a recent group. A shorter list stays
 *  alphabetical: a recent group there only moves rows the user can already see. */
export const LONG_WORKSPACE_LIST = 8
export const RECENT_WORKSPACES_COUNT = 3

function readVisited(): string[] {
	try {
		const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]')
		return Array.isArray(parsed) ? parsed.filter((id) => typeof id === 'string') : []
	} catch {
		return []
	}
}

/** Workspace ids, most recently entered first. Per browser, like the persisted `workspace` key. */
export const visitedWorkspaces = writable<string[]>(BROWSER ? readVisited() : [])

export function recordWorkspaceVisit(workspaceId: string) {
	visitedWorkspaces.update((visited) => {
		if (visited[0] === workspaceId) return visited
		const next = [workspaceId, ...visited.filter((id) => id !== workspaceId)].slice(
			0,
			MAX_REMEMBERED
		)
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
		} catch {}
		return next
	})
}

/**
 * The ids of the root workspaces of the `count` most recently entered families. Lists group forks
 * under their root, so a visit to a fork counts for its family. Empty for a list too short to
 * need a recent group.
 */
export function recentWorkspaceRoots(
	visited: string[],
	workspaces: UserWorkspace[],
	count = RECENT_WORKSPACES_COUNT
): Set<string> {
	const roots = new Set<string>()
	if (workspaces.length <= LONG_WORKSPACE_LIST) return roots
	for (const id of visited) {
		if (roots.size >= count) break
		const workspace = workspaces.find((w) => w.id === id)
		if (!workspace) continue
		roots.add((findWorkspaceAncestors(id, workspaces).at(-1) ?? workspace).id)
	}
	return roots
}
