import type {
	FlowConversation,
	FlowConversationMessage,
	ListFlowConversationsData
} from '$lib/gen/types.gen'
import { FlowConversationsService, JobService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { waitJob } from '$lib/components/waitJob'
import { tick } from 'svelte'
import InfiniteList from '$lib/components/InfiniteList.svelte'
import { workspaceStore, userStore } from '$lib/stores'
import { get } from 'svelte/store'
import { parseStreamEvents } from '$lib/components/chat/utils'
import { randomUUID } from '$lib/utils/uuid'
import {
	prefersInstantReveal,
	TypewriterReveal
} from '$lib/components/copilot/chat/typewriterReveal'
import { appendRevealed, applyStreamEvent, emptyTurnState, type TurnState } from './turnTranscript'

export interface ChatMessage extends FlowConversationMessage {
	loading?: boolean
	streaming?: boolean
	/**
	 * The tool a row's call belongs to, as the stream reports it. Local to a running turn:
	 * afterwards the name comes from the summary the server stored, and the call itself
	 * from the tool's own job (see toolCallContext) or from the row's own
	 * `tool_arguments` / `tool_result` when the tool had no job.
	 */
	tool_name?: string
}

export interface ConversationWithDraft extends FlowConversation {
	isDraft?: boolean
}

/** A chat run from the editor's test panel, one started on the deployed flow, or both. */
export type ConversationKind = NonNullable<ListFlowConversationsData['kind']>

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
	/**
	 * A turn is on its way but has no job yet — attachments uploading, say. `isLoading`
	 * only covers the run itself, and the gap between them is long enough to change what
	 * the send lands in.
	 */
	isDispatchingTurn = $state(false)
	/** The thinking of the turn in flight, until it is attached to the answer it produced. */
	currentReasoning = $state('')
	/** The model is reasoning: true from the first thinking token until the answer starts. */
	isReasoningActive = $state(false)

	// What the turn has written so far — which row is open, and the text in it. Held here
	// rather than in the stream handler's locals: the typewriter reveals on animation
	// frames, long after the chunk that delivered the text was applied.
	#turn: TurnState = emptyTurnState('')

	/** Row ids are temp- prefixed: the sweep after a run keeps only what the server stored. */
	#newRowId = () => 'temp-' + randomUUID()

	// The worker's events reach us in bursts — the provider batches tokens, and the SSE
	// endpoint ships whatever accumulated — so display is paced separately from arrival,
	// exactly as the session chat does it. Answer and thinking pace independently.
	#replyReveal = new TypewriterReveal({
		onReveal: (chunk) => this.#reveal('answer', chunk),
		instant: prefersInstantReveal()
	})
	#reasoningReveal = new TypewriterReveal({
		onReveal: (chunk) => this.#reveal('reasoning', chunk),
		instant: prefersInstantReveal()
	})

	#reveal(kind: 'answer' | 'reasoning', chunk: string) {
		const step = appendRevealed(
			{ rows: this.messages, state: this.#turn },
			kind,
			chunk,
			this.#newRowId
		)
		this.messages = step.rows
		this.#turn = step.state
		if (kind === 'reasoning') this.currentReasoning = step.state.reasoning
	}

	/** Reveal everything buffered now, so the row is whole before the turn moves on. */
	#flushReveals() {
		this.#replyReveal.flush()
		this.#reasoningReveal.flush()
	}
	/**
	 * Which conversations the list holds. The editor shows its own test chats, since
	 * testing is what happens there; a deployed flow shows the chats its users started,
	 * so nobody's trial runs are mixed into them.
	 */
	conversationKind = $state<ConversationKind>('deployed')
	/**
	 * Whether the sidebar offers the kind filter. Only the editor does: a deployed flow has
	 * no test chats of its own to show, and offering to list someone's trial runs there
	 * would put editor scratch in front of the flow's users.
	 */
	canFilterConversationKind = $state(false)
	selectedConversationId = $state<string | undefined>(undefined)
	conversationListComponent = $state<InfiniteList | undefined>(undefined)

	// Private state
	#conversationsCache = $state<Record<string, ChatMessage[]>>({})
	#scrollTimeout: ReturnType<typeof setTimeout> | undefined = undefined
	#perPage = 50

	// Options
	#onRunFlow?: (
		userMessage: string,
		conversationId: string,
		additionalInputs?: Record<string, any>
	) => Promise<string | undefined>
	#useStreaming = $state(false)
	#path = $state<string | undefined>(undefined)

	// When the flow editor runs as an AI-session live editor, it acts on a workspace
	// that can differ from the nav store. FlowChat.svelte wires this to
	// FlowEditorContext.opWorkspace so workspace-scoped calls hit the acting workspace.
	operatingWorkspace?: () => string | undefined

	#workspace(): string | undefined {
		return this.operatingWorkspace?.() ?? get(workspaceStore)
	}

	initialize(
		onRunFlow: (
			userMessage: string,
			conversationId: string,
			additionalInputs?: Record<string, any>
		) => Promise<string | undefined>,
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
		this.#replyReveal.reset()
		this.#reasoningReveal.reset()
		this.#turn = emptyTurnState('')
		this.currentReasoning = ''
		this.isReasoningActive = false
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
			workspace_id: this.#workspace()!,
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

	/**
	 * Open this flow's most recent conversation, unless the caller already chose one.
	 *
	 * Every turn is stored the moment it runs — a preview from the editor exactly like a
	 * deployed run — so a chat that has been used before should come back to it instead of
	 * to an empty pane. The editor needs this most: it hides the conversations sidebar, so
	 * without it there is no way back to what was said.
	 */
	async selectLatestConversation() {
		if (this.selectedConversationId || !this.#workspace() || !this.#path) return
		const [latest] = await this.loadConversations(1, 1)
		// Re-checked after the await: a message sent meanwhile has already opened its own.
		if (!latest || this.selectedConversationId) return
		await this.selectConversation(latest.id)
	}

	/**
	 * Narrow the list to one kind of chat and reload it.
	 *
	 * The open conversation goes with it when it is not of the new kind: the composer sends
	 * into whatever is selected, and a test run appended to a deployed conversation would be
	 * stored as part of it — `get_or_create_conversation_with_id` keeps the row's own
	 * `is_test`, so the mixing would be invisible afterwards.
	 */
	async setConversationKind(kind: ConversationKind) {
		if (this.conversationKind === kind) return
		// A turn in flight writes into the conversation it started in — the stream appends
		// rows, the poller merges more — so the selection cannot be moved under it. Held
		// from the moment the composer starts dispatching, since an upload runs before the
		// job exists and a send landing after the switch would start a fresh conversation.
		// The control is disabled meanwhile; this is the same rule where it is enforced.
		if (this.isTurnInFlight) return
		this.conversationKind = kind
		const open = this.conversations.find((c) => c.id === this.selectedConversationId)
		const stillListed =
			open === undefined ||
			open.isDraft === true ||
			kind === 'all' ||
			(kind === 'test') === (open.is_test === true)
		if (!stillListed) {
			this.selectedConversationId = undefined
			this.clearMessages()
		}
		await this.refreshConversations()
	}

	// A name typed on a chat that has not run yet, kept until its first turn creates the row
	// server-side: that insert titles the conversation from the message, and the refresh
	// which follows would otherwise replace the typed name with it.
	#draftTitle: { id: string; title: string } | undefined = undefined

	/** Write a title to the server and to the row the list holds. */
	async #writeConversationTitle(conversationId: string, title: string) {
		try {
			await FlowConversationsService.updateFlowConversation({
				workspace: this.#workspace()!,
				conversationId,
				requestBody: { title }
			})
			this.conversations = this.conversations.map((c) =>
				c.id === conversationId ? { ...c, title } : c
			)
			return true
		} catch (error) {
			console.error('Failed to rename conversation:', error)
			sendUserToast('Failed to rename conversation', true)
			return false
		}
	}

	/** Rename a chat. The list holds the row, so it is patched rather than reloaded. */
	async renameConversation(conversationId: string, title: string) {
		const trimmed = title.trim()
		const current = this.conversations.find((c) => c.id === conversationId)
		if (!current || trimmed === '' || trimmed === current.title) return
		// A chat that has never run is local to this list; there is nothing to rename yet.
		// The name is held instead, and written once the first turn creates the row.
		if (current.isDraft) {
			this.#draftTitle = { id: conversationId, title: trimmed }
			this.conversations = this.conversations.map((c) =>
				c.id === conversationId ? { ...c, title: trimmed } : c
			)
			return
		}
		await this.#writeConversationTitle(conversationId, trimmed)
	}

	/** A turn is being dispatched or is running: nothing may move the conversation under it. */
	get isTurnInFlight(): boolean {
		return this.isLoading || this.isWaitingForResponse || this.isDispatchingTurn
	}

	async refreshConversations() {
		await this.conversationListComponent?.loadData('forceRefresh')
	}

	// Only used by InfiniteList
	private async deleteConversation(conversationId: string) {
		try {
			this.deletingConversationId = conversationId
			await FlowConversationsService.deleteFlowConversation({
				workspace: this.#workspace()!,
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
		if (!this.#workspace()) {
			return
		}

		try {
			if (this.currentJobId) {
				await JobService.cancelQueuedJob({
					workspace: this.#workspace()!,
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
		if (!this.#workspace() || !this.#path) return []

		try {
			const response = await FlowConversationsService.listFlowConversations({
				workspace: this.#workspace()!,
				flowPath: this.#path,
				kind: this.conversationKind,
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
		if (!this.#workspace() || !conversationIdToUse) return

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

			const response = await FlowConversationsService.listConversationMessages({
				workspace: this.#workspace()!,
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

	private getLastPersistedMessageSeq() {
		for (let i = this.messages.length - 1; i >= 0; i--) {
			const message = this.messages[i]
			if (!message.id.startsWith('temp-')) {
				return message.created_seq
			}
		}

		return undefined
	}

	// Polling
	private async pollJobResult(jobId: string) {
		try {
			await waitJob(jobId, this.#workspace())
		} catch (error) {
			console.error('Error polling job result:', error)
		} finally {
			// Do a final poll to get all messages from database
			try {
				if (this.selectedConversationId) {
					await this.pollConversationMessages(this.selectedConversationId, {
						removeTempMessages: true
					})
				}
			} catch {}
			this.cleanup()
		}
	}

	private async pollConversationMessages(
		conversationId: string,
		options?: { isNewConversation?: boolean; removeTempMessages?: boolean }
	) {
		if (!this.#workspace()) return

		try {
			const lastSeq = this.getLastPersistedMessageSeq()
			const response = await FlowConversationsService.listConversationMessages({
				workspace: this.#workspace()!,
				conversationId: conversationId,
				page: 1,
				perPage: 50,
				afterSeq: lastSeq
			})

			if (options?.isNewConversation) {
				await this.refreshConversations()
			}

			const filteredResponse = response.filter((msg) => msg.message_type !== 'user')
			for (const msg of filteredResponse) {
				if (!this.messages.find((m) => m.id === msg.id)) {
					this.messages = [...this.messages, msg]
				}
			}

			// Only remove temporary messages when explicitly requested (e.g., after job completion)
			// During streaming, we keep temp messages to avoid them disappearing due to race conditions
			if (options?.removeTempMessages) {
				this.messages = this.messages.filter(
					(msg) => !msg.id.startsWith('temp-') || msg.message_type === 'user'
				)
			}
		} catch (error) {
			console.error('Polling error:', error)
		}
	}

	private startPolling(conversationId: string, isNewConversation?: boolean) {
		if (this.pollingInterval) return
		this.pollingInterval = setInterval(() => {
			this.pollConversationMessages(conversationId, { isNewConversation })
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
	/**
	 * Send `inputMessage` as a turn. `onUserRow` is called with the id of the row added
	 * for it, which is the only moment that id is knowable: the caller needs it to hang
	 * what the composer sent — attachments, other inputs — on a row that has no job yet.
	 *
	 * An empty message is allowed here: a turn can carry attachments alone, and only the
	 * caller knows whether it does.
	 */
	async sendMessage(additionalInputs?: Record<string, any>, onUserRow?: (rowId: string) => void) {
		if (this.isLoading) return

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
			id: `temp-${randomUUID()}`,
			content: this.inputMessage.trim(),
			created_at: new Date().toISOString(),
			created_seq: 0,
			message_type: 'user',
			conversation_id: currentConversationId
		}

		this.messages = [...this.messages, userMessage]
		onUserRow?.(userMessage.id)
		const messageContent = this.inputMessage.trim()
		this.inputMessage = ''
		this.isLoading = true
		this.isWaitingForResponse = true

		try {
			await tick()
			this.scrollToUserMessage(userMessage.id)

			if (this.#useStreaming && this.#path) {
				await this.handleStreamingMessage(
					messageContent,
					currentConversationId,
					isNewConversation,
					additionalInputs
				)
			} else {
				await this.handlePollingMessage(
					messageContent,
					currentConversationId,
					isNewConversation,
					additionalInputs
				)
			}
		} catch (error) {
			console.error('Error running flow:', error)
			sendUserToast('Failed to run flow: ' + error, true)
		} finally {
			if (!this.#useStreaming) {
				this.isLoading = false
			}
		}

		// The row now exists, titled from the message by the server. A name typed while it
		// was a draft has to be written over that — unconditionally, not through
		// renameConversation: on the streaming path this runs before any refresh, so the
		// local row still carries the typed name and an equality check would skip the write.
		// Cleared only once it lands, so a failed run keeps the name for the next attempt.
		if (this.#draftTitle?.id === currentConversationId) {
			const { title } = this.#draftTitle
			if (await this.#writeConversationTitle(currentConversationId, title)) {
				this.#draftTitle = undefined
			}
		}

		await tick()
		this.focusInput()
	}

	private async handleStreamingMessage(
		messageContent: string,
		currentConversationId: string,
		isNewConversation: boolean,
		additionalInputs?: Record<string, any>
	) {
		// Close any existing EventSource
		if (this.currentEventSource) {
			this.currentEventSource.close()
		}

		// Track stream state for this message
		this.#turn = emptyTurnState(currentConversationId)
		this.#replyReveal.reset()
		this.#reasoningReveal.reset()
		let isCompleted = false

		try {
			const jobId = await this.#onRunFlow?.(messageContent, currentConversationId, additionalInputs)
			if (!jobId) {
				console.error('No jobId returned from onRunFlow')
				return
			}

			// Build the EventSource URL
			const streamUrl = `/api/w/${this.#workspace()}/jobs_u/getupdate_sse/${jobId}`
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
					const type = data.type

					// Handle timeout - reconnect to SSE
					if (type === 'timeout') {
						eventSource.close()
						this.currentEventSource = undefined
						// Reconnect
						this.handleStreamingMessage(
							messageContent,
							currentConversationId,
							isNewConversation,
							additionalInputs
						)
						return
					}

					// Handle ping - just ignore
					if (type === 'ping') {
						return
					}

					// Handle error
					if (type === 'error') {
						eventSource.close()
						this.currentEventSource = undefined
						console.error('SSE error:', data)
						sendUserToast('Stream error: ' + (data.error || 'Unknown error'), true)
						this.cleanup()
						return
					}

					// Handle not found
					if (type === 'not_found') {
						eventSource.close()
						this.currentEventSource = undefined
						console.error('Job not found')
						sendUserToast('Job not found', true)
						this.cleanup()
						return
					}

					if (type === 'update') {
						if (data.flow_stream_job_id) {
							this.currentJobId = data.flow_stream_job_id
						}
						// Process new stream content
						if (data.new_result_stream) {
							// Stop polling since we are receiving last step streaming
							this.stopPolling()
							// One chunk can carry several events, so each is applied in turn: a
							// chunk holding a call and its result must produce both.
							for (const event of parseStreamEvents(data.new_result_stream)) {
								if (event.kind === 'reasoning') {
									this.isReasoningActive = true
									this.#reasoningReveal.push(event.content)
								} else if (event.kind === 'token') {
									this.#replyReveal.push(event.content)
								} else {
									// Whatever the pacing still holds belongs to the row before the tool —
									// thinking that led straight to the call included — so it is revealed
									// before the event that closes that row.
									if (event.kind === 'tool_call' || event.kind === 'tool_execution') {
										this.#flushReveals()
										this.currentReasoning = ''
										this.isReasoningActive = false
									}
									const step = applyStreamEvent(
										{ rows: this.messages, state: this.#turn },
										event,
										this.#newRowId
									)
									this.messages = step.rows
									this.#turn = step.state
								}
							}
						}

						// Handle completion
						if (data.completed) {
							isCompleted = true
							// Anything still buffered would be dropped by the temp-row sweep below.
							this.#flushReveals()
							// Do a final poll to get all messages from database
							if (this.selectedConversationId) {
								await this.pollConversationMessages(this.selectedConversationId, {
									removeTempMessages: true
								})
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
		isNewConversation: boolean,
		additionalInputs?: Record<string, any>
	) {
		const jobId = await this.#onRunFlow?.(messageContent, currentConversationId, additionalInputs)
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
}

export const createFlowChatManager = () => new FlowChatManager()
