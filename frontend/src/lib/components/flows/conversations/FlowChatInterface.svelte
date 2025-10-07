<script lang="ts">
	import { Button, Alert } from '$lib/components/common'
	import { MessageCircle, Loader2, ArrowUp } from 'lucide-svelte'
	import { FlowConversationService, type FlowConversationMessage } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import autosize from '$lib/autosize'
	import { waitJob } from '$lib/components/waitJob'
	import { tick } from 'svelte'
	import FlowChatMessage from './FlowChatMessage.svelte'

	interface Props {
		onRunFlow: (userMessage: string, conversationId: string) => Promise<string | undefined>
		useStreaming?: boolean
		refreshConversations?: () => Promise<void>
		conversationId?: string
		deploymentInProgress?: boolean
		createConversation: (options: { clearMessages?: boolean }) => Promise<string>
		path?: string
	}

	interface ChatMessage extends FlowConversationMessage {
		loading?: boolean
		streaming?: boolean
		error?: boolean
	}

	let {
		onRunFlow,
		conversationId,
		refreshConversations,
		deploymentInProgress = false,
		createConversation,
		useStreaming = false,
		path
	}: Props = $props()

	let messages = $state<ChatMessage[]>([])
	let inputMessage = $state('')
	let isLoading = $state(false)
	let isLoadingMessages = $state(false)
	let messagesContainer: HTMLDivElement | undefined = $state()
	let page = $state(1)
	let perPage = 50
	let hasMoreMessages = $state(false)
	let loadingMoreMessages = $state(false)
	let scrollTimeout: ReturnType<typeof setTimeout> | undefined = undefined
	let inputElement: HTMLTextAreaElement | undefined = $state()
	let currentEventSource: EventSource | undefined = $state()
	let pollingInterval: ReturnType<typeof setInterval> | undefined = $state()
	let streamingJobId: string | undefined = $state()
	let isFlowComplete = $state(false)
	let isWaitingForResponse = $state(false)

	const conversationsCache = $state<Record<string, ChatMessage[]>>({})

	// Cleanup EventSource and polling on unmount
	$effect(() => {
		return () => {
			if (currentEventSource) {
				currentEventSource.close()
			}
			if (pollingInterval) {
				clearInterval(pollingInterval)
				pollingInterval = undefined
			}
		}
	})

	export function fillInputMessage(message: string) {
		inputMessage = message
	}

	export function focusInput() {
		inputElement?.focus()
	}

	export function clearMessages() {
		messages = []
		inputMessage = ''
		page = 1
	}

	export async function loadConversationMessages(convId: string) {
		page = 1
		await loadMessages(true)
	}

	async function loadMessages(reset: boolean) {
		if (!$workspaceStore || !conversationId) return

		if (reset) {
			if (conversationsCache[conversationId]) {
				messages = conversationsCache[conversationId]
				return
			}
			isLoadingMessages = true
		} else {
			loadingMoreMessages = true
		}

		const pageToFetch = reset ? 1 : page + 1

		try {
			const previousScrollHeight = messagesContainer?.scrollHeight || 0

			const response = await FlowConversationService.listConversationMessages({
				workspace: $workspaceStore,
				conversationId: conversationId,
				page: pageToFetch,
				perPage: perPage
			})

			if (reset) {
				conversationsCache[conversationId] = response
				messages = response
				isLoadingMessages = false
				await new Promise((resolve) => setTimeout(resolve, 100))
				scrollToBottom()
			} else {
				messages = [...response, ...messages]
				page = pageToFetch
				// Restore scroll position
				await new Promise((resolve) => setTimeout(resolve, 50))
				if (messagesContainer) {
					messagesContainer.scrollTop = messagesContainer.scrollHeight - previousScrollHeight
				}
			}

			hasMoreMessages = response.length === perPage
		} catch (error) {
			console.error('Failed to load messages:', error)
			sendUserToast('Failed to load messages: ' + error)
		} finally {
			isLoadingMessages = false
			loadingMoreMessages = false
		}
	}

	function handleScroll() {
		if (scrollTimeout) clearTimeout(scrollTimeout)

		scrollTimeout = setTimeout(() => {
			if (!messagesContainer || !hasMoreMessages || loadingMoreMessages) return

			if (messagesContainer.scrollTop <= 10) {
				loadMessages(false)
			}
		}, 200)
	}

	function scrollToBottom() {
		if (messagesContainer) {
			messagesContainer.scrollTop = messagesContainer.scrollHeight
		}
	}

	function scrollToUserMessage(messageId: string) {
		if (!messagesContainer) return
		const messageElement = messagesContainer.querySelector(`[data-message-id="${messageId}"]`)
		if (messageElement) {
			messageElement.scrollIntoView({ behavior: 'smooth', block: 'start' })
		}
	}

	async function pollJobResult(jobId: string) {
		try {
			await waitJob(jobId)

			// Do a final poll to get all messages from database
			if (conversationId) {
				await pollConversationMessages(conversationId)
			}
			isWaitingForResponse = false
		} catch (error) {
			console.error('Error polling job result:', error)

			// Create error message
			const resultMessageId = crypto.randomUUID()
			isWaitingForResponse = false
			messages = [
				...messages,
				{
					id: resultMessageId,
					content: 'Error: ' + (error?.message || String(error)),
					created_at: new Date().toISOString(),
					message_type: 'assistant',
					conversation_id: conversationId!,
					job_id: jobId,
					loading: false
				}
			]
		} finally {
			stopPolling()
			isLoading = false
		}
	}

	function parseStreamDeltas(streamData: string): {
		type: string
		content: string
		error?: boolean
	} {
		let type = 'message'
		const lines = streamData.trim().split('\n')
		let content = ''
		let error = false
		for (const line of lines) {
			if (!line.trim()) continue
			try {
				const parsed = JSON.parse(line)
				if (parsed.type === 'tool_result') {
					type = 'tool_result'
					const toolName = parsed.function_name
					console.log('parsed', parsed, typeof parsed.success)
					error = !parsed.success
					content = error ? `Failed to use ${toolName} tool` : `Used ${toolName} tool`
				}
				if (parsed.type === 'token_delta' && parsed.content) {
					type = 'message'
					content += parsed.content
				}
			} catch (e) {
				console.error('Failed to parse stream line:', line, e)
			}
		}
		return { type, content, error }
	}

	async function pollConversationMessages(conversationId: string) {
		if (!$workspaceStore) return

		try {
			const lastId = messages[messages.length - 1].id
			console.log('lastId', lastId)
			const response = await FlowConversationService.listConversationMessages({
				workspace: $workspaceStore,
				conversationId: conversationId,
				page: 1,
				perPage: 50,
				afterId: lastId
			})

			// Filter out the streaming job's message and user messages
			const newMessages = response.filter(
				(msg) => msg.message_type !== 'user' && msg.job_id !== streamingJobId
			)

			// Update existing messages
			messages = messages.map((msg) => {
				if (msg.id && response.find((r) => r.id === msg.id)) {
					return response.find((r) => r.id === msg.id)!
				}
				return msg
			})

			// Add any new intermediate messages not already present
			for (const msg of newMessages) {
				if (!messages.find((m) => m.id === msg.id)) {
					// Insert in chronological order
					const insertIndex = messages.findIndex(
						(m) => new Date(m.created_at) > new Date(msg.created_at)
					)
					if (insertIndex === -1) {
						messages = [...messages, msg]
					} else {
						messages = [...messages.slice(0, insertIndex), msg, ...messages.slice(insertIndex)]
					}
				}
			}

			// Check if streaming job's message appeared (flow complete)
			if (streamingJobId && response.find((r) => r.job_id === streamingJobId)) {
				// set step name to the last message
				messages = [
					...messages.slice(0, messages.length - 1),
					{ ...messages[messages.length - 1], step_name: response[response.length - 1].step_name }
				]
				stopPolling()
				isFlowComplete = true
			}
		} catch (error) {
			console.error('Polling error:', error)
		}
	}

	function startPolling(conversationId: string) {
		if (pollingInterval) return
		pollingInterval = setInterval(() => {
			pollConversationMessages(conversationId)
		}, 1500) // Poll every 1.5 seconds
	}

	function stopPolling() {
		if (pollingInterval) {
			clearInterval(pollingInterval)
			pollingInterval = undefined
		}
	}

	async function sendMessage() {
		if (!inputMessage.trim() || isLoading) return

		const isNewConversation = messages.length === 0

		// Reset state for new message
		streamingJobId = undefined
		isFlowComplete = false
		stopPolling()

		// Generate a new conversation ID if we don't have one
		let currentConversationId = conversationId
		if (!conversationId) {
			const newConversationId = await createConversation?.({ clearMessages: false })
			currentConversationId = newConversationId
		}

		if (!currentConversationId) {
			console.error('No conversation ID found')
			return
		}

		// Invalidate the conversation cache
		delete conversationsCache[currentConversationId]

		const userMessage: ChatMessage = {
			id: crypto.randomUUID(),
			content: inputMessage.trim(),
			created_at: new Date().toISOString(),
			message_type: 'user',
			conversation_id: currentConversationId
		}

		messages = [...messages, userMessage]
		const messageContent = inputMessage.trim()
		inputMessage = ''
		isLoading = true
		isWaitingForResponse = true

		try {
			await tick()
			scrollToUserMessage(userMessage.id)

			if (useStreaming && path) {
				// Close any existing EventSource
				if (currentEventSource) {
					currentEventSource.close()
				}

				// Track stream state for this message
				let accumulatedContent = ''
				let assistantMessageId = ''

				try {
					// Encode the payload as base64
					const payload = { user_message: messageContent }
					const payloadBase64 = btoa(JSON.stringify(payload))

					// Build the EventSource URL
					const streamUrl = `/api/w/${$workspaceStore}/jobs/run_and_stream/f/${path}`
					const url = new URL(streamUrl, window.location.origin)
					url.searchParams.set('payload', payloadBase64)
					url.searchParams.set('memory_id', currentConversationId)
					url.searchParams.set('poll_delay_ms', '50')

					// Create EventSource connection
					const eventSource = new EventSource(url.toString())
					currentEventSource = eventSource

					// start polling
					startPolling(currentConversationId)

					eventSource.onmessage = async (event) => {
						try {
							const data = JSON.parse(event.data)

							if (data.type === 'update') {
								// Process new stream content
								if (data.new_result_stream) {
									// Stop polling since we are receiving last step streaming
									stopPolling()
									const {
										type,
										content: newContent,
										error
									} = parseStreamDeltas(data.new_result_stream)
									accumulatedContent += newContent

									// Create tool message if type is tool_result
									if (type === 'tool_result') {
										// set last message streaming to false
										messages = messages.map((msg) =>
											msg.id === messages[messages.length - 1].id
												? { ...msg, streaming: false }
												: msg
										)

										messages = [
											...messages,
											{
												id: crypto.randomUUID(),
												content: newContent,
												created_at: new Date().toISOString(),
												message_type: 'tool',
												conversation_id: currentConversationId,
												job_id: streamingJobId,
												loading: false,
												streaming: false,
												error
											}
										]
										// Reset assistant message ID since we are creating a tool message
										assistantMessageId = ''
										accumulatedContent = ''
									}

									// Create message on first content
									else if (
										type === 'message' &&
										assistantMessageId.length === 0 &&
										accumulatedContent.length > 0
									) {
										assistantMessageId = crypto.randomUUID()
										isWaitingForResponse = false
										messages = [
											...messages,
											{
												id: assistantMessageId,
												content: accumulatedContent,
												created_at: new Date().toISOString(),
												message_type: 'assistant',
												conversation_id: currentConversationId,
												job_id: streamingJobId,
												loading: false,
												streaming: true
											}
										]
									} else {
										// Update existing message
										messages = messages.map((msg) =>
											msg.id === assistantMessageId ? { ...msg, content: accumulatedContent } : msg
										)
									}
								}

								// Handle completion
								if (data.completed && data.only_result) {
									const finalContent =
										data.only_result.output ||
										accumulatedContent ||
										JSON.stringify(data.only_result.error)

									// If no message created yet (no stream), create it now
									if (assistantMessageId.length === 0) {
										assistantMessageId = crypto.randomUUID()
										isWaitingForResponse = false
										messages = [
											...messages,
											{
												id: assistantMessageId,
												content: finalContent,
												created_at: new Date().toISOString(),
												message_type: 'assistant',
												conversation_id: currentConversationId,
												job_id: streamingJobId,
												loading: false,
												streaming: false
											}
										]
									} else {
										messages = messages.map((msg) =>
											msg.id === assistantMessageId
												? {
														...msg,
														content: finalContent,
														loading: false,
														streaming: false,
														job_id: streamingJobId
													}
												: msg
										)
									}

									eventSource.close()
									currentEventSource = undefined
									isLoading = false

									// Do one final poll to ensure we have all messages
									// await pollConversationMessages(currentConversationId)
									// stopPolling()
								}
							}
						} catch (error) {
							console.error('Error processing stream event:', error)
						}
					}

					eventSource.onerror = (error) => {
						console.error('EventSource error:', error)
						isWaitingForResponse = false
						messages = [
							...messages,
							{
								id: crypto.randomUUID(),
								content: 'Stream error occurred',
								created_at: new Date().toISOString(),
								message_type: 'assistant',
								conversation_id: currentConversationId,
								job_id: '',
								loading: false,
								streaming: false
							}
						]
						eventSource.close()
						currentEventSource = undefined
						stopPolling()
						isLoading = false
						sendUserToast('Stream error occurred', true)
					}
				} catch (error) {
					console.error('Stream connection error:', error)
					isWaitingForResponse = false
					messages = [
						...messages,
						{
							id: crypto.randomUUID(),
							content: 'Failed to connect to stream',
							created_at: new Date().toISOString(),
							message_type: 'assistant',
							conversation_id: currentConversationId,
							job_id: '',
							loading: false,
							streaming: false
						}
					]
					stopPolling()
					isLoading = false
					sendUserToast('Failed to connect to stream', true)
				}
			} else {
				const jobId = await onRunFlow(messageContent, currentConversationId)
				if (!jobId) {
					console.error('No jobId returned from onRunFlow')
					return
				}
				// Start polling for intermediate messages in non-streaming mode too
				startPolling(currentConversationId)
				pollJobResult(jobId)
			}
		} catch (error) {
			console.error('Error running flow:', error)
			sendUserToast('Failed to run flow: ' + error, true)
		} finally {
			if (!useStreaming) {
				isLoading = false
			}
		}

		if (isNewConversation) {
			await refreshConversations?.()
		}

		await tick()
		focusInput()
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault()
			sendMessage()
		}
	}
