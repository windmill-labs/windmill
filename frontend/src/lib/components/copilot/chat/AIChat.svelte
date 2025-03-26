<script lang="ts">
	import { copilotSessionModel, dbSchemas, type DBSchema, type DBSchemas, SQLSchemaLanguages, workspaceStore } from '$lib/stores'
	import { writable, type Writable } from 'svelte/store'
	import AIChatDisplay from './AIChatDisplay.svelte'
	import {
		chatRequest,
		prepareSystemMessage,
		prepareUserMessage,
		type AIChatContext,
		type ContextElement,
		type DisplayMessage,
	} from './core'
	import { createEventDispatcher, onDestroy, setContext } from 'svelte'
	import { type AIProviderModel, type ScriptLang, ResourceService } from '$lib/gen'
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

	$: contextCodePath =
		(path?.split('/').pop() ?? 'script') + '.' + langToExt(scriptLangToEditorLang(lang))

	let initializedWithInitCode: boolean | null = null
	$: lang && (initializedWithInitCode = null)

	function onCodeChange(contextCodePath: string) {
		if (initializedWithInitCode === null && code) {
			if (isInitialCode(code)) {
				initializedWithInitCode = true
			} else {
				initializedWithInitCode = false
				selectedContext = [
					{
						type: 'code',
						title: contextCodePath,
						content: code,
						lang
					}
				]
			}
		} else if (initializedWithInitCode) {
			// if the code was initial and was changed, add code context, then prevent it from being added again
			selectedContext = [
				{
					type: 'code',
					title: contextCodePath,
					content: code,
					lang
				}
			]
			initializedWithInitCode = false
		}
	}
	$: code && onCodeChange(contextCodePath)

	let db: { schema: DBSchema; resource: string } | undefined = undefined

	function updateSchema(
		lang: ScriptLang | 'bunnative',
		args: Record<string, any>,
		dbSchemas: DBSchemas
	) {
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
	}
	$: updateSchema(lang, args, $dbSchemas)

	let selectedContext: ContextElement[] = []

	let availableContext: ContextElement[] = []

	async function updateAvailableContext(
		contextCodePath: string,
		code: string,
		lang: ScriptLang | 'bunnative',
		error: string | undefined,
		db: { schema: DBSchema; resource: string } | undefined,
		providerModel: AIProviderModel | undefined,
		dbSchemas: DBSchemas,
		workspace?: string,
	) {
		availableContext = [
			{
				type: 'code',
				title: contextCodePath,
				content: code,
				lang
			}
		]

		if (workspace && !providerModel?.model.endsWith('/thinking')) {
			// Make all dbs in the workspace available
			const dbs = await ResourceService.listResource({
				workspace: workspace,
				resourceType: SQLSchemaLanguages.join(',')
			})
			for (const d of dbs) {
				const loadedSchema = dbSchemas[d.path]
				availableContext.push({
					type: 'db',
					title: d.path,
					// If the db is already fetched, add the schema to the context
					...(loadedSchema ? { schema: loadedSchema } : {})
				})
			}
		}

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

 		if (db) {
			// If the db is already fetched, add it to the selected context
			if (!selectedContext.find((c) => c.type === 'db' && c.title === db.resource) && !providerModel?.model.endsWith('/thinking')) {
				selectedContext = [
					...selectedContext,
					{
						type: 'db',
						title: db.resource,
						schema: db.schema
					}
				]
			}
		}
	}

	function updateSelectedContext(selectedContext: ContextElement[], availableContext: ContextElement[]) {
		return selectedContext.filter((c) => availableContext.find((ac) => ac.type === c.type && ac.title === c.title))
	}

	$: updateAvailableContext(contextCodePath, code, lang, error, db, $copilotSessionModel, $dbSchemas, $workspaceStore)

	$: selectedContext = updateSelectedContext(selectedContext, availableContext)

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

	async function sendRequest() {
		if (!instructions.trim()) {
			return
		}
		try {
			loading.set(true)
			aiChatDisplay?.enableAutomaticScroll()
			abortController = new AbortController()

			displayMessages = [
				...displayMessages,
				{
					role: 'user',
					content: instructions,
					contextElements: selectedContext
				}
			]
			const oldInstructions = instructions
			instructions = ''
			const userMessage = await prepareUserMessage(oldInstructions, lang, selectedContext)

			messages.push({ role: 'user', content: userMessage })
			await saveChat()

			$currentReply = ''
			await chatRequest(
				messages,
				abortController,
				lang,
				selectedContext.filter((c) => c.type === 'db').length > 0,
				(token) => currentReply.update((prev) => prev + token)
			)

			messages.push({ role: 'assistant', content: $currentReply })
			displayMessages = [
				...displayMessages,
				{
					role: 'assistant',
					content: $currentReply,
					contextElements: selectedContext
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
		instructions = 'Fix the error'

		const codeContext = availableContext.find((c) => c.type === 'code' && c.title === contextCodePath)
		const errorContext = availableContext.find((c) => c.type === 'error')

		if (codeContext && errorContext) {
			selectedContext = [codeContext, errorContext]
		}

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
		indexDB = await openDB<ChatSchema>('copilot-chat-history', 1, {
			upgrade(indexDB) {
				if (!indexDB.objectStoreNames.contains('chats')) {
					indexDB.createObjectStore('chats', { keyPath: 'id' })
				}
			}
		})

		const chats = await indexDB.getAll('chats')
		savedChats = chats.reduce((acc, chat) => {
			acc[chat.id] = chat
			return acc
		}, {} as typeof savedChats)
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
					contextElements: selectedContext
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
