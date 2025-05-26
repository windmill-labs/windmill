<script lang="ts">
	import {
		copilotSessionModel,
		dbSchemas,
		type DBSchema,
		type DBSchemas,
		SQLSchemaLanguages,
		workspaceStore
	} from '$lib/stores'
	import { writable, type Writable } from 'svelte/store'
	import AIChatDisplay from './AIChatDisplay.svelte'
	import {
		chatRequest,
		prepareSystemMessage,
		prepareUserMessage,
		type AIChatContext,
		type ContextElement,
		type DisplayMessage
	} from './core'
	import { createEventDispatcher, onDestroy, setContext } from 'svelte'
	import {
		type AIProviderModel,
		type ListResourceResponse,
		type ScriptLang,
		ResourceService
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { openDB, type DBSchema as IDBSchema, type IDBPDatabase } from 'idb'
	import { isInitialCode } from '$lib/script_helpers'
	import { createLongHash, langToExt } from '$lib/editorUtils'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import { diffLines } from 'diff'

	export let lang: ScriptLang | 'bunnative'
	export let code: string
	export let error: string | undefined
	export let args: Record<string, any>
	export let path: string | undefined
	export let lastSavedCode: string | undefined = undefined
	export let lastDeployedCode: string | undefined = undefined
	export let diffMode: boolean = false

	$: contextCodePath = path
		? (path.split('/').pop() ?? 'script') + '.' + langToExt(scriptLangToEditorLang(lang))
		: undefined

	let initializedWithInitCode: boolean | null = null
	$: lang && (initializedWithInitCode = null)

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

	let selectedContext: ContextElement[] = []

	let availableContext: ContextElement[] = []

	let dbResources: ListResourceResponse = []

	async function updateDBResources(workspace: string | undefined) {
		if (workspace) {
			dbResources = await ResourceService.listResource({
				workspace: workspace,
				resourceType: SQLSchemaLanguages.join(',')
			})
		}
	}

	async function updateAvailableContext(
		contextCodePath: string | undefined,
		code: string,
		lang: ScriptLang | 'bunnative',
		error: string | undefined,
		db: { schema: DBSchema; resource: string } | undefined,
		providerModel: AIProviderModel | undefined,
		dbSchemas: DBSchemas,
		dbResources: ListResourceResponse,
		lastSavedCode: string | undefined,
		lastDeployedCode: string | undefined
	) {
		if (!contextCodePath) {
			return
		}
		try {
			let newAvailableContext: ContextElement[] = [
				{
					type: 'code',
					title: contextCodePath,
					content: code,
					lang
				}
			]
			if (!providerModel?.model.endsWith('/thinking')) {
				for (const d of dbResources) {
					const loadedSchema = dbSchemas[d.path]
					newAvailableContext.push({
						type: 'db',
						title: d.path,
						// If the db is already fetched, add the schema to the context
						...(loadedSchema ? { schema: loadedSchema } : {})
					})
				}
			}

			if (lastSavedCode && lastSavedCode !== code) {
				newAvailableContext.push({
					type: 'diff',
					title: 'diff_with_last_saved_draft',
					content: lastSavedCode ?? '',
					diff: diffLines(lastSavedCode ?? '', code),
					lang
				})
			}

			if (lastDeployedCode && lastDeployedCode !== code) {
				newAvailableContext.push({
					type: 'diff',
					title: 'diff_with_last_deployed_version',
					content: lastDeployedCode ?? '',
					diff: diffLines(lastDeployedCode ?? '', code),
					lang
				})
			}

			if (error) {
				newAvailableContext = [
					...newAvailableContext,
					{
						type: 'error',
						title: 'error',
						content: error
					}
				]
			}

			if (db) {
				// If the db is already fetched, add it to the selected context
				if (
					!selectedContext.find((c) => c.type === 'db' && c.title === db.resource) &&
					!providerModel?.model.endsWith('/thinking')
				) {
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

			availableContext = newAvailableContext

			if (
				code &&
				((initializedWithInitCode === null && !isInitialCode(code)) || initializedWithInitCode)
			) {
				selectedContext = [
					{
						type: 'code',
						title: contextCodePath,
						content: code,
						lang
					}
				]
			}

			if (code && initializedWithInitCode === null) {
				initializedWithInitCode = isInitialCode(code)
			}

			selectedContext = selectedContext
				.map((c) =>
					c.type === 'code_piece' && code.includes(c.content)
						? c
						: availableContext.find((ac) => ac.type === c.type && ac.title === c.title)
				)
				.filter((c) => c !== undefined) as ContextElement[]
		} catch (err) {
			console.error('Could not update available context', err)
		}
	}

	function updateDisplayMessages(dbSchemas: DBSchemas) {
		return displayMessages.map((m) => ({
			...m,
			contextElements: m.contextElements?.map((c) =>
				c.type === 'db'
					? {
							type: 'db',
							title: c.title,
							schema: dbSchemas[c.title]
						}
					: c
			) as ContextElement[]
		}))
	}

	$: updateDBResources($workspaceStore)

	$: updateAvailableContext(
		contextCodePath,
		code,
		lang,
		error,
		db,
		$copilotSessionModel,
		$dbSchemas,
		dbResources,
		lastSavedCode,
		lastDeployedCode
	)

	let instructions = ''
	let loading = writable(false)
	let currentReply: Writable<string> = writable('')

	const dispatch = createEventDispatcher<{
		applyCode: { code: string }
		showDiffMode: null
	}>()

	setContext<AIChatContext>('AIChatContext', {
		loading,
		currentReply,
		applyCode: (code: string) => {
			dispatch('applyCode', { code })
		}
	})

	let currentChatId: string = createLongHash()
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

	$: displayMessages = updateDisplayMessages($dbSchemas)

	async function sendRequest(options: { removeDiff?: boolean; addBackCode?: boolean } = {}) {
		if (!instructions.trim()) {
			return
		}
		try {
			// Remove code pieces from the context to not include them on the next request
			const oldSelectedContext = selectedContext
			selectedContext = selectedContext.filter((c) => c.type !== 'code_piece')
			if (options.removeDiff) {
				selectedContext = selectedContext.filter((c) => c.type !== 'diff')
			}
			if (options.addBackCode) {
				const codeContext = availableContext.find(
					(c) => c.type === 'code' && c.title === contextCodePath
				)
				if (codeContext) {
					selectedContext = [...selectedContext, codeContext]
				}
			}
			loading.set(true)
			aiChatDisplay?.enableAutomaticScroll()
			abortController = new AbortController()

			displayMessages = [
				...displayMessages,
				{
					role: 'user',
					content: instructions,
					contextElements: oldSelectedContext
				}
			]
			const oldInstructions = instructions
			instructions = ''
			const userMessage = await prepareUserMessage(oldInstructions, lang, oldSelectedContext)

			messages.push({ role: 'user', content: userMessage })
			await saveChat()

			$currentReply = ''
			await chatRequest(
				messages,
				abortController,
				lang,
				oldSelectedContext.filter((c) => c.type === 'db').length > 0,
				(token) => currentReply.update((prev) => prev + token)
			)

			messages.push({ role: 'assistant', content: $currentReply })
			displayMessages = [
				...displayMessages,
				{
					role: 'assistant',
					content: $currentReply,
					contextElements: oldSelectedContext.filter((c) => c.type === 'code')
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
		currentChatId = createLongHash()
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

	export function addSelectedLinesToContext(lines: string, startLine: number, endLine: number) {
		if (
			selectedContext.find(
				(c) => c.type === 'code_piece' && c.title === `L${startLine}-L${endLine}`
			)
		) {
			return
		}
		selectedContext = [
			...selectedContext,
			{
				type: 'code_piece',
				title: `L${startLine}-L${endLine}`,
				startLine,
				endLine,
				content: lines,
				lang
			}
		]
	}

	export function fix() {
		if (!contextCodePath) {
			return
		}
		instructions = 'Fix the error'

		const codeContext = availableContext.find(
			(c) => c.type === 'code' && c.title === contextCodePath
		)
		const errorContext = availableContext.find((c) => c.type === 'error')

		if (codeContext && errorContext) {
			selectedContext = [codeContext, errorContext]
		}

		sendRequest()
	}

	export function askAi(
		prompt: string,
		options: { withCode?: boolean; withDiff?: boolean } = {
			withCode: true,
			withDiff: false
		}
	) {
		instructions = prompt
		const codeContext = availableContext.find(
			(c) => c.type === 'code' && c.title === contextCodePath
		)
		if (!codeContext) {
			return
		}
		selectedContext = [
			...(options.withCode === false ? [] : [codeContext]),
			...(options.withDiff
				? [
						{
							type: 'diff' as const,
							title: 'diff_with_last_deployed_version',
							content: lastDeployedCode ?? '',
							diff: diffLines(lastDeployedCode ?? '', code),
							lang
						}
					]
				: [])
		]
		sendRequest({
			removeDiff: options.withDiff,
			addBackCode: options.withCode === false
		})
		if (options.withDiff) {
			dispatch('showDiffMode')
		}
	}

	export function focusTextArea() {
		aiChatDisplay?.focusInput()
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
			savedChats = chats.reduce(
				(acc, chat) => {
					acc[chat.id] = chat
					return acc
				},
				{} as typeof savedChats
			)
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
					contextElements: selectedContext.filter((c) => c.type === 'code')
				}
			]
		: displayMessages}
	bind:instructions
	on:sendRequest={() => sendRequest()}
	on:saveAndClear={saveAndClear}
	on:deletePastChat={(e) => deletePastChat(e.detail.id)}
	on:loadPastChat={(e) => loadPastChat(e.detail.id)}
	on:analyzeChanges={() => {
		askAi(
			'Based on the changes I made to the code, look for potential issues and recommend better solutions',
			{ withDiff: true }
		)
	}}
	on:explainChanges={() =>
		askAi('Explain the changes I made to the code from the last diff', {
			withCode: false,
			withDiff: true
		})}
	on:suggestImprovements={() =>
		askAi('Look for potential issues and recommend better solutions in the actual code')}
	hasDiff={!!lastDeployedCode && lastDeployedCode !== code}
	{diffMode}
>
	<slot name="header-left" slot="header-left" />
	<slot name="header-right" slot="header-right" />
</AIChatDisplay>
