import { type DBSchema as IDBSchema, type IDBPDatabase } from 'idb'
import type { DisplayMessage } from './shared'
import { expanded, messageDraft } from './chatDraft'
import { createLongHash } from '$lib/editorLangUtils'
import { userScopedDb, type UserScopedDbMigrateDeps } from '$lib/userScopedDb'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import type { PersistedContextUsage } from './tokenUsage'

// Base IndexedDB name; userScopedDb namespaces the effective DB by the logged-in
// user's email so chat messages are never physically shared across users on a
// shared browser. The bare name is also the legacy (pre-namespacing) DB, claimed
// once on first login.
const DB_NAME = 'copilot-chat-history'

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
		}
	}
}

function createChatStore(db: IDBPDatabase<ChatSchema>): void {
	if (!db.objectStoreNames.contains('chats')) {
		db.createObjectStore('chats', { keyPath: 'id' })
	}
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

export default class HistoryManager {
	// Per-instance handle to the shared per-user DB lifecycle. There is one
	// HistoryManager per AIChatManager (the singleton + one per session runtime),
	// so the handle must be per-instance — not a module singleton.
	private dbh = userScopedDb<ChatSchema>(DB_NAME, {
		version: 1,
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
		}
	> = $state({})

	private currentChatId: string = $state(createLongHash())

	// When set, this manager is bound to a session: only chats tagged with this id
	// are surfaced and new chats are saved with this id. When undefined (singleton),
	// session-tagged chats are excluded from history.
	private sessionId: string | undefined = $state(undefined)

	private pastChats = $derived(
		Object.values(this.savedChats)
			.filter((c) => c.id !== this.currentChatId)
			.filter((c) => (this.sessionId ? c.sessionId === this.sessionId : !c.sessionId))
			.sort((a, b) => b.lastModified - a.lastModified)
	)

	async init() {
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
		// Resolve the DB via the handle (not a cached ref) so a write always lands
		// in the current user's DB, even after an in-place user switch.
		const db = await this.dbh.whenReady()
		if (db) await db.put('chats', updated)
	}

	getPastChats() {
		return this.pastChats
	}

	getAllSavedChats() {
		return Object.values(this.savedChats)
	}

	async saveChat(
		displayMessages: DisplayMessage[],
		messages: ChatCompletionMessageParam[],
		contextUsage?: number
	) {
		if (displayMessages.length > 0) {
			// Compaction replaces the original first message with a summary boundary.
			// Re-deriving the title would then shift it to the first surviving tail
			// message, so once that boundary leads the transcript, keep the title
			// computed before compaction. Otherwise derive it from the first message,
			// expanding collapsed-paste tokens so it reads as text rather than the
			// chip label + its zero-width id chars.
			const existingTitle = this.savedChats[this.currentChatId]?.title
			const title =
				displayMessages[0].role === 'summary' && existingTitle !== undefined
					? existingTitle
					: expanded(
							messageDraft(displayMessages.find((m) => m.role !== 'summary') ?? displayMessages[0])
						).slice(0, 50)
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
				...(contextUsage !== undefined ? { contextUsage } : {})
			}
			this.savedChats = {
				...this.savedChats,
				[updatedChat.id]: updatedChat
			}

			const db = await this.dbh.whenReady()
			if (db) await db.put('chats', updatedChat)
		}
	}

	async save(
		displayMessages: DisplayMessage[],
		messages: ChatCompletionMessageParam[],
		contextUsage?: number
	) {
		await this.saveChat(displayMessages, messages, contextUsage)
		this.currentChatId = createLongHash()
	}

	deletePastChat(id: string) {
		this.savedChats = Object.fromEntries(
			Object.entries(this.savedChats).filter(([key]) => key !== id)
		)
		void this.dbh.whenReady().then((db) => db?.delete('chats', id))
	}

	loadPastChat(id: string) {
		const chat = this.savedChats[id]
		if (chat) {
			this.currentChatId = id
			return chat
		}
	}
}
