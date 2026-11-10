import { get } from 'svelte/store'
import { AiService, type AiArtifact, type AiArtifactWriteResult } from '$lib/gen'
import { userWorkspaces } from '$lib/stores'
import { scopedKey } from '$lib/userScopedStorage'
import {
	currentVersion,
	deleteArtifact,
	patchArtifact,
	purgeDeletedArtifacts,
	TRASH_RETENTION_MS,
	type PersistedArtifact
} from './artifactsDB'
import { shareWorkspaceId } from './artifactSharing'

/** The workspace an artifact made while working in `workspaceId` belongs to. */
export function homeWorkspace(workspaceId: string): string {
	return shareWorkspaceId(workspaceId, get(userWorkspaces))
}

/** A call about one of the user's artifacts on the server; undefined when the server holds no
 * copy of it, which is every artifact never shared. */
async function onServer<T>(call: () => Promise<T>): Promise<T | undefined> {
	try {
		return await call()
	} catch (err) {
		if ((err as { status?: number })?.status === 404) return undefined
		throw err
	}
}

/** Put the artifact on the server as its owner's item, with the version this browser holds as
 * the one people see, before its access list is first changed. A copy already there keeps its
 * path and access list. */
export function publishArtifact(a: PersistedArtifact, workspace: string): Promise<AiArtifact> {
	return AiService.publishAiArtifact({
		workspace,
		requestBody: {
			artifact_id: a.id,
			name: a.name,
			kind: a.kind,
			role: a.role,
			session_id: a.sessionId,
			version: currentVersion(a),
			content: a.content
		}
	})
}

/** Choose the version people with access see: `pinned` keeps them on `version` while newer ones
 * are written, unpinned shows them the latest, which `version` must then be. The server takes
 * only versions it knows, so the latest this browser wrote while it was out of reach goes to it
 * first. */
export async function showVersion(
	local: PersistedArtifact,
	workspace: string,
	serverLatest: number,
	shown: { pinned: boolean; version: number; name: string; content: string }
): Promise<AiArtifactWriteResult> {
	if (serverLatest < currentVersion(local)) await publishArtifact(local, workspace)
	return AiService.showAiArtifactVersion({ workspace, artifactId: local.id, requestBody: shown })
}

/** Move one of the user's artifacts to Recently deleted. Sharing stops while it is there. */
export async function trashArtifact(id: string, workspace: string): Promise<void> {
	await onServer(() => AiService.trashAiArtifact({ workspace, artifactId: id }))
	await patchArtifact(id, { deletedAt: Date.now() })
}

export async function restoreArtifact(id: string, workspace: string): Promise<void> {
	await onServer(() => AiService.restoreAiArtifact({ workspace, artifactId: id }))
	await patchArtifact(id, { deletedAt: undefined })
}

/** Delete an artifact for good: from this browser, and the server's copy people opened. */
export async function deleteArtifactForever(id: string, workspace: string): Promise<void> {
	await onServer(() => AiService.deleteAiArtifact({ workspace, artifactId: id }))
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
