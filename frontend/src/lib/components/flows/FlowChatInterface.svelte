<script lang="ts">
	import { Button, Alert } from '$lib/components/common'
	import { MessageCircle, Loader2, ArrowUp } from 'lucide-svelte'
	import { FlowConversationService, type FlowConversationMessage } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import autosize from '$lib/autosize'
	import { waitJob } from '$lib/components/waitJob'
	import { tick } from 'svelte'

	interface Props {
		onRunFlow: (args: Record<string, any>, conversationId?: string) => Promise<string | undefined>
		refreshConversations?: () => Promise<void>
		conversationId?: string
		deploymentInProgress?: boolean
		createConversation?: (options: { clearMessages?: boolean }) => Promise<string>
	}

	interface ChatMessage extends FlowConversationMessage {
		loading?: boolean
	}

	let {
		onRunFlow,
		conversationId,
		refreshConversations,
		deploymentInProgress = false,
		createConversation
	}: Props = $props()

	let messages = $state<ChatMessage[]>([])
	let inputMessage = $state('')
	let isLoading = $state(false)
	let isLoadingMessages = $state(false)
	let messagesContainer: HTMLDivElement | undefined = $state()
	let page = $state(1)
	let perPage = 50
	let hasMoreMessages = $state(true)
	let loadingMoreMessages = $state(false)
	let scrollTimeout: ReturnType<typeof setTimeout> | undefined = undefined
	let inputElement: HTMLTextAreaElement | undefined = $state()

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
		hasMoreMessages = true
	}

	export async function loadConversationMessages(convId: string) {
		page = 1
		hasMoreMessages = true
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

		if (typeof result === 'string') {
			return result
		}

		if (typeof result === 'object') {
			return JSON.stringify(result, null, 2)
		}

		return String(result)
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
			// Run the flow with the user message as input
			// The backend will automatically store messages when the flow runs
			const jobId = await onRunFlow({ user_message: messageContent }, currentConversationId)

			if (!jobId) {
				console.error('No jobId returned from onRunFlow')
				return
			}

			// Add assistant message placeholder
			const assistantMessageId = crypto.randomUUID()
			const assistantMessage: ChatMessage = {
				id: assistantMessageId,
				content: '',
				created_at: new Date().toISOString(),
				message_type: 'assistant',
				conversation_id: currentConversationId,
				job_id: jobId,
				loading: true
			}

			messages = [...messages, assistantMessage]
			scrollToBottom()

			// Start polling for job result
			pollJobResult(jobId, assistantMessageId)
		} catch (error) {
			console.error('Error running flow:', error)
			sendUserToast('Failed to run flow: ' + error, true)
		} finally {
			isLoading = false
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
				{#each messages as message (message.id)}
					<div class="flex {message.message_type === 'user' ? 'justify-end' : 'justify-start'}">
						<div
							class="max-w-[80%] rounded-lg p-3 {message.message_type === 'user'
								? 'bg-surface-secondary'
								: 'bg-surface border border-gray-200 dark:border-gray-600'}"
						>
							{#if message.message_type === 'user'}
								<p class="whitespace-pre-wrap">{message.content}</p>
							{:else if message.loading}
								<div class="flex items-center gap-2 text-tertiary">
									<Loader2 size={16} class="animate-spin" />
									<span>Processing...</span>
								</div>
							{:else if message.content}
								<div class="whitespace-pre-wrap">
									{message.content}
								</div>
							{:else}
								<p class="text-tertiary">No result</p>
							{/if}
						</div>
					</div>
				{/each}
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
					class:cursor-not-allowed={deploymentInProgress}
					disabled={isLoading || deploymentInProgress}
					rows={3}
				></textarea>
				<div class="flex-shrink-0 pr-2">
					<Button
						color="blue"
						size="xs2"
						btnClasses="!rounded-full !p-1.5"
						startIcon={{ icon: isLoading ? Loader2 : ArrowUp }}
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
