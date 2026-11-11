import { get } from 'svelte/store'
import { userWorkspaces } from '$lib/stores'
import { scopedKey } from '$lib/userScopedStorage'
import {
	deleteArtifact,
	patchArtifact,
	purgeDeletedArtifacts,
	TRASH_RETENTION_MS
} from './artifactsDB'
import type { ArtifactServer } from './artifactServer'
import { shareWorkspaceId } from './artifactSharing'
import { httpArtifactServer } from './httpArtifactServer'

/** The workspace an artifact made while working in `workspaceId` belongs to. */
export function homeWorkspace(workspaceId: string): string {
	return shareWorkspaceId(workspaceId, get(userWorkspaces))
}

/** Move one of the user's artifacts to Recently deleted, its server copy in `workspace` with it:
 * sharing stops while it is there. */
export async function trashArtifact(
	id: string,
	workspace: string | undefined,
	server: ArtifactServer = httpArtifactServer
): Promise<void> {
	if (workspace) await server.trash(workspace, id)
	await patchArtifact(id, { deletedAt: Date.now() })
}

export async function restoreArtifact(
	id: string,
	workspace: string | undefined,
	server: ArtifactServer = httpArtifactServer
): Promise<void> {
	if (workspace) await server.restore(workspace, id)
	await patchArtifact(id, { deletedAt: undefined })
}

/** Delete an artifact for good: from this browser, and the server's copy people opened. */
export async function deleteArtifactForever(
	id: string,
	workspace: string,
	server: ArtifactServer = httpArtifactServer
): Promise<void> {
	await server.delete(workspace, id)
	await deleteArtifact(id)
}

/** Delete from this browser what has been in Recently deleted for longer than it keeps them;
 * the server purges its copies on the same schedule. */
export function sweepTrash(): Promise<string[]> {
	return purgeDeletedArtifacts(Date.now() - TRASH_RETENTION_MS)
}

const OPENS_KEY = 'windmill_artifact_opens'
/** How many opens are remembered, the oldest forgotten first. */
const MAX_OPENS = 500

/** When this user last opened each artifact in this browser, for the page's "Last opened by me"
 * sort. */
export function artifactOpens(): Record<string, number> {
	const key = scopedKey(OPENS_KEY)
	if (!key) return {}
	try {
		const parsed = JSON.parse(localStorage.getItem(key) ?? '{}')
		return parsed && typeof parsed === 'object' ? parsed : {}
	} catch {
		return {}
	}
}

export function markArtifactOpened(id: string): void {
	const key = scopedKey(OPENS_KEY)
	if (!key) return
	const opens = { ...artifactOpens(), [id]: Date.now() }
	const kept = Object.entries(opens)
		.sort(([, a], [, b]) => b - a)
		.slice(0, MAX_OPENS)
	try {
		localStorage.setItem(key, JSON.stringify(Object.fromEntries(kept)))
	} catch {
		// Storage full or blocked: the sort falls back to what it had.
	}
}
