import { get } from 'svelte/store'
import { hash } from '$lib/components/apps/editor/commonAppUtils'
import { AiService, type AiArtifact, type AiArtifactWriteResult } from '$lib/gen'
import { userWorkspaces } from '$lib/stores'
import { scopedKey } from '$lib/userScopedStorage'
import {
	currentVersion,
	deleteArtifact,
	getArtifactVersion,
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

/** The SHA-256 of a version's text, which the server knows its latest version by: two of the
 * owner's browsers can each hold a different text under one version number. */
export function fingerprint(content: string): Promise<string> {
	return hash(content)
}

/** Whether this browser holds `version` of `local`, older than its latest, with the text whose
 * SHA-256 is `latestHash`: the server's latest version is then one this browser wrote on the way
 * to its own latest, which was made on it. */
export async function holdsOlder(
	local: PersistedArtifact,
	version: number,
	latestHash: string
): Promise<boolean> {
	if (version >= currentVersion(local)) return false
	const held = await getArtifactVersion(local.id, version).catch(() => undefined)
	return held !== undefined && (await fingerprint(held.content)) === latestHash
}

/** Give a server copy whose latest version `holdsOlder` found in this browser this browser's
 * latest, as a write made on that version. Refused if the server's latest moved meanwhile. */
function bringUpToDate(
	local: PersistedArtifact,
	workspace: string,
	latestHash: string
): Promise<AiArtifactWriteResult> {
	return AiService.writeAiArtifact({
		workspace,
		artifactId: local.id,
		requestBody: {
			base_hash: latestHash,
			version: currentVersion(local),
			name: local.name,
			content: local.content,
			session_id: local.sessionId
		}
	})
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
 * path and access list, and takes this browser's latest only when its own latest is one of this
 * browser's older versions. */
export async function publishArtifact(
	a: PersistedArtifact,
	workspace: string
): Promise<AiArtifact> {
	const item = await AiService.publishAiArtifact({
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
	if (item.latest_version >= currentVersion(a)) return item
	// Only the server knows its latest text's SHA-256, so the write names this browser's text
	// under the same number, and the server refuses it when that is not its latest.
	const held = await getArtifactVersion(a.id, item.latest_version).catch(() => undefined)
	if (!held) return item
	try {
		const result = await bringUpToDate(a, workspace, await fingerprint(held.content))
		return result.artifact ?? item
	} catch {
		return item
	}
}

/** Choose the version people with access see: `pinned` keeps them on `version` while newer ones
 * are written, unpinned shows them the latest, which `local` holds. Refused, `stale`, unless the
 * server's latest version is `local`'s; a server copy behind `local` is brought up to it first. */
export async function showVersion(
	local: PersistedArtifact,
	workspace: string,
	shown: { pinned: boolean; version: number; name: string; content: string }
): Promise<AiArtifactWriteResult> {
	const base_hash = await fingerprint(local.content)
	const show = () =>
		AiService.showAiArtifactVersion({
			workspace,
			artifactId: local.id,
			requestBody: { ...shown, base_hash }
		})
	const result = await show()
	const { latest_version, latest_hash } = result
	if (result.outcome !== 'stale' || latest_version === undefined || latest_hash === undefined) {
		return result
	}
	if (!(await holdsOlder(local, latest_version, latest_hash))) return result
	const caught = await bringUpToDate(local, workspace, latest_hash)
	return caught.outcome === 'saved' ? show() : caught
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
