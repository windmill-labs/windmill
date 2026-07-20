import { type DBSchema as IDBSchema, type IDBPDatabase } from 'idb'
import type { ChatJob, DisplayMessage } from './shared'
import { expanded, messageDraft } from './chatDraft'
import { createLongHash } from '$lib/editorLangUtils'
import { userScopedDb, type UserScopedDbMigrateDeps } from '$lib/userScopedDb'
import { scopedKey } from '$lib/userScopedStorage'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import type { PersistedContextUsage } from './tokenUsage'
import { IMAGE_OMITTED_PLACEHOLDER, type AttachedImage } from './imageUtils'
import { randomUUID } from '$lib/utils/uuid'

// Base IndexedDB name; userScopedDb namespaces the effective DB by the logged-in
// user's email so chat messages are never physically shared across users on a
// shared browser. The bare name is also the legacy (pre-namespacing) DB, claimed
// once on first login.
const DB_NAME = 'copilot-chat-history'
// v3 adds the images blob store (replacing v2's short-lived toolImages store).
const DB_VERSION = 3
/** Newest image blobs kept per chat; each is a bounded (≤1568px) data URL. */
const MAX_IMAGES_PER_CHAT = 30
/** Marks a persisted image whose bytes live in the `images` store. */
const IMAGE_REF_PREFIX = 'wm-image:'

interface ChatSchema extends IDBSchema {
	chats: {
		key: string
		value: {
			id: string
			actualMessages: ChatCompletionMessageParam[]
			displayMessages: DisplayMessage[]
			title: string
			lastModified: number
			sessionId?: string
			// New writes store the plain reported token count; chats persisted by
			// earlier versions may still hold the legacy anchor object.
			contextUsage?: PersistedContextUsage
			// Workspace items this chat modified via AI tool calls, as
			// `${UserDraftItemKind}:${storagePath}` keys. Persisted out-of-band from
			// the message arrays so it survives compaction. Absent (undefined) on
			// chats predating this feature → consumers fall back to showing all
			// workspace drafts; a defined array (even empty) means "tracked".
			modifiedItems?: string[]
			// Jobs this chat started that detached into the background, so an
			// in-flight job's tray row and completion survive a reload. Absent on
			// chats predating this feature. Persisted out-of-band like modifiedItems.
			backgroundJobs?: ChatJob[]
		}
	}
	// Image bytes, out-of-band from the chat record on purpose: the record is
	// re-cloned into IndexedDB on every saveChat, while a blob is written once
	// and read again only when its chat is reloaded. The persisted message
	// arrays carry `wm-image:<id>` refs in place of the data URLs; swapping
	// happens entirely inside this class (dehydrate on save, hydrate on load),
	// so live chat state never sees a ref.
	images: {
		key: string
		value: {
			id: string
			chatId: string
			dataUrl: string
			savedAt: number
		}
		indexes: { 'by-chat': [string, number] }
	}
}

function createChatStore(db: IDBPDatabase<ChatSchema>): void {
	if (!db.objectStoreNames.contains('chats')) {
		db.createObjectStore('chats', { keyPath: 'id' })
	}
	// v2 briefly kept full-resolution tool screenshots in their own store; the
	// general blob store below covers them now.
	if ((db.objectStoreNames as DOMStringList).contains('toolImages')) {
		db.deleteObjectStore('toolImages' as never)
	}
	if (!db.objectStoreNames.contains('images')) {
		const store = db.createObjectStore('images', { keyPath: 'id' })
		store.createIndex('by-chat', ['chatId', 'savedAt'])
	}
}

/** All image-blob primary keys owned by a chat (via the [chatId, savedAt] index). */
function imageKeysForChat(db: IDBPDatabase<ChatSchema>, chatId: string) {
	return db.getAllKeysFromIndex(
		'images',
		'by-chat',
		IDBKeyRange.bound([chatId, -Infinity], [chatId, Infinity])
	)
}

// Shared across all HistoryManager instances. Each instance owns its own
// userScopedDb handle (see below), so without this the legacy claim could run
// concurrently in several instances on first login — and racing `deleteDB`s can
// block on each other's still-open legacy connections. Deduping to a single
// session-wide promise restores the "runs exactly once" guarantee the pre-factory
// module-level guard had. Reset on failure so a later instance can retry.
let legacyChatClaim: Promise<void> | undefined

