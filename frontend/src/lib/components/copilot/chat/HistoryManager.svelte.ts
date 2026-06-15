import { openDB, deleteDB, type DBSchema as IDBSchema, type IDBPDatabase } from 'idb'
import type { DisplayMessage } from './shared'
import { expanded, messageDraft } from './chatDraft'
import { createLongHash } from '$lib/editorLangUtils'
import { scopedKey } from '$lib/userScopedStorage'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import type { PersistedContextUsage } from './tokenUsage'

// Base IndexedDB name; the effective DB is namespaced by the logged-in user's
// email (scopedKey) so chat messages are never physically shared across users
// on a shared browser. The bare name is also the legacy (pre-namespacing) DB,
// claimed once on first login.
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

// Scoped DB names whose legacy-migration has already been attempted this
// session — avoids re-opening the legacy DB on every init() call.
const legacyMigrationAttempted = new Set<string>()

// One-shot claim of the pre-namespacing chat-history DB: when the user-scoped
// DB has no chats yet (first login on a previously single-user browser), copy
// every record from the legacy un-namespaced DB into it, then delete the
// legacy DB. Both session-tagged and untagged chats belong to the prior single
// browser user, so all are claimed. Subsequent users start with an empty DB.
async function maybeMigrateLegacyChatDb(
	scopedDb: IDBPDatabase<ChatSchema>,
	scopedName: string
): Promise<void> {
	if (scopedName === DB_NAME || legacyMigrationAttempted.has(scopedName)) return
	legacyMigrationAttempted.add(scopedName)
	try {
		if ((await scopedDb.count('chats')) > 0) return
		const legacy = await openDB<ChatSchema>(DB_NAME, 1, {
			upgrade(indexDB) {
				if (!indexDB.objectStoreNames.contains('chats')) {
					indexDB.createObjectStore('chats', { keyPath: 'id' })
				}
			}
		})
		const legacyChats = await legacy.getAll('chats')
		if (legacyChats.length > 0) {
			const tx = scopedDb.transaction('chats', 'readwrite')
			await Promise.all([...legacyChats.map((c) => tx.store.put(c)), tx.done])
		}
		legacy.close()
		await deleteDB(DB_NAME)
	} catch (e) {
		console.error('Could not migrate legacy chat history database', e)
	}
}

export default class HistoryManager {
	private indexDB: IDBPDatabase<ChatSchema> | undefined = undefined

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
		const dbName = scopedKey(DB_NAME)
		// Email-gated: never open a browser-global DB before the logged-in user
		// is known. All callers (AiChatLayout mount, session runtime init,
		// ensureChatIdsSeeded) run post-login, so this is defensive. The singleton
		// also re-runs init() via onUserChange once the email resolves, so an
		// early bail self-heals rather than leaving savedChats empty.
		if (!dbName) return
		try {
			this.indexDB = await openDB<ChatSchema>(dbName, 1, {
				upgrade(indexDB) {
					if (!indexDB.objectStoreNames.contains('chats')) {
						indexDB.createObjectStore('chats', { keyPath: 'id' })
					}
				}
			})

			await maybeMigrateLegacyChatDb(this.indexDB, dbName)

			const chats = await this.indexDB.getAll('chats')
			this.savedChats = chats.reduce(
				(acc, chat) => {
					acc[chat.id] = chat
					return acc
				},
				{} as typeof this.savedChats
			)
		} catch (err) {
			console.error('Could not open chat history database', err)
			return {}
		}
	}

	close() {
		this.indexDB?.close()
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
		if (this.indexDB) {
			await this.indexDB.put('chats', updated)
		}
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
			// Expand any collapsed-paste tokens so the title is readable text, not
			// the chip label + its zero-width id chars.
			const title = expanded(messageDraft(displayMessages[0])).slice(0, 50)
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

			if (this.indexDB) {
				await this.indexDB.put('chats', updatedChat)
			}
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
		this.indexDB?.delete('chats', id)
	}

	loadPastChat(id: string) {
		const chat = this.savedChats[id]
		if (chat) {
			this.currentChatId = id
			return chat
		}
	}
}
