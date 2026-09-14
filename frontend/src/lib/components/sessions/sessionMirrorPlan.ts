// The pure half of the session backup: given what the local stores hold for one session
// and what was last pushed, decide what the next push carries. Every piece is compared
// against its own marker (a signature for the record, `lastModified` for a chat, the id
// for a write-once image) so a session that only changed locally in ways the backup
// does not keep sends nothing.
import type { AISessionBackupPush } from '$lib/gen'
import { orderedJsonStringify } from '$lib/utils'
import type { Session } from './sessionState.svelte'
import type { ArtifactVersion, PersistedArtifact } from '../copilot/chat/artifacts/artifactsDB'

/** What the backup remembers of a session after a successful push. */
export interface MirrorSyncState {
	id: string
	/** The workspace whose storage holds the backup. */
	ws: string
	/** `headSig` of the record pushed. */
	head: string
	/** `lastModified` of each chat pushed, by chat id. */
	chats: Record<string, number>
	/** The chat each pushed image belongs to, by image id. */
	images: Record<string, string>
	artifacts?: string
}

/**
 * The part of a session record the backup keeps. Left out on purpose: `name` (a
 * per-browser counter), the unsent-draft fields (`pending_*`, `draftPrompt`,
 * `autoSendDraftAt`), `workspace_root_id` (derived on import), `transient`, and the two
 * fields reading a session bumps (`lastSeenCount`, `lastActivityAt`) — so opening a
 * session and reading its new messages never costs a push.
 */
export type SessionHead = Pick<
	Session,
	| 'id'
	| 'workspace_id'
	| 'chatId'
	| 'summary'
	| 'summarySource'
	| 'createdAt'
	| 'archived'
	| 'archivedByWorkspace'
	| 'previewTabs'
	| 'activePreviewTabId'
	| 'previewCollapsed'
	| 'previewSize'
>

export function sessionHead(s: Session): SessionHead {
	const head: SessionHead = { id: s.id, createdAt: s.createdAt }
	if (s.workspace_id !== undefined) head.workspace_id = s.workspace_id
	if (s.chatId !== undefined) head.chatId = s.chatId
	if (s.summary !== undefined) head.summary = s.summary
	if (s.summarySource !== undefined) head.summarySource = s.summarySource
	if (s.archived !== undefined) head.archived = s.archived
	if (s.archivedByWorkspace !== undefined) head.archivedByWorkspace = s.archivedByWorkspace
	if (s.previewTabs !== undefined) head.previewTabs = s.previewTabs
	if (s.activePreviewTabId !== undefined) head.activePreviewTabId = s.activePreviewTabId
	if (s.previewCollapsed !== undefined) head.previewCollapsed = s.previewCollapsed
	if (s.previewSize !== undefined) head.previewSize = s.previewSize
	return head
}

export function headSig(s: Session): string {
	return orderedJsonStringify(sessionHead(s))
}

export interface ArtifactsSnapshot {
	items: PersistedArtifact[]
	versions: ArtifactVersion[]
}

/** Cheap to compute from the rows alone: every edit bumps `updatedAt`, every snapshot has
 * its own key, and approving a plan changes `approvedVersion`. */
export function artifactsFingerprint(a: ArtifactsSnapshot): string {
	const items = a.items
		.map((i) => `${i.id}:${i.updatedAt}:${i.version ?? 1}:${i.approvedVersion ?? ''}`)
		.sort()
	const versions = a.versions.map((v) => v.key).sort()
	return JSON.stringify([items, versions])
}

export interface ChatSnapshot {
	id: string
	lastModified: number
	/** The stored record; absent for a chat that did not change since the last push, whose
	 * bytes the caller did not read, or one too large to back up. */
	record?: unknown
	imageIds: string[]
}

export interface PlanInput {
	session: Session
	/** Every chat the session owns right now. */
	chats: ChatSnapshot[]
	artifacts: ArtifactsSnapshot
	sync?: MirrorSyncState
}

export interface PlannedPush {
	workspaceId: string
	/** Absent when nothing changed that the backup keeps. */
	entry?: AISessionBackupPush
	/** Images the entry needs uploaded, whose bytes the caller loads. */
	images: { chat_id: string; id: string }[]
	/** The workspace the session was backed up in before it moved. */
	removeFrom?: string
	next: MirrorSyncState
}

/** `undefined` for a session with nowhere to go: an unsent draft has no workspace yet. */
export function planSessionPush(input: PlanInput): PlannedPush | undefined {
	const { session, chats, artifacts } = input
	const workspaceId = session.workspace_id
	if (!workspaceId) return undefined
	// A move is a full push into the new workspace's storage; the copy in the old one goes.
	const prev = input.sync?.ws === workspaceId ? input.sync : undefined
	const removeFrom = input.sync && input.sync.ws !== workspaceId ? input.sync.ws : undefined

	const entry: AISessionBackupPush = { id: session.id }
	let changed = false
	const sig = headSig(session)
	if (prev?.head !== sig) {
		entry.head = sessionHead(session)
		changed = true
	}

	const next: MirrorSyncState = {
		id: session.id,
		ws: workspaceId,
		head: sig,
		chats: {},
		images: {}
	}
	const images: { chat_id: string; id: string }[] = []
	const pushedChats: { id: string; record: Record<string, unknown> }[] = []
	for (const chat of chats) {
		next.chats[chat.id] = chat.lastModified
		if (prev?.chats[chat.id] !== chat.lastModified && chat.record !== undefined) {
			pushedChats.push({ id: chat.id, record: chat.record as Record<string, unknown> })
		}
		for (const id of chat.imageIds) {
			next.images[id] = chat.id
			if (prev?.images[id] === undefined) images.push({ chat_id: chat.id, id })
		}
	}
	if (pushedChats.length > 0) {
		entry.chats = pushedChats
		changed = true
	}
	if (prev) {
		const gone = Object.keys(prev.chats).filter((id) => next.chats[id] === undefined)
		if (gone.length > 0) {
			entry.delete_chats = gone
			changed = true
		}
		// An image evicted by the per-chat cap, from a chat that is still there (a deleted
		// chat takes its images with it server-side).
		const evicted = Object.entries(prev.images)
			.filter(([id, chatId]) => next.images[id] === undefined && next.chats[chatId] !== undefined)
			.map(([id, chatId]) => ({ chat_id: chatId, id }))
		if (evicted.length > 0) {
			entry.delete_images = evicted
			changed = true
		}
	}

	const fingerprint = artifactsFingerprint(artifacts)
	next.artifacts = fingerprint
	if (prev?.artifacts !== fingerprint && (artifacts.items.length > 0 || prev?.artifacts)) {
		entry.artifacts = { items: artifacts.items, versions: artifacts.versions }
		changed = true
	}

	return {
		workspaceId,
		entry: changed ? entry : undefined,
		images,
		removeFrom,
		next
	}
}

/** Bytes a JSON body would carry for this value. */
export function jsonBytes(value: unknown): number {
	return JSON.stringify(value).length
}
