<script lang="ts">
	import { copilotInfo, copilotSessionModel, workspaceStore } from '$lib/stores'
	import { writable, type Writable } from 'svelte/store'
	import AIChatDisplay from './AIChatDisplay.svelte'
	import {
		chatRequest,
		prepareSystemMessage,
		prepareUserMessage,
		type AIChatContext,
		type ContextConfig,
		type DisplayMessage
	} from './core'
	import { createEventDispatcher, onDestroy, setContext } from 'svelte'
	import type { ScriptLang } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { openDB, type DBSchema, type IDBPDatabase } from 'idb'
	import { isInitialCode } from '$lib/script_helpers'

	export let lang: ScriptLang
	export let code: string
	export let error: string | undefined

	let contextConfig: ContextConfig = {
		code: true,
		error: false
	}

	let hasInitCode: boolean | null = null
	$: lang && (hasInitCode = null)
	function onCodeChange() {
		if (hasInitCode === null && code) {
			if (isInitialCode(code)) {
				hasInitCode = true
				contextConfig.code = false
			} else {
				hasInitCode = false
			}
		} else if (hasInitCode) {
			contextConfig.code = true
		}
	}
	$: code && onCodeChange()
	$: !error && (contextConfig.error = false)

	let instructions = ''
	let loading = writable(false)
	let currentReply: Writable<string> = writable('')
	const codeStore = writable(code)

	$: codeStore.set(code)

	const dispatch = createEventDispatcher<{
		applyCode: { code: string }
	}>()

	setContext<AIChatContext>('AIChatContext', {
		originalCode: codeStore,
		loading,
		currentReply,
		applyCode: (code: string) => {
			dispatch('applyCode', { code })
		}
	})

	let currentChatId: string = crypto.randomUUID()
	let savedChats: Record<
		string,
		{
			actualMessages: { role: 'user' | 'assistant' | 'system'; content: string }[]
			displayMessages: DisplayMessage[]
			title: string
			id: string
		}
	> = {}
	$: pastChats = Object.values(savedChats)
		.filter((c) => c.id !== currentChatId)
		.reverse()

	let messages: { role: 'user' | 'assistant' | 'system'; content: string }[] = [
		prepareSystemMessage(lang)
	]
	let displayMessages: DisplayMessage[] = []
	let abortController: AbortController | undefined = undefined

	function checkForInvalidModel() {
		if (
			!$copilotSessionModel ||
			($copilotSessionModel && !$copilotInfo.ai_models.includes($copilotSessionModel))
		) {
			$copilotSessionModel = $copilotInfo.ai_models[0]
		}
	}
	$: $copilotInfo && checkForInvalidModel()

	async function sendRequest() {
		if (!instructions.trim()) {
			return
		}
		try {
			loading.set(true)
			abortController = new AbortController()
			displayMessages = [...displayMessages, { role: 'user', content: instructions, contextConfig }]
			const oldInstructions = instructions
			instructions = ''
			const userMessage = await prepareUserMessage(oldInstructions, lang, $workspaceStore!, {
				code: contextConfig.code ? code : undefined,
				error: contextConfig.error ? error : undefined
			})

			messages.push({ role: 'user', content: userMessage })
			await saveChat()

			$currentReply = ''
			await chatRequest(messages, abortController, $copilotInfo.ai_provider, lang, (token) => {
				currentReply.update((prev) => prev + token)
			})

			// if (completion) {
			// for await (const part of completion) {
			// 	const token = getResponseFromEvent(part, $copilotInfo.ai_provider)
			// 	currentReply.update((prev) => prev + token)
			// }

			messages.push({ role: 'assistant', content: $currentReply })
			displayMessages = [...displayMessages, { role: 'assistant', content: $currentReply }]
			currentReply.set('')
			await saveChat()
			// }
		} catch (err) {
			console.error(err)
			if (err instanceof Error) {
				sendUserToast('Failed to send request: ' + err.message, true)
			} else {
				sendUserToast('Failed to send request', true)
			}
		} finally {
			loading.set(false)
		}
	}

	function cancel() {
		currentReply.set('')
		abortController?.abort()
	}

	async function saveChat() {
		if (displayMessages.length > 0) {
			const updatedChat = {
				actualMessages: messages,
				displayMessages: displayMessages,
				title: displayMessages[0].content.slice(0, 50),
				id: currentChatId
			}
			savedChats = {
				...savedChats,
				[updatedChat.id]: updatedChat
			}

			if (db) {
				await db.put('chats', updatedChat)
			}
		}
	}

	async function saveAndClear() {
		await saveChat()
		currentChatId = crypto.randomUUID()
		displayMessages = []
		messages = [prepareSystemMessage(lang)]
	}

	function deletePastChat(id: string) {
		savedChats = Object.fromEntries(Object.entries(savedChats).filter(([key]) => key !== id))
		db?.delete('chats', id)
	}

	function loadPastChat(id: string) {
		const chat = savedChats[id]
		if (chat) {
			messages = chat.actualMessages
			displayMessages = chat.displayMessages
			currentChatId = id
		}
		return []
	}

	export function fix() {
		instructions = 'Fix the error'
		contextConfig = {
			code: true,
			error: true
		}
		sendRequest()
	}

	interface ChatSchema extends DBSchema {
		chats: {
			key: string
			value: {
				id: string
				actualMessages: { role: 'user' | 'assistant' | 'system'; content: string }[]
				displayMessages: DisplayMessage[]
				title: string
			}
		}
	}

	let db: IDBPDatabase<ChatSchema> | undefined = undefined
	async function initDB() {
		db = await openDB<ChatSchema>('copilot-chat-history', 1, {
			upgrade(db) {
				if (!db.objectStoreNames.contains('chats')) {
					db.createObjectStore('chats', { keyPath: 'id' })
				}
			}
		})

		const chats = await db.getAll('chats')
		savedChats = chats.reduce((acc, chat) => {
			acc[chat.id] = chat
			return acc
		}, {} as typeof savedChats)
	}

	initDB()

	onDestroy(() => {
		db?.close()
	})
</script>

<AIChatDisplay
	{pastChats}
	{error}
	bind:contextConfig
	messages={$currentReply
		? [...displayMessages, { role: 'assistant', content: $currentReply }]
		: displayMessages}
	bind:instructions
	on:sendRequest={sendRequest}
	on:cancel={cancel}
	on:saveAndClear={saveAndClear}
	on:deletePastChat={(e) => deletePastChat(e.detail.id)}
	on:loadPastChat={(e) => loadPastChat(e.detail.id)}
>
	<slot name="header-left" slot="header-left" />
	<slot name="header-right" slot="header-right" />
</AIChatDisplay>
