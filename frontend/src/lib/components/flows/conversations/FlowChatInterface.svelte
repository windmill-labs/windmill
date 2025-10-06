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

	const conversationsCache = $state<Record<string, ChatMessage[]>>({})

	// Auto-scroll to bottom when messages change
	$effect(() => {
		const scroll = async () => {
			if (messages.length > 0) {
				await tick()
				scrollToBottom()
			}
		}
		scroll()
	})

	// Cleanup EventSource on unmount
	$effect(() => {
		return () => {
			if (currentEventSource) {
				currentEventSource.close()
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

	async function pollJobResult(jobId: string, messageId: string) {
		try {
			const result = await waitJob(jobId)

			// Job completed successfully, update the message with the result
			messages = messages.map((msg) =>
				msg.id === messageId
					? {
							...msg,
							loading: false,
							content: formatJobResult(result)
						}
					: msg
			)
		} catch (error) {
			console.error('Error polling job result:', error)

			// Job failed, update the message with error
			messages = messages.map((msg) =>
				msg.id === messageId
					? {
							...msg,
							loading: false,
							content: 'Error: ' + (error?.message || String(error))
						}
					: msg
			)
		}
	}

	function formatJobResult(result: any): string {
		if (result === null || result === undefined) {
			return 'No result returned'
		}

		// If result is an object with an output field, use that
		if (typeof result === 'object' && result.output !== undefined) {
			if (typeof result.output === 'string') {
				return result.output
			}
			return JSON.stringify(result.output, null, 2)
		}

		if (typeof result === 'string') {
			return result
		}

		if (typeof result === 'object') {
			return JSON.stringify(result, null, 2)
		}

		return String(result)
	}

	function parseStreamDeltas(streamData: string): string {
		const lines = streamData.trim().split('\n')
		let content = ''
		for (const line of lines) {
			if (!line.trim()) continue
			try {
				const parsed = JSON.parse(line)
				if (parsed.type === 'token_delta' && parsed.content) {
					content += parsed.content
				}
			} catch (e) {
				console.error('Failed to parse stream line:', line, e)
			}
		}
		return content
	}

	async function sendMessage() {
		if (!inputMessage.trim() || isLoading) return

		const isNewConversation = messages.length === 0

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

		try {
			// Add assistant message placeholder
			const assistantMessageId = crypto.randomUUID()
			const assistantMessage: ChatMessage = {
				id: assistantMessageId,
				content: '',
				created_at: new Date().toISOString(),
				message_type: 'assistant',
				conversation_id: currentConversationId,
				job_id: '',
				loading: true
			}

			messages = [...messages, assistantMessage]

			if (useStreaming && path) {
				// Close any existing EventSource
				if (currentEventSource) {
					currentEventSource.close()
				}

				// Track stream state for this message
				let accumulatedContent = ''

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

					eventSource.onmessage = (event) => {
						try {
							const data = JSON.parse(event.data)

							if (data.type === 'update') {
								// Process new stream content
								if (data.new_result_stream) {
									const newContent = parseStreamDeltas(data.new_result_stream)
									accumulatedContent += newContent
									// Update message content
									messages = messages.map((msg) =>
										msg.id === assistantMessageId
											? {
													...msg,
													content: accumulatedContent,
													loading: accumulatedContent.length === 0
												}
											: msg
									)
								}

								// Handle completion
								if (data.completed && data.only_result) {
									const finalContent =
										data.only_result.output ||
										accumulatedContent ||
										JSON.stringify(data.only_result.error)
									messages = messages.map((msg) =>
										msg.id === assistantMessageId
											? {
													...msg,
													content: finalContent,
													loading: false
												}
											: msg
									)
									eventSource.close()
									currentEventSource = undefined
									isLoading = false
								}
							}
						} catch (error) {
							console.error('Error processing stream event:', error)
						}
					}

					eventSource.onerror = (error) => {
						console.error('EventSource error:', error)
						messages = messages.map((msg) =>
							msg.id === assistantMessageId
								? {
										...msg,
										content: accumulatedContent || 'Stream error occurred',
										loading: false
									}
								: msg
						)
						eventSource.close()
						currentEventSource = undefined
						isLoading = false
						sendUserToast('Stream error occurred', true)
					}
				} catch (error) {
					console.error('Stream connection error:', error)
					messages = messages.map((msg) =>
						msg.id === assistantMessageId
							? {
									...msg,
									content: 'Failed to connect to stream',
									loading: false
								}
							: msg
					)
					isLoading = false
					sendUserToast('Failed to connect to stream', true)
				}
			} else {
				const jobId = await onRunFlow(messageContent, currentConversationId)
				if (!jobId) {
					console.error('No jobId returned from onRunFlow')
					return
				}
				pollJobResult(jobId, assistantMessageId)
			}

			scrollToBottom()
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
			class="flex-1 overflow-y-auto p-4 space-y-4 bg-background"
			onscroll={handleScroll}
		>
			{#if deploymentInProgress}
				<Alert type="warning" title="Deployment in progress" size="xs" />
			{/if}
			{#if isLoadingMessages}
				<div class="flex items-center justify-center">
					<Loader2 size={24} class="animate-spin" />
				</div>
			{:else if messages.length === 0}
				<div class="text-center text-tertiary py-8">
					<MessageCircle size={48} class="mx-auto mb-4 opacity-50" />
					<p class="text-lg font-medium">Start a conversation</p>
					<p class="text-sm">Send a message to run the flow and see the results</p>
				</div>
			{:else}
				<div class="max-w-7xl mx-auto">
					{#each messages as message (message.id)}
						<FlowChatMessage {message} />
					{/each}
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
					placeholder={deploymentInProgress
						? 'Chat is disabled during deployment...'
						: 'Type your message here...'}
					class="flex-1 min-h-[24px] max-h-32 resize-none !border-0 !bg-transparent text-sm placeholder-gray-400 !outline-none !ring-0 p-0 !shadow-none focus:!border-0 focus:!outline-none focus:!ring-0 focus:!shadow-none"
					disabled={deploymentInProgress}
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
