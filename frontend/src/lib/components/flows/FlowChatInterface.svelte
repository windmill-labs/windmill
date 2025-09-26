<script lang="ts">
	import { Button } from '$lib/components/common'
	import { MessageCircle, Send, Loader2 } from 'lucide-svelte'
	import { JobService, FlowConversationService, type FlowConversationMessage } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import autosize from '$lib/autosize'

	interface Props {
		onRunFlow: (args: Record<string, any>, conversationId?: string) => Promise<string>
		refreshConversations?: () => Promise<void>
		conversationId?: string
	}

	interface ChatMessage extends FlowConversationMessage {
		loading?: boolean
	}

	let { onRunFlow, conversationId, refreshConversations }: Props = $props()

	let messages = $state<ChatMessage[]>([])
	let inputMessage = $state('')
	let isLoading = $state(false)
	let messagesContainer: HTMLDivElement | undefined = $state()

	export function fillInputMessage(message: string) {
		inputMessage = message
	}

	export function clearMessages() {
		messages = []
		inputMessage = ''
	}

	export async function loadConversationMessages(conversationId: string) {
		if (!$workspaceStore) return

		try {
			const response = await FlowConversationService.listConversationMessages({
				workspace: $workspaceStore,
				conversationId: conversationId
			})

			messages = response
		} catch (error) {
			console.error('Failed to load conversation messages:', error)
			sendUserToast('Failed to load conversation messages', true)
		}
	}

	function scrollToBottom() {
		if (messagesContainer) {
			messagesContainer.scrollTop = messagesContainer.scrollHeight
		}
	}

	async function pollJobResult(jobId: string, messageId: string) {
		try {
			// Poll for job completion
			const maxAttempts = 30 // 30 seconds max
			let attempts = 0

			while (attempts < maxAttempts) {
				await new Promise((resolve) => setTimeout(resolve, 1000)) // Wait 1 second
				attempts++

				try {
					const completedJob = await JobService.getCompletedJob({
						workspace: $workspaceStore!,
						id: jobId
					})

					if (completedJob) {
						console.log('completedJob', completedJob)
						// Job completed, update the message with the result
						messages = messages.map((msg) =>
							msg.id === messageId
								? {
										...msg,
										loading: false,
										content: formatJobResult(completedJob.result)
									}
								: msg
						)
						return
					}
				} catch (error) {
					// Job not completed yet, continue polling
					continue
				}
			}

			// Timeout - mark as failed
			messages = messages.map((msg) =>
				msg.id === messageId
					? {
							...msg,
							loading: false,
							content: 'Job timed out or failed to complete'
						}
					: msg
			)
		} catch (error) {
			console.error('Error polling job result:', error)
			messages = messages.map((msg) =>
				msg.id === messageId
					? {
							...msg,
							loading: false,
							content: 'Error retrieving job result'
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

		const userMessage: ChatMessage = {
			id: crypto.randomUUID(),
			content: inputMessage.trim(),
			created_at: new Date().toISOString(),
			message_type: 'user',
			conversation_id: conversationId ?? ''
		}

		messages = [...messages, userMessage]
		const messageContent = inputMessage.trim()
		inputMessage = ''
		isLoading = true

		try {
			// Run the flow with the user message as input
			// The backend will automatically store messages when the flow runs
			const jobId = await onRunFlow({ user_message: messageContent }, conversationId)

			// Add assistant message placeholder
			const assistantMessageId = crypto.randomUUID()
			const assistantMessage: ChatMessage = {
				id: assistantMessageId,
				content: '',
				created_at: new Date().toISOString(),
				message_type: 'assistant',
				conversation_id: conversationId ?? '',
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
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault()
			sendMessage()
		}
	}

	$effect(() => {
		scrollToBottom()
	})
</script>

<div class="flex flex-col h-full w-full">
	<div class="flex-1 flex flex-col min-h-0 w-full">
		<!-- Messages Container -->
		<div bind:this={messagesContainer} class="flex-1 overflow-y-auto p-4 space-y-4 bg-background">
			{#if messages.length === 0}
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
								? 'bg-blue-500 text-white'
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
		<div class="p-4 border-t border-gray-200 dark:border-gray-700 bg-surface">
			<div class="flex gap-2">
				<textarea
					bind:value={inputMessage}
					use:autosize
					onkeydown={handleKeyDown}
					placeholder="Type your message here..."
					class="flex-1 min-h-[40px] max-h-32 resize-none rounded-md border border-gray-200 dark:border-gray-600 bg-surface px-3 py-2 text-sm placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
					disabled={isLoading}
				></textarea>
				<Button
					size="md"
					startIcon={{ icon: isLoading ? Loader2 : Send }}
					disabled={!inputMessage?.trim() || isLoading}
					on:click={sendMessage}
					iconOnly
					title="Send message (Enter)"
				/>
			</div>
		</div>
	</div>
</div>

<style>
	/* Custom scrollbar for messages container */
	.overflow-y-auto::-webkit-scrollbar {
		width: 6px;
	}
	.overflow-y-auto::-webkit-scrollbar-track {
		background: transparent;
	}
	.overflow-y-auto::-webkit-scrollbar-thumb {
		background: rgba(156, 163, 175, 0.5);
		border-radius: 3px;
	}
	.overflow-y-auto::-webkit-scrollbar-thumb:hover {
		background: rgba(156, 163, 175, 0.7);
	}
</style>