async function migrateLegacyChatDb(
	scopedDb: IDBPDatabase<ChatSchema>,
	deps: UserScopedDbMigrateDeps
): Promise<void> {
	legacyChatClaim ??= claimLegacyChatDb(scopedDb, deps).catch((e) => {
		legacyChatClaim = undefined
		throw e
	})
	return legacyChatClaim
}

// One-shot claim of the pre-namespacing chat-history DB: when the user-scoped
// DB has no chats yet (first login on a previously single-user browser), copy
// every record from the legacy un-namespaced DB into it, then delete the legacy
// DB. Both session-tagged and untagged chats belong to the prior single browser
// user, so all are claimed. Subsequent users start with an empty DB.
async function claimLegacyChatDb(
	scopedDb: IDBPDatabase<ChatSchema>,
	{ openDB, deleteDB }: UserScopedDbMigrateDeps
): Promise<void> {
	if ((await scopedDb.count('chats')) > 0) return
	const legacy = await openDB<ChatSchema>(DB_NAME, 1, { upgrade: createChatStore })
	const legacyChats = await legacy.getAll('chats')
	if (legacyChats.length > 0) {
		const tx = scopedDb.transaction('chats', 'readwrite')
		await Promise.all([...legacyChats.map((c) => tx.store.put(c)), tx.done])
	}
	legacy.close()
	await deleteDB(DB_NAME)
}

// Test-only: reset the session-wide legacy-claim guard so suites can exercise
// the migration deterministically regardless of test order.
export function __resetLegacyChatClaimForTesting(): void {
	legacyChatClaim = undefined
}

// Read a chat's modified-items mask by chatId WITHOUT mounting an AIChatManager,
// for the standalone /forks/compare route. Returns undefined for a legacy chat
// (no field) so the page falls back to selecting all items; a defined array
// (even empty) narrows the preselection. Opens a throwaway user-scoped handle;
// the `get` is O(1) on the `id` keyPath.
export async function readChatModifiedItems(chatId: string): Promise<string[] | undefined> {
	const dbh = userScopedDb<ChatSchema>(DB_NAME, {
		version: DB_VERSION,
		upgrade: createChatStore,
		migrate: migrateLegacyChatDb
	})
	try {
		const db = await dbh.whenReady()
		const chat = await db?.get('chats', chatId)
		return chat?.modifiedItems
	} catch (err) {
		console.error('Could not read chat modified items', err)
		return undefined
	} finally {
		dbh.close()
	}
}

export default class HistoryManager {
	// Per-instance handle to the shared per-user DB lifecycle. There is one
	// HistoryManager per AIChatManager (the singleton + one per session runtime),
	// so the handle must be per-instance — not a module singleton.
	private dbh = userScopedDb<ChatSchema>(DB_NAME, {
		version: DB_VERSION,
		upgrade: createChatStore,
		migrate: migrateLegacyChatDb
	})

	private savedChats: Record<
		string,
		{
			actualMessages: ChatCompletionMessageParam[]
			displayMessages: DisplayMessage[]
			title: string
			id: string
			lastModified: number
			sessionId?: string
			contextUsage?: PersistedContextUsage
			modifiedItems?: string[]
			backgroundJobs?: ChatJob[]
		}
	> = $state({})

	private currentChatId: string = $state(createLongHash())

	// When set, this manager is bound to a session: only chats tagged with this id
	// are surfaced and new chats are saved with this id. When undefined (singleton),
	// session-tagged chats are excluded from history.
	private sessionId: string | undefined = $state(undefined)

	// chatId+dataUrl → stable blob id, so every save of the same conversation
	// maps an image to the record written the first time (write-once) instead of
	// minting a new one per save. Hydration seeds it back, so a reloaded chat
	// re-saves under its original ids too. Scoped by chat: each blob record has
	// exactly one owning chat, so the same image pasted into two chats becomes
	// two records — sharing one would let chat A's deletion or cap eviction
	// destroy bytes chat B still references.
	private imageIdByUrl = new Map<string, string>()

