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
import {
	appendRevealed,
	applyStreamEvent,
	emptyTurnState,
	turnFailed,
	type TurnState
} from './turnTranscript'

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

/** What the sidebar shows for one chat. */
export type ConversationStatus = 'running' | 'error' | 'queued' | 'idle'

/** What the chat renders about one conversation's turn. */
type TurnStatus = {
	isLoading: boolean
	isWaitingForResponse: boolean
	/**
	 * A turn is on its way but has no job yet — attachments uploading, say. `isLoading`
	 * only covers the run itself, and the gap between them is long enough to change what
	 * the send lands in.
	 */
	isDispatchingTurn: boolean
	/** The thinking of the turn in flight, until it is attached to the answer it produced. */
	currentReasoning: string
	/** The model is reasoning: true from the first thinking token until the answer starts. */
	isReasoningActive: boolean
	jobId?: string
}

/**
 * The machinery one conversation's live turn runs on. Deliberately not `$state`: nothing
 * renders from it, and an EventSource or a TypewriterReveal has no business behind a proxy.
 */
type TurnRuntime = {
	eventSource?: EventSource
	pollingInterval?: ReturnType<typeof setInterval>
	// What the turn has written so far — which row is open, and the text in it. Held here
	// rather than in the stream handler's locals: the typewriter reveals on animation
	// frames, long after the chunk that delivered the text was applied.
	turn: TurnState
	// The worker's events reach us in bursts — the provider batches tokens, and the SSE
	// endpoint ships whatever accumulated — so display is paced separately from arrival,
	// exactly as the session chat does it. Answer and thinking pace independently.
	replyReveal: TypewriterReveal
	reasoningReveal: TypewriterReveal
	// How far into the stream this turn has read. Sent back on reconnect so a resumed
	// stream continues after the deltas already rendered rather than replaying them. The
	// offset indexes `streamJobId`'s stream alone.
	streamOffset?: number
	streamJobId?: string
}

function emptyStatus(): TurnStatus {
	return {
		isLoading: false,
		isWaitingForResponse: false,
		isDispatchingTurn: false,
		currentReasoning: '',
		isReasoningActive: false
	}
}

export class FlowChatManager {
	// State
	inputMessage = $state('')
	isLoadingMessages = $state(false)
	messagesContainer = $state<HTMLDivElement | undefined>(undefined)
	inputElement = $state<HTMLTextAreaElement | undefined>(undefined)
	loadingMoreMessages = $state(false)
	conversations = $state<ConversationWithDraft[]>([])
	deletingConversationId = $state<string | undefined>(undefined)
	isSidebarExpanded = $state(false)
	/**
	 * Whether a turn may start while another conversation's is still running.
	 *
	 * Only the deployed chat, whose run helper posts a job and holds nothing. The editor's
	 * test panel drives the flow graph, the progress bar and the step history from one run
	 * at a time, so a second turn there would move the graph off the one being watched.
	 */
	allowsParallelTurns = $state(false)

	/** Each conversation's rows, live ones included, so a turn keeps writing while the
	 * reader is in another chat. Doubles as the load cache: rows here are never re-fetched. */
	#rowsById = $state<Record<string, ChatMessage[]>>({})
	/** How far back each conversation has been paged. Held per conversation for the same
	 * reason the rows are: a cached chat keeps its scrollback when the reader returns to it,
	 * and one global counter would ask the server for the page number of whichever chat was
	 * scrolled last. */
	#pagedTo = $state<Record<string, number>>({})
	#hasMoreById = $state<Record<string, boolean>>({})
	#status = $state<Record<string, TurnStatus>>({})
	#runtime = new Map<string, TurnRuntime>()

	/** Row ids are temp- prefixed: the sweep after a run keeps only what the server stored. */
	#newRowId = () => 'temp-' + randomUUID()

