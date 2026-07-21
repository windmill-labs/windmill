import type { UserWorkspace } from './stores'

// A fork's parent linkage lives only in its own `workspace` row. Once the fork
// is deleted remotely, `listUserWorkspaces` stops returning it and the parent is
// unrecoverable from the server. We therefore mirror `fork id -> parent id` into
// localStorage while the fork is still reachable, so that after a reload landing
// on a now-deleted fork we can send the user back to its parent instead of a
// dead workspace / forced logout.

const FORK_PARENTS_KEY = 'fork_parents'
const MAX_ENTRIES = 50

function readMap(): Record<string, string> {
	try {
		const raw = localStorage.getItem(FORK_PARENTS_KEY)
		if (!raw) return {}
		const parsed = JSON.parse(raw)
		return parsed && typeof parsed === 'object' ? (parsed as Record<string, string>) : {}
	} catch (e) {
		console.error('Could not read fork parent mapping', e)
		return {}
	}
}

function writeMap(map: Record<string, string>): void {
	try {
		localStorage.setItem(FORK_PARENTS_KEY, JSON.stringify(map))
	} catch (e) {
		console.error('Could not persist fork parent mapping', e)
	}
}

export function rememberForkParent(forkId: string, parentId: string): void {
	const map = readMap()
	if (map[forkId] === parentId) return
	// Re-insert at the end so the oldest entries are the ones trimmed below.
	delete map[forkId]
	map[forkId] = parentId
	const keys = Object.keys(map)
	if (keys.length > MAX_ENTRIES) {
		for (const k of keys.slice(0, keys.length - MAX_ENTRIES)) delete map[k]
	}
	writeMap(map)
}

export function getRememberedForkParent(forkId: string): string | undefined {
	return readMap()[forkId]
}

export function forgetForkParent(forkId: string): void {
	const map = readMap()
	if (forkId in map) {
		delete map[forkId]
		writeMap(map)
	}
}

// Records the parent of the current workspace when it is a fork still present in
// the user's workspace list. No-op otherwise, so it never clobbers a previously
// remembered parent when the fork has already disappeared.
export function recordForkParent(
	workspaceId: string | undefined,
	workspaces: UserWorkspace[]
): void {
	if (!workspaceId) return
	const ws = workspaces.find((w) => w.id === workspaceId)
	if (ws?.parent_workspace_id) {
		rememberForkParent(workspaceId, ws.parent_workspace_id)
	}
}
