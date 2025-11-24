import type { FlowConversation, FlowConversationMessage } from '$lib/gen/types.gen'
import { FlowConversationService, JobService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { waitJob } from '$lib/components/waitJob'
import { tick } from 'svelte'
import InfiniteList from '$lib/components/InfiniteList.svelte'
import { workspaceStore, userStore } from '$lib/stores'
import { get } from 'svelte/store'
import { parseStreamDeltas } from '$lib/components/chat/utils'

export interface ChatMessage extends FlowConversationMessage {
	loading?: boolean
	streaming?: boolean
}

export interface ConversationWithDraft extends FlowConversation {
	isDraft?: boolean
}

export function randomUUID() {
	// Pure JS (RFC4122 v4) UUID implementation (no external dependencies)
	return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
		const r = (Math.random() * 16) | 0
		const v = c === 'x' ? r : (r & 0x3) | 0x8
		return v.toString(16)
	})
}

export class FlowChatManager {
	// State
	messages = $state<ChatMessage[]>([])
	inputMessage = $state('')
	isLoading = $state(false)
	isLoadingMessages = $state(false)
	isWaitingForResponse = $state(false)
	messagesContainer = $state<HTMLDivElement | undefined>(undefined)
	inputElement = $state<HTMLTextAreaElement | undefined>(undefined)
	page = $state(1)
	hasMoreMessages = $state(false)
	loadingMoreMessages = $state(false)
	currentEventSource = $state<EventSource | undefined>(undefined)
	pollingInterval = $state<ReturnType<typeof setInterval> | undefined>(undefined)
	currentJobId = $state<string | undefined>(undefined)
	conversations = $state<ConversationWithDraft[]>([])
	deletingConversationId = $state<string | undefined>(undefined)
	isSidebarExpanded = $state(false)
	selectedConversationId = $state<string | undefined>(undefined)
	conversationListComponent = $state<InfiniteList | undefined>(undefined)

	// Private state
	#conversationsCache = $state<Record<string, ChatMessage[]>>({})
	#scrollTimeout: ReturnType<typeof setTimeout> | undefined = undefined
	#perPage = 50

	// Options
	#onRunFlow?: (userMessage: string, conversationId: string) => Promise<string | undefined>
	#useStreaming = $state(false)
	#path = $state<string | undefined>(undefined)

	initialize(
		onRunFlow: (userMessage: string, conversationId: string) => Promise<string | undefined>,
		path: string,
		useStreaming: boolean = false
	) {
		this.#onRunFlow = onRunFlow
		this.#path = path
		this.#useStreaming = useStreaming
	}

	updateConversationId(conversationId: string | undefined) {
		this.selectedConversationId = conversationId
	}

	cleanup() {
		if (this.currentEventSource) {
			this.currentEventSource.close()
			this.currentEventSource = undefined
		}
		this.stopPolling()
		this.isLoading = false
		this.isWaitingForResponse = false
		this.currentJobId = undefined
	}

	// Public methods for component to call
	fillInputMessage(message: string) {
		this.inputMessage = message
	}

	focusInput() {
		console.log('focusInput', this.inputElement)
		this.inputElement?.focus()
	}

	clearMessages() {
		this.messages = []
		this.inputMessage = ''
		this.page = 1
	}

	async createConversation({ clearMessages = true }: { clearMessages?: boolean }) {
		// Check if there's already a draft conversation
		const existingDraft = this.conversations.find((c) => c.isDraft)
		if (existingDraft) {
			// Select the existing draft instead of creating a new one
			this.selectedConversationId = existingDraft.id
			this.clearMessages()
			return existingDraft.id
		}
		const newConversationId = randomUUID()
		this.selectedConversationId = newConversationId

		// Create a new conversation object and add it to the top of the list
		const newConversation: ConversationWithDraft = {
			id: newConversationId,
			workspace_id: get(workspaceStore)!,
			flow_path: this.#path!,
			title: 'New chat',
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString(),
			created_by: get(userStore)!.username!,
			isDraft: true
		}

		// Prepend to conversations list
		this.conversations = [newConversation, ...this.conversations]
		// Clear messages in the chat interface
		if (clearMessages) {
			this.clearMessages()
		}
		this.focusInput()

		return newConversationId
	}

