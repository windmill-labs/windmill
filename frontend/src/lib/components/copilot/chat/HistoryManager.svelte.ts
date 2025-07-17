import { openDB, type DBSchema as IDBSchema, type IDBPDatabase } from 'idb'
import type { DisplayMessage } from './shared'
import { createLongHash } from '$lib/editorLangUtils'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
interface ChatSchema extends IDBSchema {
	chats: {
		key: string
		value: {
			id: string
			actualMessages: ChatCompletionMessageParam[]
			displayMessages: DisplayMessage[]
			title: string
			lastModified: number
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
		}
	> = $state({})

	private currentChatId: string = $state(createLongHash())

	private pastChats = $derived(
		Object.values(this.savedChats)
			.filter((c) => c.id !== this.currentChatId)
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

	getPastChats() {
		return this.pastChats
	}

	async saveChat(displayMessages: DisplayMessage[], messages: ChatCompletionMessageParam[]) {
		if (displayMessages.length > 0) {
			// we don't want to save the snapshot in the history
			const updatedChat = {
				actualMessages: $state.snapshot(messages),
				displayMessages: $state.snapshot(displayMessages).map((m) => ({
					...m,
					snapshot: undefined
				})),
				title: displayMessages[0].content.slice(0, 50),
				id: this.currentChatId,
				lastModified: Date.now()
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

	async save(displayMessages: DisplayMessage[], messages: ChatCompletionMessageParam[]) {
		await this.saveChat(displayMessages, messages)
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
