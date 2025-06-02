<script lang="ts">
	import { writable, type Writable } from 'svelte/store'
	import AIChatDisplay from './AIChatDisplay.svelte'
	import {
		dbSchemaTool,
		prepareScriptSystemMessage,
		prepareScriptUserMessage,
		resourceTypeTool,
		type ScriptChatHelpers
	} from './script/core'
	import {
		chatRequest,
		type AIChatContext,
		type DisplayMessage,
		type Tool,
		type ToolCallbacks
	} from './shared'
	import { onDestroy, setContext, untrack, type Snippet } from 'svelte'
	import { type OpenFlow, type ScriptLang } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import ContextManager, { type ScriptOptions } from './ContextManager.svelte'
	import HistoryManager from './HistoryManager.svelte'
	import {
		flowTools,
		prepareFlowSystemMessage,
		prepareFlowUserMessage,
		type FlowAIChatHelpers
	} from './flow/core'
	import type {
		ChatCompletionMessageParam,
		ChatCompletionSystemMessageParam
	} from 'openai/resources/index.mjs'
<<<<<<< HEAD
	import { chatMode, copilotSessionModel, dbSchemas, workspaceStore } from '$lib/stores'
=======
	import {
		navigatorTools,
		prepareNavigatorSystemMessage,
		prepareNavigatorUserMessage
	} from './navigator/core'
>>>>>>> 332040744 (integrate navigator mode)
	interface Props {
		scriptOptions?: ScriptOptions
		flowHelpers?: FlowAIChatHelpers & {
			getFlow: () => OpenFlow
		}
		navigatorMode?: boolean
		showDiffMode?: () => void
		applyCode?: (code: string) => void
		headerLeft?: Snippet
		headerRight?: Snippet
	}

	let {
		scriptOptions,
		flowHelpers,
		applyCode,
		showDiffMode,
		headerLeft,
		headerRight,
		navigatorMode = false
	}: Props = $props()

	let instructions = $state('')
	let loading = writable(false)
	let currentReply: Writable<string> = writable('')
	let allowedModes = $derived({
		script: scriptOptions !== undefined,
		flow: flowHelpers !== undefined,
		navigator: navigatorMode
	})