	private imageIdKey(chatId: string, dataUrl: string): string {
		return chatId + '\n' + dataUrl
	}

	// Blob writes, stale-blob deletes, and the record put span several IndexedDB
	// transactions, and saveChat has concurrent callers (turn saves, the
	// modified-items and background-jobs writers). Interleaved, an older save's
	// delete pass can remove a blob a newer save just verified, landing the
	// newer record with a dangling ref — so every DB write runs through this
	// per-manager queue. A failed write is rethrown to its caller without
	// wedging the queue.
	private dbWriteQueue: Promise<unknown> = Promise.resolve()

	private enqueueDbWrite<T>(op: (db: IDBPDatabase<ChatSchema>) => Promise<T>): Promise<T | void> {
		// A write belongs to the user who initiated it: capture the scoped DB name
		// now and skip execution if the logged-in user changed while queued —
		// resolving the handle only at execution time would write this user's chat
		// into the NEXT user's database on an in-place account switch.
		const name = scopedKey(DB_NAME)
		const exec = async () => {
			if (!name || scopedKey(DB_NAME) !== name) return
			const db = await this.dbh.whenReady()
			if (!db || db.name !== name) return
			return op(db)
		}
		const run = this.dbWriteQueue.then(exec, exec)
		this.dbWriteQueue = run.catch(() => {})
		return run
	}

	/** Drop cached blob ids of every chat but the given one, so the map doesn't
	 *  pin past chats' data URL strings in memory for the whole session (a
	 *  reopened chat re-seeds its ids through hydration). */
	private pruneImageIds(keepChatId: string) {
		const prefix = keepChatId + '\n'
		for (const key of this.imageIdByUrl.keys()) {
			if (!key.startsWith(prefix)) this.imageIdByUrl.delete(key)
		}
	}

	private pastChats = $derived(
		Object.values(this.savedChats)
			.filter((c) => c.id !== this.currentChatId)
			.filter((c) => (this.sessionId ? c.sessionId === this.sessionId : !c.sessionId))
			.sort((a, b) => b.lastModified - a.lastModified)
	)

	async init() {
		// (Re)initializing adopts a new identity's history: drop the previous
		// identity's cached blob ids with it.
		this.imageIdByUrl.clear()
		// whenReady() is email-gated (returns undefined before the user is known —
		// all callers run post-login, and the singleton re-inits via onUserChange),
		// runs the legacy migration once, and reopens automatically on user change.
		const db = await this.dbh.whenReady()
		if (!db) return
		try {
			const chats = await db.getAll('chats')
			this.savedChats = chats.reduce(
				(acc, chat) => {
					acc[chat.id] = chat
					return acc
				},
				{} as typeof this.savedChats
			)
		} catch (err) {
			console.error('Could not load chat history', err)
		}
	}

	close() {
		this.dbh.close()
	}

	getCurrentChatId() {
		return this.currentChatId
	}

	setCurrentChatId(id: string) {
		this.currentChatId = id
	}

	setSessionId(id: string | undefined) {
		this.sessionId = id
	}

	async tagChatWithSession(chatId: string, sessionId: string) {
		const existing = this.savedChats[chatId]
		if (!existing || existing.sessionId === sessionId) return
		const snapshot = $state.snapshot(existing)
		const updated = { ...snapshot, sessionId }
		this.savedChats = { ...this.savedChats, [chatId]: updated }
		await this.enqueueDbWrite((db) => db.put('chats', updated))
	}

	getPastChats() {
		return this.pastChats
	}

	getAllSavedChats() {
		return Object.values(this.savedChats)
	}

	getModifiedItems(id: string): string[] | undefined {
		return this.savedChats[id]?.modifiedItems
	}

	getBackgroundJobs(id: string): ChatJob[] | undefined {
		return this.savedChats[id]?.backgroundJobs
	}

