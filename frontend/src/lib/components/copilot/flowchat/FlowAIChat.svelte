<script lang="ts">
	import {
		chatRequest,
		prepareUserMessage,
		type FlowDisplayMessage,
		type FlowManipulationHelpers
	} from './core'
	import { getContext, onDestroy, setContext } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { writable, type Writable } from 'svelte/store'
	import FlowAIChatDisplay from './FlowAIChatDisplay.svelte'
	import type { FlowEditorContext } from '$lib/components/flows/types'

	let {
		flowManipulationHelpers
	}: {
		flowManipulationHelpers: FlowManipulationHelpers
	} = $props()

	let instructions = $state('')
	let loading = writable(false)
	let currentReply: Writable<string> = writable('')

	let messages: { role: 'user' | 'assistant' | 'system'; content: string }[] = []

	const { flowStore, currentScriptEditor } = getContext<FlowEditorContext>('FlowEditorContext')

	setContext('AIChatContext', {
		loading,
		currentReply,
		applyCode: (code: string) => {
			if ($currentScriptEditor) {
				$currentScriptEditor.reviewAndApplyCode(code)
			} else {
				console.error('No script editor found')
			}
		}
	})

	let displayMessages: FlowDisplayMessage[] = $state([])
	let abortController: AbortController | undefined = $state(undefined)
	let mode: 'flow' | 'script' = $state('flow')

	async function sendRequest(options: { removeDiff?: boolean; addBackCode?: boolean } = {}) {
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
					content: instructions
				}
			]
			const oldInstructions = instructions
			instructions = ''
			const userMessage = prepareUserMessage(oldInstructions, $flowStore.value)

			messages.push({ role: 'user', content: userMessage })

			$currentReply = ''
			await chatRequest(
				messages,
				abortController,
				{
					onNewToken: (token) => currentReply.update((prev) => prev + token),
					onMessageEnd: () => {
						if ($currentReply) {
							messages.push({ role: 'assistant', content: $currentReply })
							displayMessages = [
								...displayMessages,
								{
									role: 'assistant',
									content: $currentReply
								}
							]
						}
						$currentReply = ''
					},
					onToolCall: (id, content) => {
						displayMessages = [...displayMessages, { role: 'tool', tool_call_id: id, content }]
					},
					onFinishToolCall: (id, content) => {
						const existingToolCall = displayMessages.find(
							(m) => m.role === 'tool' && m.tool_call_id === id
						)
						if (existingToolCall) {
							existingToolCall.content = content
						} else {
							displayMessages.push({ role: 'tool', tool_call_id: id, content })
						}
					},
					getMode: () => mode,
					setMode: (m) => (mode = m),
					setCode: (code: string) => {
						if ($currentScriptEditor) {
							$currentScriptEditor.setCode(code)
						} else {
							console.error('No script editor found')
						}
					}
				},
				flowManipulationHelpers
			)
			if ($currentReply) {
				// just in case the onMessageEnd is not called (due to an error for instance)
				messages.push({ role: 'assistant', content: $currentReply })
				displayMessages = [
					...displayMessages,
					{
						role: 'assistant',
						content: $currentReply
					}
				]
				currentReply.set('')
			}
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
	async function clear() {
		displayMessages = []
		messages = []
	}

	export function focusTextArea() {
		aiChatDisplay?.focusInput()
	}

	onDestroy(() => {
		cancel()
	})

	let aiChatDisplay: FlowAIChatDisplay | undefined = undefined
</script>

<FlowAIChatDisplay
	bind:this={aiChatDisplay}
	messages={$currentReply
		? [
				...displayMessages,
				{
					role: 'assistant',
					content: $currentReply
				}
			]
		: displayMessages}
	bind:instructions
	{sendRequest}
	{clear}
/>