	setupInfiniteList() {
		this.conversationListComponent?.setLoader((page, perPage) =>
			this.loadConversations(page, perPage)
		)
		this.conversationListComponent?.setDeleteItemFn((id) => this.deleteConversation(id))
	}

	async selectConversation(conversationId: string, isDraft?: boolean) {
		this.selectedConversationId = conversationId
		// Load conversation messages into chat interface
		if (isDraft) {
			// For draft conversations, just clear messages (don't try to load from backend)
			this.clearMessages()
		} else {
			// For persisted conversations, load messages from backend
			await this.loadConversationMessages(conversationId)
		}
	}

	async refreshConversations() {
		await this.conversationListComponent?.loadData('forceRefresh')
	}

	// Only used by InfiniteList
	private async deleteConversation(conversationId: string) {
		try {
			this.deletingConversationId = conversationId
			await FlowConversationService.deleteFlowConversation({
				workspace: get(workspaceStore)!,
				conversationId
			})
			if (this.selectedConversationId === conversationId) {
				this.selectedConversationId = undefined
				this.clearMessages()
			}
			sendUserToast('Conversation deleted successfully')
		} catch (error) {
			console.error('Failed to delete conversation:', error)
			sendUserToast('Failed to delete conversation', true)
			throw error
		} finally {
			this.deletingConversationId = undefined
		}
	}

	async cancelCurrentJob() {
		if (!get(workspaceStore)) {
			return
		}

		try {
			if (this.currentJobId) {
				await JobService.cancelQueuedJob({
					workspace: get(workspaceStore)!,
					id: this.currentJobId,
					requestBody: {}
				})
				sendUserToast(`Job ${this.currentJobId} cancelled`)
			}
		} catch (error) {
			console.error('Error cancelling job:', error)
			sendUserToast('Could not cancel job', true)
		} finally {
			this.cleanup()
		}
	}

	async loadConversationMessages(conversationId?: string) {
		this.page = 1
		await this.loadMessages(true, conversationId)
	}

	// Only used by InfiniteList
	private async loadConversations(page: number, perPage: number) {
		if (!get(workspaceStore) || !this.#path) return []

		try {
			const response = await FlowConversationService.listFlowConversations({
				workspace: get(workspaceStore)!,
				flowPath: this.#path,
				page: page,
				perPage: perPage
			})

			return response
		} catch (error) {
			console.error('Failed to load conversations:', error)
			sendUserToast('Failed to load conversations', true)
			return []
		}
	}

	// Message loading
	private async loadMessages(reset: boolean, conversationId?: string) {
		let conversationIdToUse = conversationId ?? this.selectedConversationId
		if (!get(workspaceStore) || !conversationIdToUse) return

		if (reset) {
			if (this.#conversationsCache[conversationIdToUse]) {
				this.messages = this.#conversationsCache[conversationIdToUse]
				return
			}
			this.isLoadingMessages = true
		} else {
			this.loadingMoreMessages = true
		}

		const pageToFetch = reset ? 1 : this.page + 1

		try {
			const previousScrollHeight = this.messagesContainer?.scrollHeight || 0

			const response = await FlowConversationService.listConversationMessages({
				workspace: get(workspaceStore)!,
				conversationId: conversationIdToUse,
				page: pageToFetch,
				perPage: this.#perPage
			})

			if (reset) {
				this.#conversationsCache[conversationIdToUse] = response
				this.messages = response
				this.isLoadingMessages = false
				await new Promise((resolve) => setTimeout(resolve, 100))
				this.scrollToBottom()
			} else {
				this.messages = [...response, ...this.messages]
				this.page = pageToFetch
				// Restore scroll position
				await new Promise((resolve) => setTimeout(resolve, 50))
				if (this.messagesContainer) {
					this.messagesContainer.scrollTop =
						this.messagesContainer.scrollHeight - previousScrollHeight
				}
			}

			this.hasMoreMessages = response.length === this.#perPage
		} catch (error) {
			console.error('Failed to load messages:', error)
			sendUserToast('Failed to load messages: ' + error)
		} finally {
			this.isLoadingMessages = false
			this.loadingMoreMessages = false
		}
	}

	handleScroll = () => {
		if (this.#scrollTimeout) clearTimeout(this.#scrollTimeout)

		this.#scrollTimeout = setTimeout(() => {
			if (!this.messagesContainer || !this.hasMoreMessages || this.loadingMoreMessages) return

			if (this.messagesContainer.scrollTop <= 10) {
				this.loadMessages(false)
			}
		}, 200)
	}