	/**
	 * Swap every inline image (data URL) in the given message arrays for a
	 * `wm-image:<id>` ref, mutating them in place — callers pass a clone bound
	 * for IndexedDB, never live chat state or the in-memory savedChats mirror.
	 * Returns the id → dataUrl map of every image the arrays reference, plus
	 * every reference (pre-existing refs included) in walk order —
	 * persistImageBlobs ranks ids by their newest reference, so the transcript
	 * walks FIRST: it is always whole and chronological, while drop-oldest
	 * compaction removes old API messages, which would misorder a dropped
	 * message's still-displayed image.
	 */
	private dehydrateImages(
		chatId: string,
		actualMessages: ChatCompletionMessageParam[],
		displayMessages: DisplayMessage[]
	): { blobs: Map<string, string>; refs: string[] } {
		const blobs = new Map<string, string>()
		const refs: string[] = []
		const refFor = (url: string): string => {
			// Already a ref (a record that was never rehydrated): it still counts
			// as a reference — omitting it would let the stale-delete pass reclaim
			// its blob — but there are no bytes to (re)write.
			if (url.startsWith(IMAGE_REF_PREFIX)) {
				refs.push(url.slice(IMAGE_REF_PREFIX.length))
				return url
			}
			const key = this.imageIdKey(chatId, url)
			let id = this.imageIdByUrl.get(key)
			if (!id) {
				id = randomUUID()
				this.imageIdByUrl.set(key, id)
			}
			blobs.set(id, url)
			refs.push(id)
			return IMAGE_REF_PREFIX + id
		}
		for (const message of displayMessages) {
			if (message.role === 'user' && message.images) {
				for (const image of message.images) {
					if (image.dataUrl.startsWith('data:') || image.dataUrl.startsWith(IMAGE_REF_PREFIX)) {
						image.dataUrl = refFor(image.dataUrl)
					}
				}
			} else if (
				message.role === 'tool' &&
				(message.imageUrl?.startsWith('data:') || message.imageUrl?.startsWith(IMAGE_REF_PREFIX))
			) {
				message.imageUrl = refFor(message.imageUrl)
			}
		}
		for (const message of actualMessages) {
			if (!Array.isArray(message.content)) continue
			for (const part of message.content as any[]) {
				if (
					part?.type === 'image_url' &&
					(part.image_url?.url?.startsWith('data:') ||
						part.image_url?.url?.startsWith(IMAGE_REF_PREFIX))
				) {
					part.image_url.url = refFor(part.image_url.url)
				}
			}
		}
		return { blobs, refs }
	}

	/**
	 * The record's newest MAX_IMAGES_PER_CHAT distinct images, ranked by their
	 * LAST reference — the exact set of blobs the chat should own once the
	 * record is committed. The saved record is the single source of truth:
	 * deriving the set from it (rather than from persisted write times) keeps
	 * eviction deterministic and idempotent when turns are truncated, identical
	 * bytes are re-attached, or compaction rewrites the arrays. A ref outside
	 * the kept set hydrates to the omitted-image placeholder.
	 */
	private keptImageIds(refs: string[]): Set<string> {
		const keep = new Set<string>()
		for (let i = refs.length - 1; i >= 0 && keep.size < MAX_IMAGES_PER_CHAT; i--) {
			keep.add(refs[i])
		}
		return keep
	}

	private async writeKeptImageBlobs(
		db: IDBPDatabase<ChatSchema>,
		chatId: string,
		blobs: Map<string, string>,
		keep: Set<string>
	) {
		for (const id of keep) {
			const dataUrl = blobs.get(id)
			if (dataUrl !== undefined && (await db.getKey('images', id)) === undefined) {
				await db.put('images', { id, chatId, dataUrl, savedAt: Date.now() })
			}
		}
	}

	private async deleteStaleImageBlobs(
		db: IDBPDatabase<ChatSchema>,
		chatId: string,
		keep: Set<string>
	) {
		for (const key of await imageKeysForChat(db, chatId)) {
			if (!keep.has(key)) await db.delete('images', key)
		}
	}