<<<<<<< HEAD

	async function updateMode(currentMode: 'script' | 'flow') {
		if (!allowedModes[currentMode]) {
			chatMode.set(currentMode === 'script' ? 'flow' : 'script')
=======
	let mode: 'script' | 'flow' | 'navigator' = $state(
		flowHelpers ? 'flow' : scriptOptions ? 'script' : 'navigator'
	)

	async function updateMode(currentMode: 'script' | 'flow' | 'navigator') {
		if (!allowedModes[currentMode] && Object.keys(allowedModes).length === 1) {
			const firstKey = Object.keys(allowedModes)[0]
			mode = firstKey as 'script' | 'flow' | 'navigator'
>>>>>>> 332040744 (integrate navigator mode)
		}
	}
	$effect(() => {
		updateMode(untrack(() => $chatMode))
	})

	let displayMessages: DisplayMessage[] = $state([])
	let abortController: AbortController | undefined = undefined
	let messages: ChatCompletionMessageParam[] = $state([])

	setContext<AIChatContext>('AIChatContext', {
		loading,
		currentReply,
		canApplyCode: () => allowedModes.script,
		applyCode
	})

	export async function sendRequest(
		options: {
			removeDiff?: boolean
			addBackCode?: boolean
			instructions?: string
			mode?: 'script' | 'flow' | 'navigator'
			lang?: ScriptLang | 'bunnative'
			isPreprocessor?: boolean
		} = {}
	) {
		if (options.mode) {
			$chatMode = options.mode
		}
		if (options.instructions) {
			instructions = options.instructions
		}
		if (!instructions.trim()) {
			return
		}
		try {
			const oldSelectedContext = contextManager?.getSelectedContext() ?? []
			if ($chatMode === 'script') {
				contextManager?.updateContextOnRequest(options)
			}
			loading.set(true)
			aiChatDisplay?.enableAutomaticScroll()
			abortController = new AbortController()

			displayMessages = [
				...displayMessages,
				{
					role: 'user',
					content: instructions,
					contextElements: $chatMode === 'script' ? oldSelectedContext : undefined
				}
			]
			const oldInstructions = instructions
			instructions = ''

			const systemMessage =
<<<<<<< HEAD
				$chatMode === 'script' ? prepareScriptSystemMessage() : prepareFlowSystemMessage()
=======
				mode === 'script'
					? prepareScriptSystemMessage()
					: mode === 'flow'
						? prepareFlowSystemMessage()
						: prepareNavigatorSystemMessage()
>>>>>>> 332040744 (integrate navigator mode)

			if ($chatMode === 'flow' && !flowHelpers) {
				throw new Error('No flow helpers passed')
			}

			if ($chatMode === 'script' && !scriptOptions && !options.lang) {
				throw new Error('No script options passed')
			}

			const lang = scriptOptions?.lang ?? options.lang ?? 'bun'
			const isPreprocessor = scriptOptions?.path === 'preprocessor' || options.isPreprocessor

			const userMessage =
				$chatMode === 'flow'
					? prepareFlowUserMessage(oldInstructions, flowHelpers!.getFlow())
					: mode === 'navigator'
						? prepareNavigatorUserMessage(oldInstructions)
						: await prepareScriptUserMessage(oldInstructions, lang, oldSelectedContext, {
								isPreprocessor
							})

			messages.push(userMessage)
			await historyManager.saveChat(displayMessages, messages)

			$currentReply = ''

			const params: {
				systemMessage: ChatCompletionSystemMessageParam
				messages: ChatCompletionMessageParam[]
				abortController: AbortController
				callbacks: ToolCallbacks & {
					onNewToken: (token: string) => void
					onMessageEnd: () => void
				}
			} = {
				systemMessage,
				messages,
				abortController,
				callbacks: {
					onNewToken: (token) => currentReply.update((prev) => prev + token),
					onMessageEnd: () => {
						if ($currentReply) {
							displayMessages = [
								...displayMessages,
								{
									role: 'assistant',
									content: $currentReply,
									contextElements:
										$chatMode === 'script'
											? oldSelectedContext.filter((c) => c.type === 'code')
											: undefined
								}
							]
						}
						currentReply.set('')
					},
					onToolCall: (id, content) => {
						displayMessages = [...displayMessages, { role: 'tool', tool_call_id: id, content }]
					},
					onFinishToolCall: (id, content) => {
						console.log('onFinishToolCall', id, content)
						const existingIdx = displayMessages.findIndex(
							(m) => m.role === 'tool' && m.tool_call_id === id
						)
						if (existingIdx !== -1) {
							displayMessages[existingIdx].content = content
						} else {
							displayMessages.push({ role: 'tool', tool_call_id: id, content })
						}
					}
				}
			}

			if ($chatMode === 'flow') {
				if (!flowHelpers) {
					throw new Error('No flow helpers found')
				}
				await chatRequest({
					...params,
					tools: flowTools,
					helpers: flowHelpers
				})
			} else if (mode === 'script') {
				const tools: Tool<ScriptChatHelpers>[] = []
				if (['python3', 'php', 'bun', 'deno', 'nativets', 'bunnative'].includes(lang)) {
					tools.push(resourceTypeTool)
				}
				if (oldSelectedContext.filter((c) => c.type === 'db').length > 0) {
					tools.push(dbSchemaTool)
				}
				await chatRequest({
					...params,
					tools,
					helpers: {
						getLang: () => lang
					}
				})
			} else if (mode === 'navigator') {
				await chatRequest({
					...params,
					tools: navigatorTools,
					helpers: {}
				})
			}

			if ($currentReply) {
				// just in case the onMessageEnd is not called (due to an error for instance)
				displayMessages = [
					...displayMessages,
					{
						role: 'assistant',
						content: $currentReply,
						contextElements:
							$chatMode === 'script'
								? oldSelectedContext.filter((c) => c.type === 'code')
								: undefined
					}
				]
				currentReply.set('')
			}

			await historyManager.saveChat(displayMessages, messages)
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

	export function addSelectedLinesToContext(lines: string, startLine: number, endLine: number) {
		contextManager?.addSelectedLinesToContext(lines, startLine, endLine)
	}

	export function fix() {
		instructions = 'Fix the error'
		contextManager?.setFixContext()
		sendRequest()
	}

	export function askAi(
		prompt: string,
		options: { withCode?: boolean; withDiff?: boolean } = {
			withCode: true,
			withDiff: false
		}
	) {
		if (!scriptOptions) {
			throw new Error('No script options passed')
		}
		instructions = prompt
		contextManager.setAskAiContext(options)
		sendRequest({
			removeDiff: options.withDiff,
			addBackCode: options.withCode === false
		})
		if (options.withDiff) {
			showDiffMode?.()
		}
	}

	export function focusTextArea() {
		aiChatDisplay?.focusInput()
	}

	const historyManager = new HistoryManager()
	historyManager.init()

	onDestroy(() => {
		cancel()
		historyManager.close()
	})

	let aiChatDisplay: AIChatDisplay | undefined = $state(undefined)
	// let contextManager: ContextManager | undefined = $state(undefined)

	const contextManager = new ContextManager()

	$effect(() => {
		if (scriptOptions) {
			contextManager.updateAvailableContext(
				scriptOptions,
				$dbSchemas,
				$workspaceStore ?? '',
				!$copilotSessionModel?.model.endsWith('/thinking'),
				untrack(() => contextManager.getSelectedContext())
			)
		}
	})

	$effect(() => {
		displayMessages = ContextManager.updateDisplayMessages(
			untrack(() => displayMessages),
			$dbSchemas
		)
	})
</script>

<AIChatDisplay
	bind:this={aiChatDisplay}
	{allowedModes}
	pastChats={historyManager.getPastChats()}
	bind:selectedContext={
		() => contextManager.getSelectedContext(),
		(sc) => {
			scriptOptions && contextManager.setSelectedContext(sc)
		}
	}
	availableContext={contextManager.getAvailableContext()}
	messages={$currentReply
		? [
				...displayMessages,
				{
					role: 'assistant',
					content: $currentReply,
					contextElements: contextManager.getSelectedContext().filter((c) => c.type === 'code')
				}
			]
		: displayMessages}
	bind:instructions
	{sendRequest}
	saveAndClear={async () => {
		await historyManager.save(displayMessages, messages)
		displayMessages = []
		messages = []
	}}
	deletePastChat={historyManager.deletePastChat}
	loadPastChat={(id) => {
		const chat = historyManager.loadPastChat(id)
		if (chat) {
			displayMessages = ContextManager.updateDisplayMessages(chat.displayMessages, $dbSchemas)
			messages = chat.actualMessages
			aiChatDisplay?.enableAutomaticScroll()
		}
	}}
	{cancel}
	{askAi}
	{headerLeft}
	{headerRight}
	hasDiff={scriptOptions &&
		!!scriptOptions.lastDeployedCode &&
		scriptOptions.lastDeployedCode !== scriptOptions.code}
	diffMode={scriptOptions?.diffMode ?? false}
></AIChatDisplay>
