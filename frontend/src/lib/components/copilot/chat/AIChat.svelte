<script lang="ts">
	import { copilotSessionModel, dbSchemas, type DBSchema, type DBSchemas } from '$lib/stores'
	import { writable, type Writable } from 'svelte/store'
	import AIChatDisplay from './AIChatDisplay.svelte'
	import {
		chatRequest,
		prepareSystemMessage,
		prepareUserMessage,
		type AIChatContext,
		type ContextElement,
		type DisplayMessage,
		type SelectedContext
	} from './core'
	import { createEventDispatcher, onDestroy, setContext } from 'svelte'
	import type { AIProviderModel, ScriptLang } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { openDB, type DBSchema as IDBSchema, type IDBPDatabase } from 'idb'
	import { isInitialCode } from '$lib/script_helpers'
	import { langToExt } from '$lib/editorUtils'
	import { scriptLangToEditorLang } from '$lib/scripts'

	export let lang: ScriptLang | 'bunnative'
	export let code: string
	export let error: string | undefined
	export let args: Record<string, any>
	export let path: string | undefined

	$: contextCodePath = path
		? path.split('/').pop() + '.' + langToExt(scriptLangToEditorLang(lang))
		: undefined

	let initializedWithInitCode: boolean | null = null
	$: lang && (initializedWithInitCode = null)
	function onCodeChange() {
		if (!contextCodePath) {
			return
		}
		try {
			if (initializedWithInitCode === null && code) {
				if (isInitialCode(code)) {
					initializedWithInitCode = true
				} else {
					initializedWithInitCode = false
					selectedContext = [
						{
							type: 'code',
							title: contextCodePath
						}
					]
				}
			} else if (initializedWithInitCode) {
				// if the code was initial and was changed, add code context, then prevent it from being added again
				selectedContext = [
					{
						type: 'code',
						title: contextCodePath
					}
				]
				initializedWithInitCode = false
			}
		} catch (err) {
			console.error('Could not update context', err)
		}
	}
	$: contextCodePath && code && onCodeChange()

	let db: { schema: DBSchema; resource: string } | undefined = undefined

	function updateSchema(
		lang: ScriptLang | 'bunnative',
		args: Record<string, any>,
		dbSchemas: DBSchemas
	) {
		try {
			const schemaRes = lang === 'graphql' ? args.api : args.database
			if (typeof schemaRes === 'string') {
				const schemaPath = schemaRes.replace('$res:', '')
				const schema = dbSchemas[schemaPath]
				if (schema && schema.lang === lang) {
					db = { schema, resource: schemaPath }
				} else {
					db = undefined
				}
			} else {
				db = undefined
			}
		} catch (err) {
			console.error('Could not update schema', err)
		}
	}
	$: updateSchema(lang, args, $dbSchemas)

	let selectedContext: SelectedContext[] = []

	let availableContext: ContextElement[] = []

	function updateAvailableContext(
		contextCodePath: string | undefined,
		code: string,
		lang: ScriptLang | 'bunnative',
		error: string | undefined,
		db: { schema: DBSchema; resource: string } | undefined,
		providerModel: AIProviderModel | undefined
	) {
		if (!contextCodePath) {
			return
		}
		try {
			availableContext = [
				{
					type: 'code',
					title: contextCodePath,
					content: code,
					lang
				}
			]

			if (error) {
				availableContext = [
					...availableContext,
					{
						type: 'error',
						title: 'error',
						content: error
					}
				]
			}

			if (db && !providerModel?.model.endsWith('/thinking')) {
				availableContext = [
					...availableContext,
					{
						type: 'db',
						title: db.resource,
						schema: db.schema
					}
				]
			}
		} catch (err) {
			console.error('Could not update available context', err)
		}
	}

	$: updateAvailableContext(contextCodePath, code, lang, error, db, $copilotSessionModel)

	let instructions = ''
	let loading = writable(false)
	let currentReply: Writable<string> = writable('')

	const dispatch = createEventDispatcher<{
		applyCode: { code: string }
	}>()

	setContext<AIChatContext>('AIChatContext', {
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
			lastModified: number
		}
	> = {}

	$: pastChats = Object.values(savedChats)
		.filter((c) => c.id !== currentChatId)
		.sort((a, b) => b.lastModified - a.lastModified)

	let messages: { role: 'user' | 'assistant' | 'system'; content: string }[] = [
		prepareSystemMessage()
	]
	let displayMessages: DisplayMessage[] = []
	let abortController: AbortController | undefined = undefined

	function updateSelectedContextElements() {
		try {
			const contextElements: ContextElement[] = []

			for (const selected of selectedContext) {
				const el = availableContext.find(
					(c) => c.type === selected.type && c.title === selected.title
				)
				if (el) {
					contextElements.push(el)
				}
			}
			return contextElements
		} catch (err) {
			console.error('Could not update selected context elements', err)
			return []
		}
	}
	let selectedContextElements: ContextElement[] = []

	async function sendRequest() {
		if (!instructions.trim()) {
			return
		}
		try {
			loading.set(true)
			aiChatDisplay?.enableAutomaticScroll()
			abortController = new AbortController()

			selectedContextElements = updateSelectedContextElements()

			displayMessages = [
				...displayMessages,
				{
					role: 'user',
					content: instructions,
					contextElements: selectedContextElements
				}
			]
			const oldInstructions = instructions
			instructions = ''
			const userMessage = await prepareUserMessage(oldInstructions, lang, selectedContextElements)

			messages.push({ role: 'user', content: userMessage })
			await saveChat()

			$currentReply = ''
			await chatRequest(
				messages,
				abortController,
				lang,
				(
					selectedContextElements.find((c) => c.type === 'db') as
						| Extract<ContextElement, { type: 'db' }>
						| undefined
				)?.schema,
				(token) => {
					currentReply.update((prev) => prev + token)
				}
			)

			messages.push({ role: 'assistant', content: $currentReply })
			displayMessages = [
				...displayMessages,
				{
					role: 'assistant',
					content: $currentReply,
					contextElements: selectedContextElements
				}
			]
			currentReply.set('')
			await saveChat()
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
				id: currentChatId,
				lastModified: Date.now()
			}
			savedChats = {
				...savedChats,
				[updatedChat.id]: updatedChat
			}

			if (indexDB) {
				await indexDB.put('chats', updatedChat)
			}
		}
	}

	async function saveAndClear() {
		await saveChat()
		currentChatId = crypto.randomUUID()
		displayMessages = []
		messages = [prepareSystemMessage()]
	}

	function deletePastChat(id: string) {
		savedChats = Object.fromEntries(Object.entries(savedChats).filter(([key]) => key !== id))
		indexDB?.delete('chats', id)
	}

	function loadPastChat(id: string) {
		const chat = savedChats[id]
		if (chat) {
			messages = chat.actualMessages
			displayMessages = chat.displayMessages
			currentChatId = id
			aiChatDisplay?.enableAutomaticScroll()
		}
	}

	export function fix() {
		if (!contextCodePath) {
			return
		}
		instructions = 'Fix the error'

		selectedContext = [
			{
				type: 'code',
				title: contextCodePath
			},
			{
				type: 'error',
				title: 'error'
			}
		]

		sendRequest()
	}

	interface ChatSchema extends IDBSchema {
		chats: {
			key: string
			value: {
				id: string
				actualMessages: { role: 'user' | 'assistant' | 'system'; content: string }[]
				displayMessages: DisplayMessage[]
				title: string
				lastModified: number
			}
		}
	}

	let indexDB: IDBPDatabase<ChatSchema> | undefined = undefined
	async function initIndexDB() {
		try {
			console.log('Initializing chat history database')
			indexDB = await openDB<ChatSchema>('copilot-chat-history', 1, {
				upgrade(indexDB) {
					if (!indexDB.objectStoreNames.contains('chats')) {
						indexDB.createObjectStore('chats', { keyPath: 'id' })
					}
				}
			})
			console.log('Chat history database initialized')

			const chats = await indexDB.getAll('chats')
			console.log('Retrieved chats')
			savedChats = chats.reduce((acc, chat) => {
				acc[chat.id] = chat
				return acc
			}, {} as typeof savedChats)
		} catch (err) {
			console.error('Could not open chat history database', err)
		}
	}

	initIndexDB()

	onDestroy(() => {
		cancel()
		indexDB?.close()
	})

	let aiChatDisplay: AIChatDisplay | undefined = undefined
</script>

<AIChatDisplay
	bind:this={aiChatDisplay}
	{pastChats}
	bind:selectedContext
	{availableContext}
	messages={$currentReply
		? [
				...displayMessages,
				{
					role: 'assistant',
					content: $currentReply,
					contextElements: selectedContextElements
				}
		  ]
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
