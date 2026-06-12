import { openDB, type DBSchema as IDBSchema, type IDBPDatabase } from 'idb'
import type { DisplayMessage } from './shared'
import { expanded, messageDraft } from './chatDraft'
import { createLongHash } from '$lib/editorLangUtils'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import type { ContextTokenSnapshot } from './tokenUsage'
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
			contextUsage?: ContextTokenSnapshot
		}
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
			contextUsage?: ContextTokenSnapshot
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
		try {
			this.indexDB = await openDB<ChatSchema>('copilot-chat-history', 1, {
				upgrade(indexDB) {
					if (!indexDB.objectStoreNames.contains('chats')) {
						indexDB.createObjectStore('chats', { keyPath: 'id' })
					}
				}
			})

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
		contextUsage?: ContextTokenSnapshot
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
				...(contextUsage ? { contextUsage: $state.snapshot(contextUsage) } : {})
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
		contextUsage?: ContextTokenSnapshot
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