	/**
	 * Resolve every `wm-image:` ref in the chat clone back to its data URL,
	 * in place. A missing blob (evicted by the per-chat cap, or IndexedDB
	 * unavailable altogether) degrades the API part to the omitted-image
	 * placeholder and drops the transcript copy — a ref must never leak into
	 * bubbles or outgoing requests. Inline data URLs (records persisted before
	 * the blob store) pass through untouched.
	 */
	private async hydrateImages(
		db: IDBPDatabase<ChatSchema> | undefined,
		chatId: string,
		actualMessages: ChatCompletionMessageParam[],
		displayMessages: DisplayMessage[]
	) {
		const load = async (ref: string): Promise<string | undefined> => {
			const id = ref.slice(IMAGE_REF_PREFIX.length)
			const dataUrl = (await db?.get('images', id))?.dataUrl
			if (dataUrl) this.imageIdByUrl.set(this.imageIdKey(chatId, dataUrl), id)
			return dataUrl
		}
		for (const message of actualMessages) {
			if (!Array.isArray(message.content)) continue
			const content = message.content as any[]
			for (let i = 0; i < content.length; i++) {
				const part = content[i]
				if (part?.type === 'image_url' && part.image_url?.url?.startsWith(IMAGE_REF_PREFIX)) {
					const dataUrl = await load(part.image_url.url)
					content[i] = dataUrl
						? { ...part, image_url: { ...part.image_url, url: dataUrl } }
						: { type: 'text', text: IMAGE_OMITTED_PLACEHOLDER }
				}
			}
		}
		for (const message of displayMessages) {
			if (message.role === 'user' && message.images) {
				const images: AttachedImage[] = []
				for (const image of message.images) {
					if (!image.dataUrl.startsWith(IMAGE_REF_PREFIX)) {
						images.push(image)
						continue
					}
					const dataUrl = await load(image.dataUrl)
					if (dataUrl) images.push({ ...image, dataUrl })
				}
				message.images = images.length > 0 ? images : undefined
				// An image-only bubble that lost every image would render empty —
				// say what happened instead.
				if (!message.images && !message.content.trim()) {
					message.content = IMAGE_OMITTED_PLACEHOLDER
				}
			} else if (message.role === 'tool' && message.imageUrl?.startsWith(IMAGE_REF_PREFIX)) {
				message.imageUrl = await load(message.imageUrl)
			}
		}
	}

