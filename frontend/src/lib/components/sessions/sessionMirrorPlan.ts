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
	/** The workspace's storage went away after this push: what it holds is unknown, so the
	 * next push carries everything again. Kept rather than deleted, so a removal still
	 * knows a backup existed. */
	stale?: boolean
	/** The dirty mark's counter this push covered. A mark is pending while its counter is
	 * above this; retiring it here rather than deleting the mark means a tab bumping the
	 * counter while another flushes can never have its bump erased. */
	flushedV?: number
	/** The user deleted the session and its removal mark could not be written to
	 * localStorage (full): the row itself carries the removal, until it lands. */
	removed?: boolean
	/** The storage the push landed in (`storageName`), and the backup generation (bumped by
	 * a workspace key rotation) it landed under. A row recorded against another storage or
	 * generation describes objects the server no longer looks at. */
	storageId?: string
	generation?: number
	/** Other storages this workspace was on that still hold a copy of the backup (a switch
	 * leaves the old copy where it was): a removal is done only once each has answered it,
	 * or a switch back would bring a deleted session back. */
	alsoIn?: string[]
	/** Bumps of the dirty mark that localStorage refused, recorded here instead: the mark's
	 * counter plus this is what a push retires, and every row write keeps it. */
	extraV?: number
	/** A restore in progress (or cut short): the pieces it wrote for a session that has no
	 * record yet, so a later restore deletes the ones the backup no longer has. */
	staging?: { chats: string[]; images: string[]; items: string[]; versions: string[] }
}

const FALLBACK_STORAGE_PREFIX = 'instance:'

/**
 * How a storage the server answered from is named in the sync rows and the removal marks:
 * by the id the server gives it, the instance object store standing in for a workspace
 * without storage of its own (`fallback` on the answer) told apart from a workspace's own.
 * A removal owed to an instance store is retired by any answer from the workspace's own
 * storage (configuring one moves the backup generation past everything the workspace left
 * in any instance store, so none of it is read again), where one owed to a workspace
 * storage waits for that storage.
 */
export function storageName(id: string, fallback: boolean | undefined): string
export function storageName(
	id: string | undefined,
	fallback: boolean | undefined
): string | undefined
export function storageName(
	id: string | undefined,
	fallback: boolean | undefined
): string | undefined {
	if (id === undefined) return undefined
	return fallback ? FALLBACK_STORAGE_PREFIX + id : id
}

export function isFallbackStorage(name: string): boolean {
	return name.startsWith(FALLBACK_STORAGE_PREFIX)
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
	| 'moves'
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
	if (s.moves !== undefined) head.moves = s.moves
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
	 * bytes the caller did not read. */
	record?: unknown
	imageIds: string[]
	/** Too large to back up: planned as if it did not exist, so a copy pushed while it was
	 * smaller is deleted rather than restored one day as the current transcript. */
	omitted?: boolean
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
	/** Deletes past the per-entry cap were left in `next` for the following push, so the
	 * session must stay marked once this one lands. */
	carried: boolean
	/** Nothing of the session is taken to be in the storage: every piece goes, and the
	 * first part opens the push whole (see `whole` on the entry). */
	whole: boolean
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
	let carried = false
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
		if (chat.omitted) continue
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
		// An image evicted by the per-chat cap, from a chat that is still there (a deleted
		// chat takes its images with it server-side).
		const evicted = Object.entries(prev.images).filter(
			([id, chatId]) => next.images[id] === undefined && next.chats[chatId] !== undefined
		)
		// Past the server's cap per entry, the rest stays in `next` as if still pushed, so
		// the following push finds it gone again.
		if (gone.length > 0) {
			entry.delete_chats = gone.slice(0, MAX_DELETES_PER_ENTRY)
			for (const id of gone.slice(MAX_DELETES_PER_ENTRY)) {
				next.chats[id] = prev.chats[id]
				carried = true
			}
			changed = true
		}
		if (evicted.length > 0) {
			entry.delete_images = evicted
				.slice(0, MAX_DELETES_PER_ENTRY)
				.map(([id, chatId]) => ({ chat_id: chatId, id }))
			for (const [id, chatId] of evicted.slice(MAX_DELETES_PER_ENTRY)) {
				next.images[id] = chatId
				carried = true
			}
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
		next,
		carried,
		whole: prev === undefined
	}
}

/** Bytes a JSON body would carry for this value, as sent: UTF-8, not UTF-16 code units,
 * which would under-count a transcript in a non-Latin script by up to three times. Counted
 * rather than encoded: the values measured are the multi-megabyte ones. */
export function jsonBytes(value: unknown): number {
	const text = JSON.stringify(value)
	let bytes = 0
	for (let i = 0; i < text.length; i++) {
		const c = text.charCodeAt(i)
		if (c < 0x80) bytes += 1
		else if (c < 0x800) bytes += 2
		else if (c >= 0xd800 && c <= 0xdbff) {
			// A surrogate pair is one four-byte code point.
			bytes += 4
			i++
		} else bytes += 3
	}
	return bytes
}

/** Object-store calls the server makes for an entry, the unit its per-request cap counts. */
export function operationsOf(entry: AISessionBackupPush): number {
	return (
		(entry.chats?.length ?? 0) +
		(entry.images?.length ?? 0) +
		(entry.delete_chats?.length ?? 0) +
		(entry.delete_images?.length ?? 0)
	)
}

export interface PushBody {
	owner: string
	sessions: AISessionBackupPush[]
	removed?: string[]
}

/** The server's caps on chats and on each delete list per entry. */
export const MAX_CHATS_PER_ENTRY = 100
export const MAX_DELETES_PER_ENTRY = 1000

/**
 * Break an entry that outgrows the target, or the server's per-entry chat cap, into
 * chat-only entries, each written on its own, with everything else riding on the last one:
 * the entries go out in order and the server lists the session by the last, so the marker
 * never lists a chat that has not landed. (A push of the session whole moves the head to
 * whichever part goes first; see the mirror.)
 */
export function splitEntry(entry: AISessionBackupPush, targetBytes: number): AISessionBackupPush[] {
	if (
		!entry.chats ||
		entry.chats.length <= 1 ||
		(entry.chats.length <= MAX_CHATS_PER_ENTRY && jsonBytes(entry) <= targetBytes)
	) {
		return [entry]
	}
	const { chats, ...rest } = entry
	const parts: AISessionBackupPush[] = []
	let current: typeof chats = []
	let size = 0
	for (const chat of chats) {
		const bytes = jsonBytes(chat)
		if (
			current.length > 0 &&
			(current.length >= MAX_CHATS_PER_ENTRY || size + bytes > targetBytes)
		) {
			parts.push({ id: entry.id, chats: current })
			current = []
			size = 0
		}
		current.push(chat)
		size += bytes
	}
	parts.push({ ...rest, chats: current })
	return parts
}
