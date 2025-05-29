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
	import ContextManager from './ContextManager.svelte'
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
	interface Props {
		scriptOptions?: {
			lang: ScriptLang | 'bunnative'
			code: string
			error: string | undefined
			args: Record<string, any>
			path: string | undefined
			lastSavedCode?: string | undefined
			lastDeployedCode?: string | undefined
			diffMode: boolean
			applyCode: (code: string) => void
			showDiffMode: () => void
		}
		flowHelpers?: FlowAIChatHelpers & {
			getFlow: () => OpenFlow
		}
		headerLeft?: Snippet
		headerRight?: Snippet
	}

	let { scriptOptions, flowHelpers, headerLeft, headerRight }: Props = $props()

	let instructions = $state('')
	let loading = writable(false)
	let currentReply: Writable<string> = writable('')
	let allowedModes = $derived({
		script: scriptOptions !== undefined,
		flow: flowHelpers !== undefined
	})
	$inspect(allowedModes)
	let mode: 'script' | 'flow' = $state(flowHelpers ? 'flow' : 'script')

	async function updateMode(currentMode: 'script' | 'flow') {
		if (!allowedModes[currentMode]) {
			mode = currentMode === 'script' ? 'flow' : 'script'
		}
	}
	$effect(() => {
		updateMode(untrack(() => mode))
	})
	let displayMessages: DisplayMessage[] = $state([])
	let abortController: AbortController | undefined = undefined
	let messages: ChatCompletionMessageParam[] = $state([])

	setContext<AIChatContext>('AIChatContext', {
		loading,
		currentReply,
		canApplyCode: () => allowedModes.script,
		applyCode: scriptOptions?.applyCode ?? (() => {})
	})

	async function sendRequest(options: { removeDiff?: boolean; addBackCode?: boolean } = {}) {
		if (!instructions.trim()) {
			return
		}
		try {
			const oldSelectedContext = contextManager?.getSelectedContext() ?? []
			if (mode === 'script') {
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
					contextElements: mode === 'script' ? oldSelectedContext : undefined
				}
			]
			const oldInstructions = instructions
			instructions = ''

			const systemMessage =
				mode === 'script' ? prepareScriptSystemMessage() : prepareFlowSystemMessage()

			if (mode === 'flow' && !flowHelpers) {
				throw new Error('No flow helpers passed')
			}

			if (mode === 'script' && !scriptOptions) {
				throw new Error('No script options passed')
			}

			const userMessage =
				mode === 'flow'
					? prepareFlowUserMessage(oldInstructions, flowHelpers!.getFlow())
					: await prepareScriptUserMessage(oldInstructions, scriptOptions!.lang, oldSelectedContext)

			messages.push({ role: 'user', content: userMessage })
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
										mode === 'script'
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

			if (mode === 'flow') {
				if (!flowHelpers) {
					throw new Error('No flow helpers found')
				}
				await chatRequest({
					...params,
					tools: flowTools,
					helpers: flowHelpers
				})
			} else {
				const tools: Tool<ScriptChatHelpers>[] = []
				if (
					['python3', 'php', 'bun', 'deno', 'nativets', 'bunnative'].includes(scriptOptions!.lang)
				) {
					tools.push(resourceTypeTool)
				}
				if (oldSelectedContext.filter((c) => c.type === 'db').length > 0) {
					tools.push(dbSchemaTool)
				}
				await chatRequest({
					...params,
					tools,
					helpers: {
						getLang: () => scriptOptions!.lang
					}
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
							mode === 'script' ? oldSelectedContext.filter((c) => c.type === 'code') : undefined
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
		contextManager?.setAskAiContext(options)
		sendRequest({
			removeDiff: options.withDiff,
			addBackCode: options.withCode === false
		})
		if (options.withDiff) {
			scriptOptions.showDiffMode()
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
	let contextManager: ContextManager | undefined = $state(undefined)
</script>

{#if mode === 'script' && scriptOptions}
	<ContextManager
		bind:this={contextManager}
		code={scriptOptions.code}
		lang={scriptOptions.lang}
		path={scriptOptions.path}
		args={scriptOptions.args}
		lastSavedCode={scriptOptions.lastSavedCode}
		lastDeployedCode={scriptOptions.lastDeployedCode}
		error={scriptOptions.error}
		bind:displayMessages
	/>
{/if}

<AIChatDisplay
	bind:this={aiChatDisplay}
	bind:mode
	{allowedModes}
	pastChats={historyManager.getPastChats()}
	bind:selectedContext={
		() => contextManager?.getSelectedContext() ?? [],
		(sc) => {
			contextManager?.setSelectedContext(sc)
		}
	}
	availableContext={contextManager?.getAvailableContext() ?? []}
	messages={$currentReply
		? [
				...displayMessages,
				{
					role: 'assistant',
					content: $currentReply,
					contextElements: contextManager?.getSelectedContext()?.filter((c) => c.type === 'code')
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
			displayMessages = chat.displayMessages
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