	async saveChat(
		displayMessages: DisplayMessage[],
		messages: ChatCompletionMessageParam[],
		contextUsage?: number,
		modifiedItems?: string[],
		backgroundJobs?: ChatJob[]
	) {
		if (displayMessages.length > 0) {
			// Compaction replaces the original first message with a summary boundary.
			// Re-deriving the title would then shift it to the first surviving tail
			// message, so once that boundary leads the transcript, keep the title
			// computed before compaction. Otherwise derive it from the first message,
			// expanding collapsed-paste tokens so it reads as text rather than the
			// chip label + its zero-width id chars.
			const existingTitle = this.savedChats[this.currentChatId]?.title
			const titleSource = displayMessages.find((m) => m.role !== 'summary') ?? displayMessages[0]
			const derivedTitle = expanded(messageDraft(titleSource)).slice(0, 50)
			// An image-only first turn has no text to derive from — fall back to the
			// attachment's filename so the History menu entry isn't blank.
			const imageFallback =
				titleSource.role === 'user' && titleSource.images?.length
					? (titleSource.images[0].name ?? 'Image attachment')
					: ''
			// A hydrated omission marker is not user text — deriving from it would
			// overwrite the filename title an evicted image-only chat was given.
			const title =
				displayMessages[0].role === 'summary' && existingTitle !== undefined
					? existingTitle
					: derivedTitle.trim() && derivedTitle !== IMAGE_OMITTED_PLACEHOLDER
						? derivedTitle
						: imageFallback || existingTitle || ''
			// we don't want to save the snapshot in the history
			const updatedChat = {
				actualMessages: $state.snapshot(messages),
				displayMessages: $state.snapshot(displayMessages).map((m) => ({
					...m,
					snapshot: undefined
				})),
				title,
				id: this.currentChatId,
				lastModified: Date.now(),
				...(this.sessionId ? { sessionId: this.sessionId } : {}),
				...(contextUsage !== undefined ? { contextUsage } : {}),
				// Only persist when the caller passes a defined array — an untracked
				// chat (the global side-panel chat, mask still undefined) must not be
				// stamped with [], which would flip it to the filtered view. Session
				// chats are always tracked (see AIChatManager.loadPastChat), so they do
				// pass a defined array and persist it. Since `put` replaces the whole
				// record, a caller that omits the argument must not ERASE a tracked
				// chat's stored mask — fall back to the previously saved field.
				// Snapshot the fallback: savedChats is $state, so the stored value is a
				// proxy that structuredClone (used by IndexedDB put) cannot clone.
				...(modifiedItems !== undefined
					? { modifiedItems }
					: this.savedChats[this.currentChatId]?.modifiedItems !== undefined
						? { modifiedItems: $state.snapshot(this.savedChats[this.currentChatId].modifiedItems) }
						: {}),
				// Same "don't erase on omit" guard as modifiedItems: a turn-end save
				// that doesn't pass backgroundJobs must keep the tray's stored jobs.
				...(backgroundJobs !== undefined
					? { backgroundJobs }
					: this.savedChats[this.currentChatId]?.backgroundJobs !== undefined
						? {
								backgroundJobs: $state.snapshot(this.savedChats[this.currentChatId].backgroundJobs)
							}
						: {})
			}
			// The mirror mirrors what the DB holds (refs — the snapshot is
			// dehydrated below before either sees it): a reopened chat hydrates
			// through the store, reseeding stable blob ids. When IndexedDB is
			// unavailable the writes no-op and hydration degrades the refs to
			// omitted-image placeholders — like every other userScopedDb consumer,
			// history simply doesn't persist there.
			const { blobs, refs } = this.dehydrateImages(
				updatedChat.id,
				updatedChat.actualMessages,
				updatedChat.displayMessages
			)
			this.savedChats = {
				...this.savedChats,
				[updatedChat.id]: updatedChat
			}
			await this.enqueueDbWrite(async (db) => {
				// Write order is the crash-safety story: kept blobs land before the
				// record that references them, and stale blobs are deleted only after
				// the new record is committed. A failure at any step leaves the last
				// committed record fully hydratable — at worst orphan blobs linger
				// until the next successful save's delete pass reclaims them.
				const keep = this.keptImageIds(refs)
				await this.writeKeptImageBlobs(db, updatedChat.id, blobs, keep)
				await db.put('chats', updatedChat)
				// Best-effort: the record is already committed, so a failed cleanup
				// (e.g. a user switch closed this handle mid-op) must not turn a
				// successful save into a rejection — the orphans are reclaimed by
				// the next successful save's pass.
				await this.deleteStaleImageBlobs(db, updatedChat.id, keep).catch((err) =>
					console.error('Could not prune stale image blobs', err)
				)
			})
		}
	}

	async save(
		displayMessages: DisplayMessage[],
		messages: ChatCompletionMessageParam[],
		contextUsage?: number,
		modifiedItems?: string[],
		backgroundJobs?: ChatJob[]
	) {
		await this.saveChat(displayMessages, messages, contextUsage, modifiedItems, backgroundJobs)
		this.currentChatId = createLongHash()
		this.pruneImageIds(this.currentChatId)
	}

	deletePastChat(id: string) {
		this.savedChats = Object.fromEntries(
			Object.entries(this.savedChats).filter(([key]) => key !== id)
		)
		void this.enqueueDbWrite(async (db) => {
			await db.delete('chats', id)
			const keys = await imageKeysForChat(db, id)
			await Promise.all(keys.map((key) => db.delete('images', key)))
		}).catch((err) => console.error('Could not delete chat', err))
	}

	async loadPastChat(id: string) {
		const chat = this.savedChats[id]
		if (!chat) return
		this.currentChatId = id
		this.pruneImageIds(id)
		// Hand back a hydrated clone: the stored record keeps its refs (matching
		// what the DB holds) while the live chat gets real data URLs. Hydration
		// runs even without a DB so refs degrade to placeholders instead of
		// leaking into bubbles and requests.
		const snapshot = $state.snapshot(chat) as typeof chat
		const db = await this.dbh.whenReady()
		await this.hydrateImages(db, id, snapshot.actualMessages, snapshot.displayMessages)
		return snapshot
	}
}