	scrollToBottom() {
		if (this.messagesContainer) {
			this.messagesContainer.scrollTop = this.messagesContainer.scrollHeight
		}
	}

	private scrollToUserMessage(messageId: string) {
		if (!this.messagesContainer) return
		const messageElement = this.messagesContainer.querySelector(`[data-message-id="${messageId}"]`)
		if (messageElement) {
			messageElement.scrollIntoView({ behavior: 'smooth', block: 'start' })
		}
	}

	// Polling
	private async pollJobResult(jobId: string) {
		try {
			await waitJob(jobId)
		} catch (error) {
			console.error('Error polling job result:', error)
		} finally {
			// Do a final poll to get all messages from database
			try {
				if (this.selectedConversationId) {
					await this.pollConversationMessages(this.selectedConversationId)
				}
			} catch {}
			this.cleanup()
		}
	}

	private async pollConversationMessages(conversationId: string, isNewConversation?: boolean) {
		if (!get(workspaceStore)) return

		try {
			const lastId = this.messages[this.messages.length - 1].id
			const response = await FlowConversationService.listConversationMessages({
				workspace: get(workspaceStore)!,
				conversationId: conversationId,
				page: 1,
				perPage: 50,
				afterId: lastId
			})

			if (isNewConversation) {
				await this.refreshConversations()
			}

			const filteredResponse = response.filter((msg) => msg.message_type !== 'user')

			// Add any new intermediate messages not already present
			for (const msg of filteredResponse) {
				if (!this.messages.find((m) => m.id === msg.id)) {
					this.messages = [...this.messages, msg]
				}
			}

			// Remove temporary messages
			this.messages = this.messages.filter((msg) => !msg.id.startsWith('temp-'))
		} catch (error) {
			console.error('Polling error:', error)
		}
	}

	private startPolling(conversationId: string, isNewConversation?: boolean) {
		if (this.pollingInterval) return
		this.pollingInterval = setInterval(() => {
			this.pollConversationMessages(conversationId, isNewConversation)
		}, 500) // Poll every 0.5 seconds
		setTimeout(
			() => {
				this.stopPolling()
			},
			2 * 60 * 1000
		) // Stop polling after 2 minutes
	}

	private stopPolling() {
		if (this.pollingInterval) {
			clearInterval(this.pollingInterval)
			this.pollingInterval = undefined
		}
	}

	// Message sending
	async sendMessage() {
		if (!this.inputMessage.trim() || this.isLoading) return

		const isNewConversation = this.messages.length === 0

		// Reset state for new message
		this.stopPolling()

		// Generate a new conversation ID if we don't have one
		let currentConversationId = this.selectedConversationId
		if (!this.selectedConversationId) {
			const newConversationId = await this.createConversation({ clearMessages: false })
			currentConversationId = newConversationId
		}

		if (!currentConversationId) {
			console.error('No conversation ID found')
			return
		}

		// Invalidate the conversation cache
		delete this.#conversationsCache[currentConversationId]

		const userMessage: ChatMessage = {
			id: randomUUID(),
			content: this.inputMessage.trim(),
			created_at: new Date().toISOString(),
			message_type: 'user',
			conversation_id: currentConversationId
		}

		this.messages = [...this.messages, userMessage]
		const messageContent = this.inputMessage.trim()
		this.inputMessage = ''
		this.isLoading = true
		this.isWaitingForResponse = true

		try {
			await tick()
			this.scrollToUserMessage(userMessage.id)

			if (this.#useStreaming && this.#path) {
				await this.handleStreamingMessage(messageContent, currentConversationId, isNewConversation)
			} else {
				await this.handlePollingMessage(messageContent, currentConversationId, isNewConversation)
			}
		} catch (error) {
			console.error('Error running flow:', error)
			sendUserToast('Failed to run flow: ' + error, true)
		} finally {
			if (!this.#useStreaming) {
				this.isLoading = false
			}
		}

		await tick()
		this.focusInput()
	}

