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
	import { type ScriptLang } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import ContextManager from './ContextManager.svelte'
	import HistoryManager from './HistoryManager.svelte'
	import { flowTools, prepareFlowSystemMessage, prepareFlowUserMessage } from './flow/core'
	import type {
		ChatCompletionMessageParam,
		ChatCompletionSystemMessageParam
	} from 'openai/resources/index.mjs'
	import {
		chatMode,
		copilotInfo,
		copilotSessionModel,
		dbSchemas,
		flowAiChatHelpersStore,
		scriptEditorOptionsStore,
		scriptEditorShowDiffMode,
		userStore,
		workspaceStore
	} from '$lib/stores'
	import {
		navigatorTools,
		prepareNavigatorSystemMessage,
		prepareNavigatorUserMessage
	} from './navigator/core'
	import { globalChatInitialInput } from '$lib/stores'
	import { AIChatService } from './AIChatManager.svelte'

	interface Props {
		headerLeft?: Snippet
		headerRight?: Snippet
	}

	let { headerLeft, headerRight }: Props = $props()

	const isAdmin = $derived($userStore?.is_admin || $userStore?.is_super_admin)
	const hasCopilot = $derived($copilotInfo.enabled)
	const disabledMessage = $derived(
		hasCopilot
			? ''
			: isAdmin
				? `Enable Windmill AI in your [workspace settings](${base}/workspace_settings?tab=ai) to use this chat`
				: 'Ask an admin to enable Windmill AI in this workspace to use this chat'
	)

	const suggestions = [
		'Where can i see my latest runs?',
		'How do i trigger a script with a webhook endpoint?',
		'How can I connect to a database?',
		'How do I schedule a recurring job?'
	]

	let instructions = $state('')
	let loading = writable(false)
	let currentReply: Writable<string> = writable('')
	let allowedModes = $derived({
		script: $scriptEditorOptionsStore !== undefined,
		flow: $flowAiChatHelpersStore !== undefined,
		navigator: true
	})

	async function updateMode(currentMode: 'script' | 'flow' | 'navigator') {
		if (
			!allowedModes[currentMode] &&
			Object.keys(allowedModes).filter((k) => allowedModes[k]).length === 1
		) {
			const firstKey = Object.keys(allowedModes).filter((k) => allowedModes[k])[0]
			chatMode.set(firstKey as 'script' | 'flow' | 'navigator')
		}
	}

	$effect(() => {
		updateMode(untrack(() => $chatMode))
	})

	let displayMessages: DisplayMessage[] = $state([])
	let abortController: AbortController | undefined = undefined
	let messages: ChatCompletionMessageParam[] = $state([])

	$effect(() => {
		if ($globalChatInitialInput.length > 0) {
			instructions = $globalChatInitialInput
			globalChatInitialInput.set('')
			sendRequest()
		}
	})

	setContext<AIChatContext>('AIChatContext', {
		loading,
		currentReply,
		canApplyCode: () => allowedModes.script && $chatMode === 'script'
	})

	export async function generateStep(moduleId: string, lang: ScriptLang, instructions: string) {
		$flowAiChatHelpersStore?.selectStep(moduleId)
		await sendRequest({
			instructions: instructions,
			mode: 'script',
			lang: lang,
			isPreprocessor: moduleId === 'preprocessor'
		})
	}

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
				$chatMode === 'script'
					? prepareScriptSystemMessage()
					: $chatMode === 'flow'
						? prepareFlowSystemMessage()
						: prepareNavigatorSystemMessage()

			if ($chatMode === 'flow' && !$flowAiChatHelpersStore) {
				throw new Error('No flow helpers passed')
			}

			if ($chatMode === 'script' && !$scriptEditorOptionsStore && !options.lang) {
				throw new Error('No script options passed')
			}

			const lang = $scriptEditorOptionsStore?.lang ?? options.lang ?? 'bun'
			const isPreprocessor =
				$scriptEditorOptionsStore?.path === 'preprocessor' || options.isPreprocessor

			const userMessage =
				$chatMode === 'flow'
					? prepareFlowUserMessage(oldInstructions, $flowAiChatHelpersStore!.getFlowAndSelectedId())
					: $chatMode === 'navigator'
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
				if (!$flowAiChatHelpersStore) {
					throw new Error('No flow helpers found')
				}
				await chatRequest({
					...params,
					tools: flowTools,
					helpers: $flowAiChatHelpersStore
				})
			} else if ($chatMode === 'script') {
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
			} else if ($chatMode === 'navigator') {
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
		if (!$scriptEditorOptionsStore) {
			throw new Error('No script options passed')
		}
		instructions = prompt
		contextManager.setAskAiContext(options)
		sendRequest({
			removeDiff: options.withDiff,
			addBackCode: options.withCode === false
		})
		if (options.withDiff) {
			$scriptEditorShowDiffMode?.()
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
		if ($scriptEditorOptionsStore) {
			contextManager.updateAvailableContext(
				$scriptEditorOptionsStore,
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

<svelte:window
	on:keydown={(e) => {
		if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
			e.preventDefault()
			AIChatService.open = !AIChatService.open
		}
	}}
/>

<AIChatDisplay
	bind:this={aiChatDisplay}
	{allowedModes}
	pastChats={historyManager.getPastChats()}
	bind:selectedContext={
		() => contextManager.getSelectedContext(),
		(sc) => {
			$scriptEditorOptionsStore && contextManager.setSelectedContext(sc)
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
	hasDiff={$scriptEditorOptionsStore &&
		!!$scriptEditorOptionsStore.lastDeployedCode &&
		$scriptEditorOptionsStore.lastDeployedCode !== $scriptEditorOptionsStore.code}
	diffMode={$scriptEditorOptionsStore?.diffMode ?? false}
	disabled={!hasCopilot}
	{disabledMessage}
	{suggestions}
></AIChatDisplay>
