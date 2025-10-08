import type { FlowConversationMessage } from '$lib/gen/types.gen'
import { FlowConversationService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { waitJob } from '$lib/components/waitJob'
import { tick } from 'svelte'

export interface ChatMessage extends FlowConversationMessage {
	loading?: boolean
	streaming?: boolean
}

export interface FlowChatManagerOptions {
	onRunFlow: (userMessage: string, conversationId: string) => Promise<string | undefined>
	createConversation: (options: { clearMessages?: boolean }) => Promise<string>
	refreshConversations?: () => Promise<void>
	conversationId?: string
	useStreaming?: boolean
	path?: string
}

class FlowChatManager {
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

	// Private state
	#conversationsCache = $state<Record<string, ChatMessage[]>>({})
	#scrollTimeout: ReturnType<typeof setTimeout> | undefined = undefined
	#perPage = 50
	#workspace = $state<string | undefined>(undefined)

	// Options
	#onRunFlow?: FlowChatManagerOptions['onRunFlow']
	#createConversation?: FlowChatManagerOptions['createConversation']
	#refreshConversations?: FlowChatManagerOptions['refreshConversations']
	#conversationId = $state<string | undefined>(undefined)
	#useStreaming = $state(false)
	#path = $state<string | undefined>(undefined)

	initialize(options: FlowChatManagerOptions, workspace: string) {
		this.#onRunFlow = options.onRunFlow
		this.#createConversation = options.createConversation
		this.#refreshConversations = options.refreshConversations
		this.#conversationId = options.conversationId
		this.#useStreaming = options.useStreaming ?? false
		this.#path = options.path
		this.#workspace = workspace
	}

	updateConversationId(conversationId: string | undefined) {
		console.log('updateConversationId', conversationId)
		this.#conversationId = conversationId
	}

	cleanup() {
		if (this.currentEventSource) {
			this.currentEventSource.close()
			this.currentEventSource = undefined
		}
		this.stopPolling()
		this.isLoading = false
		this.isWaitingForResponse = false
	}

	// Public methods for component to call
	fillInputMessage(message: string) {
		this.inputMessage = message
	}

	focusInput() {
		this.inputElement?.focus()
	}

	clearMessages() {
		this.messages = []
		this.inputMessage = ''
		this.page = 1
	}

	async loadConversationMessages(conversationId?: string) {
		this.page = 1
		await this.loadMessages(true, conversationId)
	}

	// Message loading
	private async loadMessages(reset: boolean, conversationId?: string) {
		let conversationIdToUse = conversationId ?? this.#conversationId
		if (!this.#workspace || !conversationIdToUse) return

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
			console.log('loadMessages', this.#conversationId, pageToFetch)
			const previousScrollHeight = this.messagesContainer?.scrollHeight || 0

			const response = await FlowConversationService.listConversationMessages({
				workspace: this.#workspace,
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
				if (this.#conversationId) {
					await this.pollConversationMessages(this.#conversationId)
				}
			} catch {}
			this.cleanup()
		}
	}

	private parseStreamDeltas(streamData: string): {
		type: string
		content: string
		success: boolean
	} {
		let type = 'message'
		const lines = streamData.trim().split('\n')
		let content = ''
		let success = true
		for (const line of lines) {
			if (!line.trim()) continue
			try {
				const parsed = JSON.parse(line)
				if (parsed.type === 'tool_result') {
					type = 'tool_result'
					const toolName = parsed.function_name
					success = parsed.success
					content = success ? `Used ${toolName} tool` : `Failed to use ${toolName} tool`
				}
				if (parsed.type === 'token_delta' && parsed.content) {
					type = 'message'
					content += parsed.content
				}
			} catch (e) {
				console.error('Failed to parse stream line:', line, e)
			}
		}
		return { type, content, success }
	}

	private async pollConversationMessages(conversationId: string, isNewConversation?: boolean) {
		if (!this.#workspace) return

		try {
			const lastId = this.messages[this.messages.length - 1].id
			const response = await FlowConversationService.listConversationMessages({
				workspace: this.#workspace,
				conversationId: conversationId,
				page: 1,
				perPage: 50,
				afterId: lastId
			})

			if (isNewConversation) {
				await this.#refreshConversations?.()
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
	}

	private stopPolling() {
		console.log('stopPolling')
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
		let currentConversationId = this.#conversationId
		if (!this.#conversationId && this.#createConversation) {
			const newConversationId = await this.#createConversation({ clearMessages: false })
			currentConversationId = newConversationId
		}

		if (!currentConversationId) {
			console.error('No conversation ID found')
			return
		}

		// Invalidate the conversation cache
		delete this.#conversationsCache[currentConversationId]

		const userMessage: ChatMessage = {
			id: crypto.randomUUID(),
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

		try {
			// Encode the payload as base64
			const payload = { user_message: messageContent }
			const payloadBase64 = btoa(JSON.stringify(payload))

			// Build the EventSource URL
			const streamUrl = `/api/w/${this.#workspace}/jobs/run_and_stream/f/${this.#path}`
			const url = new URL(streamUrl, window.location.origin)
			url.searchParams.set('payload', payloadBase64)
			url.searchParams.set('memory_id', currentConversationId)
			url.searchParams.set('poll_delay_ms', '50')

			// Create EventSource connection
			const eventSource = new EventSource(url.toString())
			this.currentEventSource = eventSource

			// start polling
			this.startPolling(currentConversationId, isNewConversation)

			eventSource.onmessage = async (event) => {
				try {
					const data = JSON.parse(event.data)

					if (data.type === 'update') {
						// Process new stream content
						if (data.new_result_stream) {
							// Stop polling since we are receiving last step streaming
							this.stopPolling()
							const {
								type,
								content: newContent,
								success
							} = this.parseStreamDeltas(data.new_result_stream)
							accumulatedContent += newContent
							if (accumulatedContent.length > 0 || type === 'tool_result') {
								this.isWaitingForResponse = false
							}

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
										id: 'temp-' + crypto.randomUUID(),
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
								assistantMessageId = 'temp-' + crypto.randomUUID()
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
							// Do a final poll to get all messages from database
							if (this.#conversationId) {
								await this.pollConversationMessages(this.#conversationId)
							}
							this.cleanup()
						}
					}
				} catch (error) {
					console.error('Error processing stream event:', error)
				}
			}

			eventSource.onerror = (error) => {
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

		if (isNewConversation) {
			await this.#refreshConversations?.()
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