</script>

<div class="flex flex-col h-full w-full">
	<div class="flex-1 flex flex-col min-h-0 w-full">
		<!-- Messages Container -->
		<div
			bind:this={messagesContainer}
			class="flex-1 overflow-y-auto p-4 bg-background"
			onscroll={handleScroll}
		>
			{#if deploymentInProgress}
				<Alert type="warning" title="Deployment in progress" size="xs" />
			{/if}
			{#if isLoadingMessages}
				<div class="flex items-center justify-center h-full">
					<Loader2 size={32} class="animate-spin" />
				</div>
			{:else if messages.length === 0}
				<div class="text-center text-tertiary flex items-center justify-center flex-col h-full">
					<MessageCircle size={48} class="mx-auto mb-4 opacity-50" />
					<p class="text-lg font-medium">Start a conversation</p>
					<p class="text-sm">Send a message to run the flow and see the results</p>
				</div>
			{:else}
				<div class="max-w-7xl mx-auto space-y-4">
					{#each messages as message (message.id)}
						<FlowChatMessage {message} />
					{/each}
					{#if isWaitingForResponse}
						<div class="flex items-center gap-2 text-tertiary">
							<Loader2 size={16} class="animate-spin" />
							<span class="text-sm">Processing...</span>
						</div>
					{/if}
				</div>
			{/if}
		</div>

		<!-- Chat Input -->
		<div class="p-2 bg-surface">
			<div
				class="flex items-center gap-2 rounded-lg border border-gray-200 dark:border-gray-600 bg-surface"
				class:opacity-50={deploymentInProgress}
			>
				<textarea
					bind:this={inputElement}
					bind:value={inputMessage}
					use:autosize
					onkeydown={handleKeyDown}
					placeholder="Type your message here..."
					class="flex-1 min-h-[24px] max-h-32 resize-none !border-0 !bg-transparent text-sm placeholder-gray-400 !outline-none !ring-0 p-0 !shadow-none focus:!border-0 focus:!outline-none focus:!ring-0 focus:!shadow-none"
					rows={3}
				></textarea>
				<div class="flex-shrink-0 pr-2">
					<Button
						color="blue"
						size="xs2"
						btnClasses="!rounded-full !p-1.5"
						startIcon={{ icon: ArrowUp }}
						disabled={!inputMessage?.trim() || isLoading || deploymentInProgress}
						on:click={sendMessage}
						iconOnly
						title={deploymentInProgress ? 'Deployment in progress' : 'Send message (Enter)'}
					/>
				</div>
			</div>
		</div>
	</div>
</div>