	private async handleStreamingMessage(
		messageContent: string,
		currentConversationId: string,
		isNewConversation: boolean
	) {
		// Close any existing EventSource
		if (this.currentEventSource) {
			this.currentEventSource.close()
		}

		// Track stream state for this message
		let accumulatedContent = ''
		let assistantMessageId = ''
		let isCompleted = false

		try {
			const jobId = await this.#onRunFlow?.(messageContent, currentConversationId)
			if (!jobId) {
				console.error('No jobId returned from onRunFlow')
				return
			}

			// Build the EventSource URL
			const streamUrl = `/api/w/${get(workspaceStore)}/jobs_u/getupdate_sse/${jobId}`
			const url = new URL(streamUrl, window.location.origin)
			url.searchParams.set('poll_delay_ms', '50')
			url.searchParams.set('fast', 'true')
			url.searchParams.set('only_result', 'true')
			// Create EventSource connection
			const eventSource = new EventSource(url.toString())
			this.currentEventSource = eventSource

			// start polling
			this.startPolling(currentConversationId, isNewConversation)

			eventSource.onmessage = async (event) => {
				try {
					const data = JSON.parse(event.data)
					if (data.type === 'update') {
						if (data.flow_stream_job_id) {
							this.currentJobId = data.flow_stream_job_id
						}
						// Process new stream content
						if (data.new_result_stream) {
							// Stop polling since we are receiving last step streaming
							this.stopPolling()
							const {
								type,
								content: newContent,
								success
							} = parseStreamDeltas(data.new_result_stream)
							accumulatedContent += newContent

							// Create tool message if type is tool_result
							if (type === 'tool_result') {
								// set last message streaming to false
								this.messages = this.messages.map((msg) =>
									msg.id === this.messages[this.messages.length - 1].id
										? { ...msg, streaming: false }
										: msg
								)

								this.messages = [
									...this.messages,
									{
										id: 'temp-' + randomUUID(),
										content: newContent,
										created_at: new Date().toISOString(),
										message_type: 'tool',
										conversation_id: currentConversationId,
										job_id: '',
										loading: false,
										streaming: false,
										success
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
								assistantMessageId = 'temp-' + randomUUID()
								this.messages = [
									...this.messages,
									{
										id: assistantMessageId,
										content: accumulatedContent,
										created_at: new Date().toISOString(),
										message_type: 'assistant',
										conversation_id: currentConversationId,
										job_id: '',
										loading: false,
										streaming: true
									}
								]
							} else {
								// Update existing message
								this.messages = this.messages.map((msg) =>
									msg.id === assistantMessageId ? { ...msg, content: accumulatedContent } : msg
								)
							}
						}

						// Handle completion
						if (data.completed) {
							isCompleted = true
							// Do a final poll to get all messages from database
							if (this.selectedConversationId) {
								await this.pollConversationMessages(this.selectedConversationId)
							}
							this.cleanup()
						}
					}
				} catch (error) {
					console.error('Error processing stream event:', error)
				}
			}

			eventSource.onerror = (error) => {
				if (isCompleted) return
				console.error('EventSource error:', error)
				sendUserToast('Stream error occurred', true)
				this.cleanup()
			}
		} catch (error) {
			console.error('Stream connection error:', error)
			sendUserToast('Failed to connect to stream', true)
			this.cleanup()
		}
	}

	private async handlePollingMessage(
		messageContent: string,
		currentConversationId: string,
		isNewConversation: boolean
	) {
		const jobId = await this.#onRunFlow?.(messageContent, currentConversationId)
		if (!jobId) {
			console.error('No jobId returned from onRunFlow')
			return
		}

		// Store the current job ID so it can be cancelled
		this.currentJobId = jobId

		if (isNewConversation) {
			await this.refreshConversations()
		}

		// Start polling for intermediate messages in non-streaming mode too
		this.startPolling(currentConversationId)
		this.pollJobResult(jobId)
	}

	handleKeyDown = (event: KeyboardEvent) => {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault()
			this.sendMessage()
		}
	}
}

export const createFlowChatManager = () => new FlowChatManager()