	/** Every row id the chat still holds, across conversations. What a per-row store prunes
	 * against, so a background chat's rows are not mistaken for gone. */
	get liveRowIds(): Set<string> {
		const ids = new Set<string>()
		for (const rows of Object.values(this.#rowsById)) for (const row of rows) ids.add(row.id)
		return ids
	}

	/** Whether the open chat has older rows to fetch — what the scroll handler acts on. */
	get hasMoreMessages(): boolean {
		return this.selectedConversationId
			? (this.#hasMoreById[this.selectedConversationId] ?? false)
			: false
	}

	#rowsOf(conversationId: string): ChatMessage[] {
		return this.#rowsById[conversationId] ?? []
	}

	#statusOf(conversationId: string): TurnStatus {
		return this.#status[conversationId] ?? emptyStatus()
	}

	/** The status record to write into, created on first use. */
	#liveStatus(conversationId: string): TurnStatus {
		this.#status[conversationId] ??= emptyStatus()
		return this.#status[conversationId]
	}

	#liveRuntime(conversationId: string): TurnRuntime {
		const existing = this.#runtime.get(conversationId)
		if (existing) return existing
		const created: TurnRuntime = {
			turn: emptyTurnState(conversationId),
			replyReveal: new TypewriterReveal({
				onReveal: (chunk) => this.#reveal(conversationId, 'answer', chunk),
				instant: prefersInstantReveal()
			}),
			reasoningReveal: new TypewriterReveal({
				onReveal: (chunk) => this.#reveal(conversationId, 'reasoning', chunk),
				instant: prefersInstantReveal()
			})
		}
		this.#runtime.set(conversationId, created)
		return created
	}

	#reveal(conversationId: string, kind: 'answer' | 'reasoning', chunk: string) {
		const runtime = this.#liveRuntime(conversationId)
		const step = appendRevealed(
			{ rows: this.#rowsOf(conversationId), state: runtime.turn },
			kind,
			chunk,
			this.#newRowId
		)
		this.#rowsById[conversationId] = step.rows
		runtime.turn = step.state
		if (kind === 'reasoning')
			this.#liveStatus(conversationId).currentReasoning = step.state.reasoning
	}

	/**
	 * Name the run a turn started, on both the status and the message that began it.
	 *
	 * The server writes its own user row carrying this job id, but the poller drops user rows
	 * and the temp sweep keeps them, so the row on screen never becomes that one — it holds
	 * the id only after a reload fetches the transcript fresh. Stamping it here is what makes
	 * a retry replay the turn it is looking at rather than run the text again with whatever
	 * the composer holds later.
	 */
	#nameTurnJob(conversationId: string, userRowId: string, jobId: string) {
		this.#liveStatus(conversationId).jobId = jobId
		this.#rowsById[conversationId] = this.#rowsOf(conversationId).map((row) =>
			row.id === userRowId ? { ...row, job_id: jobId } : row
		)
	}

	/** Reveal everything buffered now, so the row is whole before the turn moves on. */
	#flushReveals(conversationId: string) {
		const runtime = this.#runtime.get(conversationId)
		runtime?.replyReveal.flush()
		runtime?.reasoningReveal.flush()
	}

	// The open conversation's turn, which is what the composer and the transcript render.
	get messages(): ChatMessage[] {
		return this.selectedConversationId ? this.#rowsOf(this.selectedConversationId) : []
	}
	set messages(rows: ChatMessage[]) {
		if (this.selectedConversationId) this.#rowsById[this.selectedConversationId] = rows
	}
	get isLoading(): boolean {
		return this.selectedConversationId
			? this.#statusOf(this.selectedConversationId).isLoading
			: false
	}
	get isWaitingForResponse(): boolean {
		return this.selectedConversationId
			? this.#statusOf(this.selectedConversationId).isWaitingForResponse
			: false
	}
	get isDispatchingTurn(): boolean {
		return this.selectedConversationId
			? this.#statusOf(this.selectedConversationId).isDispatchingTurn
			: false
	}
	get currentReasoning(): string {
		return this.selectedConversationId
			? this.#statusOf(this.selectedConversationId).currentReasoning
			: ''
	}
	get isReasoningActive(): boolean {
		return this.selectedConversationId
			? this.#statusOf(this.selectedConversationId).isReasoningActive
			: false
	}
	get currentJobId(): string | undefined {
		return this.selectedConversationId
			? this.#statusOf(this.selectedConversationId).jobId
			: undefined
	}

	/** Mark a turn as dispatching before it has a job — the upload the composer awaits. */
	setDispatching(conversationId: string, dispatching: boolean) {
		this.#liveStatus(conversationId).isDispatchingTurn = dispatching
	}

	/** Whether this conversation has a turn being dispatched or running. */
	isConversationBusy(conversationId: string): boolean {
		const status = this.#statusOf(conversationId)
		return status.isLoading || status.isWaitingForResponse || status.isDispatchingTurn
	}

	/** The conversations with a turn in flight right now. */
	get runningConversationIds(): string[] {
		return Object.keys(this.#status).filter((id) => this.isConversationBusy(id))
	}

	/**
	 * Whether a message is waiting to go out in this chat. The composer owns the queue —
	 * it is the thing that took the draft — and the sidebar shows it per row.
	 */
	hasQueuedMessage?: (conversationId: string) => boolean

	/**
	 * How many rows each chat held when the reader last had it open. Compared against what
	 * it holds now for the unread count, exactly as the session sidebar does it. In memory
	 * rather than on the row: what is unread is what arrived while this page was open.
	 */
	#lastSeenCount = $state<Record<string, number>>({})

	/**
	 * Rows that landed in this chat since the reader last looked at it. The open chat is
	 * being read, so it is never unread — which is also what makes the watermark cheap:
	 * it is written once, on the way out of a chat, rather than on every row that arrives.
	 */
	unreadCount(conversationId: string): number {
		if (conversationId === this.selectedConversationId) return 0
		const seen = this.#lastSeenCount[conversationId] ?? 0
		return Math.max(0, this.#rowsOf(conversationId).length - seen)
	}

	/**
	 * Unread across every chat, for the collapsed rail. Summed over the conversations whose
	 * rows this page holds — an unopened chat has none, and counting it would mean fetching
	 * every row of every chat to put a number on a button.
	 */
	get totalUnread(): number {
		return Object.keys(this.#rowsById).reduce((total, id) => total + this.unreadCount(id), 0)
	}

	/**
	 * Forget a conversation entirely: its turn, its rows and what was read of them.
	 *
	 * Leaving a chat marks it read, but a chat that leaves the *list* has nothing left to
	 * read — and its rows would otherwise keep counting toward the rail's unread badge with
	 * no row in the sidebar that could ever clear them.
	 */
	#forget(conversationId: string) {
		this.endTurn(conversationId)
		this.#runtime.delete(conversationId)
		delete this.#rowsById[conversationId]
		delete this.#status[conversationId]
		delete this.#lastSeenCount[conversationId]
		delete this.#pagedTo[conversationId]
		delete this.#hasMoreById[conversationId]
	}

	#markSeen(conversationId: string | undefined) {
		if (!conversationId) return
		this.#lastSeenCount[conversationId] = this.#rowsOf(conversationId).length
	}

	/**
	 * What the sidebar shows for one chat. Only says `error` for a conversation whose rows
	 * are loaded: an unopened chat has none, and fetching every row of every chat to put a
	 * triangle on one is not worth the requests.
	 */
	conversationStatus(conversationId: string): ConversationStatus {
		if (this.isConversationBusy(conversationId)) return 'running'
		if (this.hasQueuedMessage?.(conversationId)) return 'queued'
		const rows = this.#rowsById[conversationId]
		if (rows) {
			for (let i = rows.length - 1; i >= 0; i--) {
				if (rows[i].message_type !== 'user') continue
				return turnFailed(rows, i) ? 'error' : 'idle'
			}
		}
		return 'idle'
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
	/**
	 * What this surface's own runs are, which the filter does not change: the editor runs
	 * previews, the flow page runs the deployed flow. A conversation is fixed to one kind
	 * at creation, so a turn sent from here into a conversation of the other kind would be
	 * stored as part of it and the mixing would be invisible afterwards.
	 */
	surfaceKind = $state<Exclude<ConversationKind, 'all'>>('deployed')
	selectedConversationId = $state<string | undefined>(undefined)
	conversationListComponent = $state<InfiniteList | undefined>(undefined)

	// Private state
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

	/**
	 * Called when a turn finishes on its own, so whatever was typed into that chat while it
	 * ran can go out. This is the only thing that sends a queued message, so it fires from
	 * a settled run and nothing else: a stream that merely dropped leaves the flow job
	 * running on a worker, and the next turn must not start alongside it. The chat closing
	 * and a cancel are both silent here — a cancel instead hands the queue back to the
	 * composer, since Stop should not arm the next turn.
	 */
	onTurnSettled?: (conversationId: string) => void

	/** Stop following one conversation's turn and forget what it was mid-way through. */
	endTurn(conversationId: string, options?: { settled?: boolean }) {
		const runtime = this.#runtime.get(conversationId)
		if (runtime) {
			runtime.replyReveal.reset()
			runtime.reasoningReveal.reset()
			runtime.turn = emptyTurnState(conversationId)
			runtime.eventSource?.close()
			runtime.eventSource = undefined
			if (runtime.pollingInterval) {
				clearInterval(runtime.pollingInterval)
				runtime.pollingInterval = undefined
			}
		}
		const status = this.#liveStatus(conversationId)
		status.currentReasoning = ''
		status.isReasoningActive = false
		status.isLoading = false
		status.isWaitingForResponse = false
		status.jobId = undefined
		if (options?.settled) this.onTurnSettled?.(conversationId)
	}

	/** Every turn this chat is following. Called when the chat itself goes away. */
	cleanup() {
		for (const conversationId of Object.keys(this.#status)) this.endTurn(conversationId)
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
	}

	async createConversation({ clearMessages = true }: { clearMessages?: boolean }) {
		// Leaving a chat is leaving it read, however the selection moves — this path sets it
		// directly rather than going through `selectConversation`, and without this the rows
		// the reader just watched arrive would be counted as unread behind them.
		this.#markSeen(this.selectedConversationId)
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
			// The kind the first turn will give it: a draft started here runs on this surface.
			is_test: this.surfaceKind === 'test',
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
		// Everything the chat being left holds has been in front of the reader.
		this.#markSeen(this.selectedConversationId)
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
	 * to an empty pane, which reads as having lost it.
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
			// Still a real chat, just not one this filter lists: what was read of it stays read.
			this.#markSeen(this.selectedConversationId)
			this.selectedConversationId = undefined
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

	/** No job came of the send, so nothing is in flight and nothing is waiting on one. */
	#turnFailedToStart(conversationId: string) {
		const status = this.#liveStatus(conversationId)
		status.isLoading = false
		status.isWaitingForResponse = false
		status.isDispatchingTurn = false
	}

	/**
	 * Why the composer must stay shut, when the open conversation belongs to the other
	 * surface. Reading such a chat is fine; adding to it from here is not.
	 */
	get wrongKindReason(): string | undefined {
		const open = this.conversations.find((c) => c.id === this.selectedConversationId)
		if (!open || open.isDraft || (open.is_test === true) === (this.surfaceKind === 'test'))
			return undefined
		return this.surfaceKind === 'test'
			? 'This chat belongs to the deployed flow. Start a new chat to test.'
			: 'This chat was run from the editor. Start a new chat to continue here.'
	}

	/**
	 * The open conversation has a turn being dispatched or running: nothing may move the
	 * conversation under it. Other chats are unaffected where turns run in parallel.
	 */
	get isTurnInFlight(): boolean {
		return this.selectedConversationId
			? this.isConversationBusy(this.selectedConversationId)
			: false
	}

	/**
	 * Why this conversation cannot be opened right now. Only where a turn holds the whole
	 * surface: the editor's panel shows one run on the graph, so leaving the chat that
	 * started it would leave the graph describing a conversation nobody is reading.
	 */
	lockedReason(conversationId: string): string | undefined {
		if (this.allowsParallelTurns) return undefined
		const running = this.runningConversationIds.find((id) => id !== conversationId)
		return running ? 'Wait for the current answer to switch conversation' : undefined
	}

	/** Why a new chat cannot be started right now. Same rule, with no conversation yet. */
	get newChatReason(): string | undefined {
		if (this.allowsParallelTurns) return undefined
		return this.runningConversationIds.length > 0
			? 'Wait for the current answer to start a new chat'
			: undefined
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
			}
			this.#forget(conversationId)
			sendUserToast('Conversation deleted successfully')
		} catch (error) {
			console.error('Failed to delete conversation:', error)
			sendUserToast('Failed to delete conversation', true)
			throw error
		} finally {
			this.deletingConversationId = undefined
		}
	}

	/** Stop the open conversation's turn. Every other chat's keeps running. */
	async cancelCurrentJob() {
		const conversationId = this.selectedConversationId
		if (!this.#workspace() || !conversationId) {
			return
		}
		const jobId = this.#statusOf(conversationId).jobId

		try {
			if (jobId) {
				await JobService.cancelQueuedJob({
					workspace: this.#workspace()!,
					id: jobId,
					requestBody: {}
				})
				sendUserToast(`Job ${jobId} cancelled`)
			}
		} catch (error) {
			console.error('Error cancelling job:', error)
			sendUserToast('Could not cancel job', true)
		} finally {
			this.endTurn(conversationId)
		}
	}

	async loadConversationMessages(conversationId?: string) {
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
			// Rows already held are either what a previous load fetched or what a turn is
			// writing right now; either way re-fetching would drop a live turn's temp rows.
			if (this.#rowsById[conversationIdToUse]) {
				return
			}
			this.isLoadingMessages = true
		} else {
			this.loadingMoreMessages = true
		}

		const pageToFetch = reset ? 1 : (this.#pagedTo[conversationIdToUse] ?? 1) + 1

		try {
			const previousScrollHeight = this.messagesContainer?.scrollHeight || 0

			const response = await FlowConversationsService.listConversationMessages({
				workspace: this.#workspace()!,
				conversationId: conversationIdToUse,
				page: pageToFetch,
				perPage: this.#perPage
			})

			if (reset) {
				this.#rowsById[conversationIdToUse] = response
				this.#pagedTo[conversationIdToUse] = 1
				this.isLoadingMessages = false
				await new Promise((resolve) => setTimeout(resolve, 100))
				this.scrollToBottom()
			} else {
				// Held rows win: a page refetched after a switch back would otherwise be
				// prepended a second time.
				const held = this.#rowsOf(conversationIdToUse)
				const heldIds = new Set(held.map((m) => m.id))
				this.#rowsById[conversationIdToUse] = [
					...response.filter((m) => !heldIds.has(m.id)),
					...held
				]
				this.#pagedTo[conversationIdToUse] = pageToFetch
				// Restore scroll position
				await new Promise((resolve) => setTimeout(resolve, 50))
				if (this.messagesContainer) {
					this.messagesContainer.scrollTop =
						this.messagesContainer.scrollHeight - previousScrollHeight
				}
			}

			this.#hasMoreById[conversationIdToUse] = response.length === this.#perPage
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

	private getLastPersistedMessageSeq(conversationId: string) {
		const rows = this.#rowsOf(conversationId)
		for (let i = rows.length - 1; i >= 0; i--) {
			const message = rows[i]
			if (!message.id.startsWith('temp-')) {
				return message.created_seq
			}
		}

		return undefined
	}

	// Polling
	private async pollJobResult(conversationId: string, jobId: string) {
		try {
			await waitJob(jobId, this.#workspace())
		} catch (error) {
			console.error('Error polling job result:', error)
		} finally {
			// Do a final poll to get all messages from database
			try {
				await this.pollConversationMessages(conversationId, { removeTempMessages: true })
			} catch {}
			this.endTurn(conversationId, { settled: true })
		}
	}

	private async pollConversationMessages(
		conversationId: string,
		options?: { isNewConversation?: boolean; removeTempMessages?: boolean }
	) {
		if (!this.#workspace()) return

		try {
			const lastSeq = this.getLastPersistedMessageSeq(conversationId)
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

			// Written to this conversation's rows, not the open one's: a turn keeps landing
			// rows while the reader is in another chat.
			const filteredResponse = response.filter((msg) => msg.message_type !== 'user')
			for (const msg of filteredResponse) {
				const rows = this.#rowsOf(conversationId)
				if (!rows.find((m) => m.id === msg.id)) {
					this.#rowsById[conversationId] = [...rows, msg]
				}
			}

			// Only remove temporary messages when explicitly requested (e.g., after job completion)
			// During streaming, we keep temp messages to avoid them disappearing due to race conditions
			if (options?.removeTempMessages) {
				this.#rowsById[conversationId] = this.#rowsOf(conversationId).filter(
					(msg) => !msg.id.startsWith('temp-') || msg.message_type === 'user'
				)
			}
		} catch (error) {
			console.error('Polling error:', error)
		}
	}

	private startPolling(conversationId: string, isNewConversation?: boolean) {
		const runtime = this.#liveRuntime(conversationId)
		if (runtime.pollingInterval) return
		runtime.pollingInterval = setInterval(() => {
			this.pollConversationMessages(conversationId, { isNewConversation })
		}, 500) // Poll every 0.5 seconds
		setTimeout(
			() => {
				this.stopPolling(conversationId)
			},
			2 * 60 * 1000
		) // Stop polling after 2 minutes
	}

	private stopPolling(conversationId: string) {
		const runtime = this.#runtime.get(conversationId)
		if (runtime?.pollingInterval) {
			clearInterval(runtime.pollingInterval)
			runtime.pollingInterval = undefined
		}
	}

	// Message sending
	/**
	 * Send `inputMessage` as a turn. `onUserRow` is called with the id of the row added
	 * for it, which is the only moment that id is knowable: the caller needs it to hang
	 * what the composer sent — attachments, other inputs — on a row that has no job yet.
	 *
	 * Returns whether a run actually started, so a caller that spent the composer's draft
	 * on it can put the draft back.
	 */
	async sendMessage(
		additionalInputs?: Record<string, any>,
		onUserRow?: (rowId: string) => void,
		/**
		 * The conversation the turn was started in. Passed by a caller that had to await
		 * something first — an attachment upload — since the reader can select another
		 * conversation while it runs, and the turn belongs to the one they sent it from.
		 */
		pinnedConversationId?: string
	): Promise<boolean> {
		// Generate a new conversation ID if we don't have one
		let currentConversationId = pinnedConversationId ?? this.selectedConversationId
		if (!currentConversationId) {
			const newConversationId = await this.createConversation({ clearMessages: false })
			currentConversationId = newConversationId
		}

		if (!currentConversationId) {
			console.error('No conversation ID found')
			return false
		}

		// One turn per conversation: a second one would run against the same agent memory,
		// and the two would interleave into one transcript. A message typed meanwhile is
		// queued by the composer and flushed when this one settles.
		if (this.isConversationBusy(currentConversationId)) return false
		// Where a turn holds the whole surface, another chat's run is equally a reason not
		// to start: the editor's graph can only follow one.
		if (!this.allowsParallelTurns && this.runningConversationIds.length > 0) return false

		const isNewConversation = this.#rowsOf(currentConversationId).length === 0

		// Reset state for new message
		this.stopPolling(currentConversationId)

		const userMessage: ChatMessage = {
			id: `temp-${randomUUID()}`,
			content: this.inputMessage.trim(),
			created_at: new Date().toISOString(),
			created_seq: 0,
			message_type: 'user',
			conversation_id: currentConversationId
		}

		this.#rowsById[currentConversationId] = [...this.#rowsOf(currentConversationId), userMessage]
		onUserRow?.(userMessage.id)
		const messageContent = this.inputMessage.trim()
		this.inputMessage = ''
		const status = this.#liveStatus(currentConversationId)
		status.isLoading = true
		status.isWaitingForResponse = true

		// This turn's own answer, not shared state: a queued follow-up can flush while this
		// one is still finishing, and re-enter sendMessage before it reads the result.
		let started = false
		try {
			await tick()
			this.scrollToUserMessage(userMessage.id)

			if (this.#useStreaming && this.#path) {
				started = await this.handleStreamingMessage(
					messageContent,
					currentConversationId,
					isNewConversation,
					userMessage.id,
					additionalInputs
				)
			} else {
				started = await this.handlePollingMessage(
					messageContent,
					currentConversationId,
					isNewConversation,
					userMessage.id,
					additionalInputs
				)
			}
		} catch (error) {
			console.error('Error running flow:', error)
			sendUserToast('Failed to run flow: ' + error, true)
			// A turn that never started leaves nothing to wait for. Said here as well as in
			// the finally because the streaming path keeps `isLoading` for its own stream,
			// and without this the composer and the sidebar stay locked until a reload.
			this.#turnFailedToStart(currentConversationId)
			started = false
		} finally {
			if (!this.#useStreaming) {
				status.isLoading = false
			}
		}

		// The row now exists, titled from the message by the server. A name typed while it
		// was a draft has to be written over that — unconditionally, not through
		// renameConversation: on the streaming path this runs before any refresh, so the
		// local row still carries the typed name and an equality check would skip the write.
		// Cleared only once it lands, so a failed run keeps the name for the next attempt.
		// Only when a turn actually ran: nothing created the row otherwise, so the write
		// would 404 and stack a rename failure on top of the real one.
		if (started && this.#draftTitle?.id === currentConversationId) {
			const { title } = this.#draftTitle
			if (await this.#writeConversationTitle(currentConversationId, title)) {
				this.#draftTitle = undefined
			}
		}

		await tick()
		this.focusInput()
		if (!started) {
			// Nothing ran, so the row claiming a turn has to go with it — the caller puts
			// the message back in the composer.
			this.#rowsById[currentConversationId] = this.#rowsOf(currentConversationId).filter(
				(m) => m.id !== userMessage.id
			)
			return false
		}
		return true
	}

	/** Answers whether a job was actually started. */
	private async handleStreamingMessage(
		messageContent: string,
		currentConversationId: string,
		isNewConversation: boolean,
		userRowId: string,
		additionalInputs?: Record<string, any>
	): Promise<boolean> {
		const runtime = this.#liveRuntime(currentConversationId)
		// Close any existing EventSource
		runtime.eventSource?.close()

		// Track stream state for this message
		runtime.turn = emptyTurnState(currentConversationId)
		runtime.replyReveal.reset()
		runtime.reasoningReveal.reset()
		runtime.streamOffset = undefined
		runtime.streamJobId = undefined

		try {
			const jobId = await this.#onRunFlow?.(messageContent, currentConversationId, additionalInputs)
			if (!jobId) {
				console.error('No jobId returned from onRunFlow')
				this.#turnFailedToStart(currentConversationId)
				return false
			}
			// What Stop cancels. Set from the flow job now rather than waiting for
			// `flow_stream_job_id`, which stays null until a step starts streaming — Stop
			// pressed before the first token would otherwise end the turn on screen and leave
			// the flow running.
			this.#nameTurnJob(currentConversationId, userRowId, jobId)

			// start polling
			this.startPolling(currentConversationId, isNewConversation)

			this.#followJob(currentConversationId, jobId)
		} catch (error) {
			// Everything that can throw here happens before the stream is live — the run
			// request itself (which the deployed page's launcher throws from), or building
			// the EventSource. Either way no turn ran.
			console.error('Stream connection error:', error)
			sendUserToast('Failed to connect to stream', true)
			this.endTurn(currentConversationId)
			this.#turnFailedToStart(currentConversationId)
			return false
		}
		return true
	}

	/**
	 * Follow a job that is already running. The server ends every stream after
	 * `TIMEOUT_SSE_STREAM`, so a timeout re-enters here on the same job rather than starting
	 * a second run — which is what it used to do, leaving two runs writing one conversation.
	 */
	#followJob(currentConversationId: string, jobId: string) {
		const runtime = this.#liveRuntime(currentConversationId)
		const status = this.#liveStatus(currentConversationId)
		runtime.eventSource?.close()

		const streamUrl = `/api/w/${this.#workspace()}/jobs_u/getupdate_sse/${jobId}`
		const url = new URL(streamUrl, window.location.origin)
		url.searchParams.set('poll_delay_ms', '50')
		url.searchParams.set('fast', 'true')
		url.searchParams.set('only_result', 'true')
		// Resume after what is already on screen; without it the stream replays from the
		// start and the answer gains a second copy of everything rendered so far.
		if (runtime.streamOffset !== undefined) {
			url.searchParams.set('stream_offset', runtime.streamOffset.toString())
		}
		const eventSource = new EventSource(url.toString())
		runtime.eventSource = eventSource
		let isCompleted = false

		eventSource.onmessage = async (event) => {
			try {
				const data = JSON.parse(event.data)
				const type = data.type

				// The server ends the stream on its own clock; re-attach to the same job.
				if (type === 'timeout') {
					eventSource.close()
					runtime.eventSource = undefined
					this.#followJob(currentConversationId, jobId)
					return
				}

				// Handle ping - just ignore
				if (type === 'ping') {
					return
				}

				// Handle error
				if (type === 'error') {
					eventSource.close()
					runtime.eventSource = undefined
					console.error('SSE error:', data)
					sendUserToast('Stream error: ' + (data.error || 'Unknown error'), true)
					this.endTurn(currentConversationId)
					return
				}

				// Handle not found
				if (type === 'not_found') {
					eventSource.close()
					runtime.eventSource = undefined
					console.error('Job not found')
					sendUserToast('Job not found', true)
					this.endTurn(currentConversationId)
					return
				}

				if (type === 'update') {
					if (data.flow_stream_job_id) {
						status.jobId = data.flow_stream_job_id
						if (data.flow_stream_job_id !== runtime.streamJobId) {
							const offsetFromOtherJob =
								runtime.streamJobId !== undefined && runtime.streamOffset !== undefined
							runtime.streamJobId = data.flow_stream_job_id
							if (offsetFromOtherJob) {
								// The offset indexes the previous sub-job's stream — a retried last step
								// gets a new one — so this connection skipped the new job's first
								// chunks. Drop this delta and re-attach from the start of that stream.
								runtime.streamOffset = undefined
								eventSource.close()
								runtime.eventSource = undefined
								this.#followJob(currentConversationId, jobId)
								return
							}
						}
					}
					if (data.stream_offset !== undefined) {
						runtime.streamOffset = data.stream_offset
					}
					// Process new stream content
					if (data.new_result_stream) {
						// Stop polling since we are receiving last step streaming
						this.stopPolling(currentConversationId)
						// One chunk can carry several events, so each is applied in turn: a
						// chunk holding a call and its result must produce both.
						for (const event of parseStreamEvents(data.new_result_stream)) {
							if (event.kind === 'reasoning') {
								status.isReasoningActive = true
								runtime.reasoningReveal.push(event.content)
							} else if (event.kind === 'token') {
								runtime.replyReveal.push(event.content)
							} else {
								// Whatever the pacing still holds belongs to the row before the tool —
								// thinking that led straight to the call included — so it is revealed
								// before the event that closes that row.
								if (event.kind === 'tool_call' || event.kind === 'tool_execution') {
									this.#flushReveals(currentConversationId)
									status.currentReasoning = ''
									status.isReasoningActive = false
								}
								const step = applyStreamEvent(
									{ rows: this.#rowsOf(currentConversationId), state: runtime.turn },
									event,
									this.#newRowId
								)
								this.#rowsById[currentConversationId] = step.rows
								runtime.turn = step.state
							}
						}
					}

					// Handle completion
					if (data.completed) {
						isCompleted = true
						// Anything still buffered would be dropped by the temp-row sweep below.
						this.#flushReveals(currentConversationId)
						// Do a final poll to get all messages from database
						await this.pollConversationMessages(currentConversationId, {
							removeTempMessages: true
						})
						this.endTurn(currentConversationId, { settled: true })
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
			this.endTurn(currentConversationId)
		}
	}

	/** Answers whether a job was actually started. */
	private async handlePollingMessage(
		messageContent: string,
		currentConversationId: string,
		isNewConversation: boolean,
		userRowId: string,
		additionalInputs?: Record<string, any>
	): Promise<boolean> {
		const jobId = await this.#onRunFlow?.(messageContent, currentConversationId, additionalInputs)
		if (!jobId) {
			console.error('No jobId returned from onRunFlow')
			this.#turnFailedToStart(currentConversationId)
			return false
		}

		// Store the current job ID so it can be cancelled
		this.#nameTurnJob(currentConversationId, userRowId, jobId)

		if (isNewConversation) {
			await this.refreshConversations()
		}

		// Start polling for intermediate messages in non-streaming mode too
		this.startPolling(currentConversationId)
		this.pollJobResult(currentConversationId, jobId)
		return true
	}
}

export const createFlowChatManager = () => new FlowChatManager()
