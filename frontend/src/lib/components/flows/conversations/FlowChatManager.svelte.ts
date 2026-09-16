import type {
	FlowConversation,
	FlowConversationMessage,
	ListFlowConversationsData
} from '$lib/gen/types.gen'
import { FlowConversationsService, JobService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { tick } from 'svelte'
import InfiniteList from '$lib/components/InfiniteList.svelte'
import { workspaceStore, userStore, enterpriseLicense } from '$lib/stores'
import { get } from 'svelte/store'
import { base } from '$lib/base'
import { extractChatAnswer, followJob, WindmillChatApi, type AgentStreamEvent } from 'windmill-chat'
import type { StreamEvent } from '$lib/components/chat/utils'
import { randomUUID } from '$lib/utils/uuid'
import {
	appendRevealed,
	applyStreamEvent,
	mergePersistedRows,
	settleTurnRows,
	turnFailed
} from './turnTranscript'
import { Turn, type RevealKind } from './turn.svelte'

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
}

/**
 * How many times a turn re-opens its stream after the request itself failed, and the first
 * delay between attempts — doubled each time. Enough to outlast a server restart; bounded so
 * a genuinely gone endpoint reports itself rather than retrying under a chat that looks live.
 */
const FOLLOW_RETRIES = 4
const FOLLOW_RETRY_DELAY_MS = 500

/** How often a turn with no stream asks its job whether the run is over. */
const SETTLE_POLL_MS = 2000

/** How many times a finished turn's rows are read back while some are still missing, and the
 * wait between. The worker writes them without waiting for them, so the read that follows the
 * run's own completion can be too early — but only by about as long as an insert takes. */
const RECONCILE_ATTEMPTS = 4
const RECONCILE_DELAY_MS = 300

/** Rows per request when a poll reads a conversation. A batch shorter than this is how the
 * endpoint says there are no more. */
const POLL_PAGE_SIZE = 50
/** Most requests one poll will make. Bounds how much of a conversation one tick reads; what it
 * stops short of is left to the next tick, which resumes from the cursor it reached. */
const POLL_MAX_REQUESTS = 20

/**
 * The agent events the worker streams, in the shape the transcript applies. The SDK names
 * the same six events after the wire protocol; this is the rest of the app's vocabulary.
 */
function toStreamEvent(event: AgentStreamEvent): StreamEvent {
	switch (event.type) {
		case 'token_delta':
			return { kind: 'token', content: event.content }
		case 'reasoning_token_delta':
			return { kind: 'reasoning', content: event.content }
		case 'tool_call':
			return { kind: 'tool_call', callId: event.call_id, name: event.function_name }
		case 'tool_call_arguments':
			return {
				kind: 'tool_arguments',
				callId: event.call_id,
				name: event.function_name,
				arguments: event.arguments
			}
		case 'tool_execution':
			return { kind: 'tool_execution', callId: event.call_id, name: event.function_name }
		case 'tool_result':
			return {
				kind: 'tool_result',
				callId: event.call_id,
				name: event.function_name,
				result: event.result,
				success: event.success
			}
	}
}

/** What a run that failed said, as the line a turn ends on. Windmill wraps a failure in an
 *  error envelope; anything else is shown as it came. */
function failedRunMessage(result: unknown): string | undefined {
	const message = (result as { error?: { message?: unknown } })?.error?.message
	if (typeof message === 'string' && message !== '') return message
	return extractChatAnswer(result)
}

function emptyStatus(): TurnStatus {
	return {
		isLoading: false,
		isWaitingForResponse: false,
		isDispatchingTurn: false
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
	/**
	 * The turn each conversation is on, and the only record of whether it is on one. A turn
	 * is in here from the moment it takes the chat until it is ended.
	 *
	 * `$state` for the record, not for the turns in it: Svelte proxies plain objects and
	 * arrays and leaves class instances alone, which is what `#isCurrent` needs. A proxied
	 * turn would fail `===` against the one its own work holds, and its `AbortController`
	 * would stop working behind the proxy.
	 */
	#turns = $state<Record<string, Turn>>({})

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

	/**
	 * Start a turn on this conversation, ending whatever it was on.
	 *
	 * One turn per conversation at a time: a second would write the same agent memory, and
	 * the chat is held for the length of the first precisely so there cannot be one.
	 */
	#startTurn(conversationId: string, createdConversation = false): Turn {
		this.#turns[conversationId]?.end()
		const turn: Turn = new Turn({
			conversationId,
			createdConversation,
			onReveal: (kind, chunk) => this.#reveal(turn, kind, chunk),
			onPoll: () =>
				this.pollConversationMessages(conversationId, {
					isNewConversation: turn.listPending,
					turn
				})
		})
		this.#turns[conversationId] = turn
		return turn
	}

	/** The turn this conversation is on, if it is on one. */
	#turnOf(conversationId: string): Turn | undefined {
		return this.#turns[conversationId]
	}

	/**
	 * Whether work started by a turn may still write to the chat.
	 *
	 * The one question every write-back asks. A stream frame already in the reader's buffer,
	 * a poll dispatched a tick ago, a settle that outlived its Stop: each resolves after the
	 * turn that wanted it may be gone, and each would otherwise land in whatever turn is
	 * there now.
	 */
	#isCurrent(turn: Turn): boolean {
		return this.#turns[turn.conversationId] === turn && !turn.ended
	}

	#reveal(turn: Turn, kind: RevealKind, chunk: string) {
		if (!this.#isCurrent(turn)) return
		const step = appendRevealed(
			{ rows: this.#rowsOf(turn.conversationId), state: turn.transcript },
			kind,
			chunk,
			() => turn.mintRowId()
		)
		this.#rowsById[turn.conversationId] = step.rows
		turn.transcript = step.state
		if (kind === 'reasoning') turn.reasoning = step.state.reasoning
	}

	/**
	 * Name the run a turn started, on both the status and the message that began it.
	 *
	 * The server writes its own user row carrying this job id, but the poller drops user rows
	 * and nothing the server sends may stand for one, so the row on screen never becomes that
	 * one — it holds the id only after a reload fetches the transcript fresh. Stamping it here
	 * is what makes a retry replay the turn it is looking at rather than run the text again
	 * with whatever the composer holds later.
	 */
	#nameTurnJob(turn: Turn, userRowId: string, jobId: string) {
		// The row is named whatever became of the turn: it is the row this run was started
		// for, and Retry replays that run's arguments from the job named here, as does the
		// chip lane under the message. Both have to keep working on a turn that was stopped.
		this.#rowsById[turn.conversationId] = this.#rowsOf(turn.conversationId).map((row) =>
			row.id === userRowId ? { ...row, job_id: jobId } : row
		)
		// The turn is passed rather than looked up: a Stop landing while the run request was
		// in flight ends this one, and the conversation may be on another by now — naming
		// that one with this run would have Stop cancel a job it does not own.
		if (this.#isCurrent(turn)) turn.jobId = jobId
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
			? (this.#turnOf(this.selectedConversationId)?.reasoning ?? '')
			: ''
	}
	get isReasoningActive(): boolean {
		return this.selectedConversationId
			? (this.#turnOf(this.selectedConversationId)?.reasoningActive ?? false)
			: false
	}
	get currentJobId(): string | undefined {
		return this.selectedConversationId
			? this.#turnOf(this.selectedConversationId)?.jobId
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
		delete this.#turns[conversationId]
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
			// A turn that wrote more rows than the page the transcript opened on leaves no
			// message of its own in it, and every row held is then part of that one turn.
			return turnFailed(rows, -1) ? 'error' : 'idle'
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

	/**
	 * End this turn, if it is still the one its conversation is on.
	 *
	 * What every caller holding a turn means by "end it". A turn that is no longer current
	 * has already been ended by whatever replaced it, so there is nothing to do — and
	 * ending *the conversation's* turn would end the one that replaced it, freeing a chat
	 * whose run is still going.
	 */
	#endTurnIfCurrent(turn: Turn, options?: { settled?: boolean }) {
		if (this.#isCurrent(turn)) this.endTurn(turn.conversationId, options)
	}

	/** Stop following one conversation's turn and forget what it was mid-way through. */
	endTurn(conversationId: string, options?: { settled?: boolean; byUser?: boolean }) {
		// Out of the record first: the turn is no longer this conversation's, so anything it
		// started that resolves from here on fails `#isCurrent` and writes nothing.
		const turn = this.#turns[conversationId]
		delete this.#turns[conversationId]
		turn?.end({ byUser: options?.byUser })
		const status = this.#liveStatus(conversationId)
		status.isLoading = false
		status.isWaitingForResponse = false
		// Including a hold taken before a turn had a job — an upload, or a load working out
		// whether this chat has a run in flight. Stop is the way out of those too, and it
		// reaches them only here.
		status.isDispatchingTurn = false
		// Nothing writes to this conversation's rows any more, so nothing in them may still
		// look like it is being written to — a card whose closing event never came would
		// otherwise keep its spinner for as long as the chat is open. Here rather than where a
		// turn is read back, because Stop, the panel going away and a conversation being
		// deleted are ends too. Only where there are rows: a conversation with none has not
		// been read yet, and giving it an empty list would read as one that has.
		const rows = this.#rowsById[conversationId]
		if (rows?.length) this.#rowsById[conversationId] = settleTurnRows(rows)
		if (options?.settled) this.onTurnSettled?.(conversationId)
	}

	/**
	 * Stop following every turn. Called when the chat goes away, which is the only way one
	 * flow's chat is left: a manager belongs to the panel built for one flow in one
	 * workspace, and `FlowChat` replaces that panel rather than re-pointing it.
	 */
	cleanup() {
		const held = new Set([...Object.keys(this.#status), ...Object.keys(this.#turns)])
		for (const conversationId of held) this.endTurn(conversationId)
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
	#turnFailedToStart(turn: Turn) {
		// The turn goes with the holds it took: nothing ran, so there is nothing for it to be
		// current for. `IfCurrent` because the run request is not tied to the turn's signal —
		// a Stop and a second send during it leave this one answering for a turn that is now
		// streaming, and tearing that down would free a chat whose run carries on.
		this.#endTurnIfCurrent(turn)
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

	/**
	 * A run that came back to a turn that is already over, which the reader sees as a send
	 * that did not happen.
	 *
	 * The run request is not tied to the turn's handle, so the whole of it is a window in
	 * which the turn can end. Stop means the reader did not want this run, so now that it
	 * has a name it is cancelled; every other way a turn ends — the chat closing, a flow
	 * change — has never cancelled a run, and a reload picks that one back up.
	 */
	#abandonLaunchedRun(turn: Turn, jobId: string): false {
		if (turn.endedByUser) void this.#cancelOrphanedRun(jobId)
		return false
	}

	/** Cancel a run nothing is following, reporting only to the console: the reader has
	 *  already been told the turn stopped, and this is the part that was still in flight. */
	async #cancelOrphanedRun(jobId: string) {
		try {
			await JobService.cancelQueuedJob({
				workspace: this.#workspace()!,
				id: jobId,
				requestBody: {}
			})
		} catch (error) {
			console.error('Could not cancel a run left by a turn that was stopped:', error)
		}
	}

	/** Stop the open conversation's turn. Every other chat's keeps running. */
	async cancelCurrentJob() {
		const conversationId = this.selectedConversationId
		if (!this.#workspace() || !conversationId) {
			return
		}
		// Held, not looked up again: the turn can settle while the cancel request is in
		// flight, and the failure path must not hand this job to whatever turn is running by
		// then — it would name that turn with the wrong run.
		const turn = this.#turnOf(conversationId)
		const jobId = turn?.jobId

		try {
			if (jobId) {
				await JobService.cancelQueuedJob({
					workspace: this.#workspace()!,
					id: jobId,
					requestBody: {}
				})
				sendUserToast(`Job ${jobId} cancelled`)
			}
			// The turn that settled during the cancel request has already gone, and the one
			// that replaced it owns a run of its own.
			if (!turn || this.#isCurrent(turn)) this.endTurn(conversationId, { byUser: true })
		} catch (error) {
			// The run may well still be going, and freeing the chat would let the next turn
			// write the same agent memory. It is left to the job to say when it is over.
			console.error('Error cancelling job:', error)
			sendUserToast('Could not stop the run; waiting for it to finish', true)
			// The turn's own follower is still on the job and is what will settle it; another
			// would ask the same question twice. With no turn left there is nothing following
			// it, and the chat is released instead.
			if (!turn || !this.#isCurrent(turn)) this.endTurn(conversationId, { byUser: true })
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

		// The turn the opening read holds the chat with, for as long as it takes to find out
		// whether a run is going. Held here so every write below can ask whether it is still
		// the turn this conversation is on.
		let turn: Turn | undefined
		if (reset) {
			// Rows already held are either what a previous load fetched or what a turn is
			// writing right now; either way re-fetching would drop a live turn's temp rows. A
			// turn with no rows yet is the same chat being opened twice at once, and the
			// second opening has nothing to add. A read that failed leaves neither, which is
			// what lets selecting the chat again retry it.
			if (this.#rowsById[conversationIdToUse] || this.#turnOf(conversationIdToUse)) {
				return
			}
			this.isLoadingMessages = true
			// Whether this chat has a run in flight is unknown until its rows are here and
			// the job has answered, and a message accepted meanwhile starts a second one. The
			// hold is a turn like any other because Stop, the panel going away and a later
			// send all have to reach the read it is waiting on: each ends the turn, and a read
			// whose turn is over writes nowhere.
			turn = this.#startTurn(conversationIdToUse)
			this.#liveStatus(conversationIdToUse).isDispatchingTurn = true
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
				// Stopped, or overtaken by a send that started its own turn while this was in
				// flight. The rows it wrote are the live ones; these are what the conversation
				// looked like before it, and writing them would drop the turn's own.
				if (!turn || !this.#isCurrent(turn)) return
				this.#rowsById[conversationIdToUse] = response
				this.#pagedTo[conversationIdToUse] = 1
				this.isLoadingMessages = false
				void this.#resumeRunningTurn(turn)
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
			if (turn) this.#holdUnreadableChat(turn)
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

	/**
	 * How far this conversation has been read, which is the furthest row it holds and not the
	 * last one in the list. Rows sit in the order they were shown, and a stored row the
	 * transcript could not pair with anything goes on the end whatever its place in the
	 * conversation — read from the end, one of those would move the cursor backwards and keep
	 * it there, re-reading the same tail on every tick for the life of the page.
	 */
	private getLastPersistedMessageSeq(conversationId: string) {
		let furthest: number | undefined
		for (const row of this.#rowsOf(conversationId)) {
			if (row.id.startsWith('temp-')) continue
			if (furthest === undefined || row.created_seq > furthest) furthest = row.created_seq
		}
		return furthest
	}

	/** Read a conversation forward from where the transcript left off, and fold in what comes
	 *  back. Answers how many rows it read, which is how a caller asking repeatedly knows to
	 *  stop. */
	private async pollConversationMessages(
		conversationId: string,
		options?: { isNewConversation?: boolean; turn?: Turn }
	): Promise<number> {
		if (!this.#workspace()) return 0

		try {
			// Paged, not one request: the endpoint answers oldest-first with a limit, so a turn
			// that wrote more rows than a page — an agent calling several tools a round — would
			// hand back its earliest and leave its answer behind.
			const response: ChatMessage[] = []
			// Sequences start at 1, so a conversation with no row to resume from reads from 0
			// rather than with no cursor at all — without one the endpoint answers with the
			// newest page instead of the oldest, which is not a prefix of anything.
			let afterSeq = this.getLastPersistedMessageSeq(conversationId) ?? 0
			for (let request = 0; request < POLL_MAX_REQUESTS; request++) {
				// A read for a turn stops with it: a page boundary is where a poll walking a long
				// conversation notices the chat it was reading for has gone.
				if (options?.turn && !this.#isCurrent(options.turn)) return 0
				const batch = await FlowConversationsService.listConversationMessages({
					workspace: this.#workspace()!,
					conversationId: conversationId,
					page: 1,
					perPage: POLL_PAGE_SIZE,
					afterSeq
				})
				response.push(...batch)
				if (batch.length < POLL_PAGE_SIZE) break
				const furthest = Math.max(...batch.map((m) => m.created_seq))
				if (furthest <= afterSeq) {
					// A full page that leaves the cursor where it was would be asked for again on
					// every tick and answered the same way, so nothing here or later can advance.
					console.warn(
						`Stopped reading conversation ${conversationId} at seq ${afterSeq}: a full ` +
							`page of ${POLL_PAGE_SIZE} rows did not move the cursor`
					)
					this.#turnOf(conversationId)?.stopPolling()
					break
				}
				afterSeq = furthest
			}

			// The read is done; the turn it was for may not be. Everything below writes to the
			// transcript, so it asks the same question every other write-back asks.
			if (options?.turn && !this.#isCurrent(options.turn)) return 0

			if (options?.isNewConversation) {
				await this.refreshConversations()
				// The list has the row now, so the ticks after this one do not ask again.
				if (options.turn) options.turn.listPending = false
			}

			// Written to this conversation's rows, not the open one's: a turn keeps landing
			// rows while the reader is in another chat. User rows are dropped because the row
			// the send put on screen is the one that stays — the server's copy would arrive
			// beside it as a second question.
			this.#rowsById[conversationId] = mergePersistedRows(
				this.#rowsOf(conversationId),
				response.filter((msg) => msg.message_type !== 'user')
			)
			return response.length
		} catch (error) {
			console.error('Polling error:', error)
		}
		return 0
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

		// The turn begins here, before the run: it is what holds the chat for the length of
		// the send, and what a Stop during it has to find.
		const turn = this.#startTurn(currentConversationId, isNewConversation)

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
					turn,
					userMessage.id,
					additionalInputs
				)
			} else {
				started = await this.handlePollingMessage(
					messageContent,
					turn,
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
			this.#turnFailedToStart(turn)
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
		turn: Turn,
		userRowId: string,
		additionalInputs?: Record<string, any>
	): Promise<boolean> {
		const currentConversationId = turn.conversationId
		try {
			const jobId = await this.#onRunFlow?.(messageContent, currentConversationId, additionalInputs)
			if (!jobId) {
				console.error('No jobId returned from onRunFlow')
				this.#turnFailedToStart(turn)
				return false
			}
			// What Stop cancels: the flow job, never the streaming step's own sub-job — which
			// would leave the steps after the agent running.
			this.#nameTurnJob(turn, userRowId, jobId)
			if (!this.#isCurrent(turn)) return this.#abandonLaunchedRun(turn, jobId)

			// start polling
			turn.startPolling()

			// Runs for the length of the turn, and reports its own failures.
			void this.#followJob(turn, jobId)
		} catch (error) {
			// The run request itself, which the deployed page's launcher throws from. The
			// stream is not live yet, so no turn ran.
			console.error('Stream connection error:', error)
			sendUserToast('Failed to connect to stream', true)
			this.#turnFailedToStart(turn)
			return false
		}
		return true
	}

	/**
	 * Follow a job that is already running, to completion.
	 *
	 * `followJob` owns the transport: the server ends every stream after
	 * `TIMEOUT_SSE_STREAM`, and it re-attaches to the same job from the last `stream_offset`.
	 * Starting a second run instead leaves two of them writing one conversation. It
	 * reconnects on an unclean close too, and buffers a chunk that ends mid-line.
	 *
	 * The stream request itself failing — a restarting server, a 502 — is this function's to
	 * absorb, by retrying from the offset already read. A turn whose stream cannot be
	 * recovered is handed to its job rather than ended: the flow may still be running, and
	 * freeing the composer would let the next turn write the same agent memory.
	 */
	async #followJob(turn: Turn, jobId: string) {
		const currentConversationId = turn.conversationId
		const signal = turn.signal
		const api = this.#chatApi()

		// Counted since the last attempt that delivered anything, so a long turn blipping
		// once an hour is not the same as an endpoint that has gone.
		let sinceProgress = 0
		for (;;) {
			let delivered = false
			try {
				for await (const update of followJob(api, jobId, {
					signal,
					// Kept on the turn so a reconnect resumes after what is already on screen. A
					// retried agent step gets its own stream, which `followJob` detects and
					// restarts from, leaving the row holding both attempts — the rows read back
					// take its place, so what is on screen ends up as what was stored either way.
					streamOffset: turn.streamOffset,
					onOffset: (offset) => (turn.streamOffset = offset)
				})) {
					// Frames already buffered by the SSE reader keep arriving after an abort, and
					// the turn they would be applied to is gone by then.
					if (!this.#isCurrent(turn)) return
					delivered = true
					if (update.type === 'stream') {
						// Stop polling since we are receiving last step streaming
						turn.stopPolling()
						// One chunk can carry several events, so each is applied in turn: a
						// chunk holding a call and its result must produce both.
						for (const streamed of update.events) {
							const event = toStreamEvent(streamed)
							if (event.kind === 'reasoning') {
								turn.reasoningActive = true
								turn.pushReasoning(event.content)
							} else if (event.kind === 'token') {
								turn.pushAnswer(event.content)
							} else {
								// Whatever the pacing still holds belongs to the row before the tool —
								// thinking that led straight to the call included — so it is revealed
								// before the event that closes that row.
								if (event.kind === 'tool_call' || event.kind === 'tool_execution') {
									turn.flushReveals()
									turn.reasoning = ''
									turn.reasoningActive = false
								}
								const step = applyStreamEvent(
									{ rows: this.#rowsOf(currentConversationId), state: turn.transcript },
									event,
									() => turn.mintRowId()
								)
								this.#rowsById[currentConversationId] = step.rows
								turn.transcript = step.state
							}
						}
						continue
					}
					// What the pacing still holds belongs on screen before the last read, so the
					// rows it lands can stand for it.
					turn.flushReveals()
					await this.#reconcileTurn(turn)
					this.#endTurnIfCurrent(turn, { settled: true })
				}
				return
			} catch (error) {
				// A Stop, a conversation's turn ending, or the chat going away: whoever ended
				// the turn has already torn it down.
				if (!this.#isCurrent(turn)) return
				console.error('Error following the flow job:', error)
				if (delivered) sinceProgress = 0
				if (sinceProgress < FOLLOW_RETRIES) {
					await new Promise((resolve) =>
						setTimeout(resolve, FOLLOW_RETRY_DELAY_MS * 2 ** sinceProgress)
					)
					sinceProgress++
					if (!this.#isCurrent(turn)) return
					continue
				}
				// Out of attempts, and the run is the only thing that knows whether it is over.
				const reason = (error instanceof Error ? error.message : String(error)).slice(0, 200)
				sendUserToast(
					`Lost the live answer for this turn; it will land when the run finishes. ${reason}`,
					true
				)
				void this.#settleFromJob(turn, jobId)
				return
			}
		}
	}

	/**
	 * Pick a conversation's turn back up if its run is still going.
	 *
	 * Nothing in the browser survives a reload, so a chat opened while its flow is running
	 * would otherwise read as idle: the composer would take a message and the two turns
	 * would write one agent memory. The row the server wrote when the turn started carries
	 * its flow job, which is the only thing that knows whether it is over.
	 *
	 * The chat is held from the moment the question is asked, not from the answer: a send
	 * accepted during that round trip is the very thing this exists to prevent. The turn
	 * doing the holding is the one the opening read started, carried on rather than
	 * replaced — starting a second here would end the first, and with it the read that
	 * produced the rows this is about to reason over.
	 */
	async #resumeRunningTurn(turn: Turn) {
		const conversationId = turn.conversationId
		const status = this.#liveStatus(conversationId)
		try {
			await this.#takeOverRunningTurn(turn, status)
			// The hold this attempt was given back, and only it: a turn that replaced this one
			// took a hold of its own, and releasing that would unlock a chat with a run in it.
			// A turn taken over is busy on its own flags by now.
			if (!this.#turnOf(conversationId) || this.#isCurrent(turn)) {
				status.isDispatchingTurn = false
			}
		} catch (error) {
			console.error('Could not tell whether a conversation had a run in flight:', error)
			this.#holdUnreadableChat(turn)
		}
	}

	/**
	 * Hold a chat whose state could not be read, with nothing following a run for it.
	 *
	 * Whether a run owns this conversation's agent memory is exactly what failed to be read,
	 * so the chat cannot take a message that would write that memory a second time. The turn
	 * that was asking goes with it, having answered nothing and having no run to follow, and
	 * Stop is the reader's way out of what is left.
	 */
	#holdUnreadableChat(turn: Turn) {
		// Only a turn that still holds the chat may hold it again. A read is not tied to the
		// turn's handle, so it runs on past a Stop and fails afterwards — and taking the hold
		// back then would shut a composer the reader has just been given, over a run that was
		// cancelled. The chat going away and a later turn taking it are the same story.
		if (!this.#isCurrent(turn)) return
		this.endTurn(turn.conversationId)
		this.#liveStatus(turn.conversationId).isDispatchingTurn = true
	}

	/**
	 * The message that started the newest turn in a conversation, which is the only turn
	 * that can still be running.
	 *
	 * Asked of the server when the rows on screen do not hold it. The transcript opens on
	 * the newest page and one turn can fill it on its own — an agent writes a row per round
	 * and per tool call — so the message that started a long turn sits an unbounded number
	 * of pages back, and reading towards it is a search with no end the browser can name.
	 * Without it a reload cannot tell a finished conversation from one still running, and
	 * would offer a composer that writes a second turn into the same agent memory.
	 */
	async #newestUserRow(conversationId: string): Promise<ChatMessage | undefined> {
		const held = this.#rowsOf(conversationId).findLast((row) => row.message_type === 'user')
		if (held) return held
		const [newest] = await FlowConversationsService.listConversationMessages({
			workspace: this.#workspace()!,
			conversationId,
			messageType: 'user',
			page: 1,
			perPage: 1
		})
		// No rows at all: a conversation written by something other than a chat, and there is
		// no turn to resume.
		if (!newest) return undefined
		// The kind is checked here rather than trusted to the request, because the caller
		// takes this row's job as the turn's. Every other kind names the agent step's own
		// job, and a turn named with that has Stop cancel the step while the rest of the
		// flow runs on — so a server that did not honour the filter has answered a different
		// question, and the caller is told so rather than handed the wrong run.
		if (newest.message_type !== 'user') {
			throw new Error(
				`Asked conversation ${conversationId} for its newest user message and got a ` +
					`${newest.message_type} one`
			)
		}
		return newest
	}

	async #takeOverRunningTurn(turn: Turn, status: TurnStatus): Promise<boolean> {
		const conversationId = turn.conversationId
		// The newest turn is the only one that can still be running, and the message that
		// started it is named with its flow job by the run that created it. A read that fails
		// leaves the caller to decide what becomes of the chat's hold.
		const startedBy = await this.#newestUserRow(conversationId)
		// Torn down or stopped while that read was in flight. The run is left alone either
		// way: leaving a chat has never cancelled one, and this cannot tell the two apart.
		// A turn that is no longer this conversation's has already been ended by whatever
		// replaced it; ending "the conversation's turn" here would end that one.
		if (!this.#isCurrent(turn)) return false
		if (!startedBy?.job_id) {
			this.endTurn(conversationId)
			return false
		}
		const jobId = startedBy.job_id
		const api = this.#chatApi()
		// Named before the question, not after it: Stop with no job cancels nothing while the
		// run carries on.
		turn.jobId = jobId
		{
			// Asked until answered, for the same reason a turn is: an API that cannot be
			// reached has not said the run is over, and a chat freed on that guess takes a
			// message that writes the same agent memory. Stop is on screen throughout.
			for (;;) {
				if (!this.#isCurrent(turn)) return false
				try {
					const { completed } = await api.getCompletedResult(jobId, turn.signal)
					// Nothing to take over, so the turn opened to ask the question goes with it.
					if (completed) {
						this.#endTurnIfCurrent(turn)
						return false
					}
					break
				} catch (error) {
					if (!this.#isCurrent(turn)) return false
					console.error('Could not tell whether a conversation had a run in flight:', error)
					await new Promise((resolve) => setTimeout(resolve, SETTLE_POLL_MS))
				}
			}
		}
		if (!this.#isCurrent(turn)) return false

		status.isLoading = true
		status.isWaitingForResponse = true
		// Rows written while this turn ran are replayed by the stream it is about to
		// re-attach to — an agent persists one per round — and the transcript has no way to
		// tell a replayed round from a second one. The replay writes them again, and the rows
		// the server has take their place as they are read back.
		//
		// The message that started the turn stays, and when the turn was long enough to push
		// it off the page the transcript opened on it is all that is left: polls only ever
		// add what a turn wrote, never the questions, so a transcript emptied here would come
		// back as answers with nothing asking them. It is also the cursor the next poll reads
		// forward from.
		if (this.#useStreaming) {
			const held = this.#rowsOf(conversationId)
			const startedAt = held.findLastIndex((row) => row.id === startedBy.id)
			this.#rowsById[conversationId] = startedAt >= 0 ? held.slice(0, startedAt + 1) : [startedBy]
		}
		// No catch-up read alongside the re-attach. The stream replays the turn from its
		// start, which is why the rows above were dropped, and a read landing before the
		// first frame would put them back for the replay to write a second time.
		if (this.#useStreaming) {
			void this.#followJob(turn, jobId)
		} else {
			void this.#settleFromJob(turn, jobId)
		}
		return true
	}

	#chatApi(): WindmillChatApi {
		return new WindmillChatApi({
			baseUrl: `${window.location.origin}${base}`,
			workspace: this.#workspace()!,
			// A licensed enterprise server paces the stream at this; every other build ignores
			// the parameter and logs a warning per poll, so it is left off and the server's own
			// pacing stands. A licence is the one signal the browser has, and the chat SDK
			// gates on the same thing.
			pollDelayMs: get(enterpriseLicense) ? 50 : undefined
		})
	}

	/**
	 * Wait out a turn with no stream to follow, on the job itself.
	 *
	 * The run holds the conversation's agent memory, so only the run may say the turn is
	 * over: a chat freed on a guess lets the next turn write the same memory. An API that
	 * cannot be reached has said nothing, so this keeps asking rather than deciding — and
	 * the reader is not trapped, since Stop is on screen throughout and cancels the run.
	 *
	 * Deliberately not `waitJob`, which stops answering without settling: on its fifth
	 * failed request it cancels and returns, leaving its promise pending forever.
	 */
	async #settleFromJob(turn: Turn, jobId: string) {
		// Waiting on the job is waiting with no stream, so the catch-up read is the only thing
		// putting the turn's rows on screen before it ends. Started here so every way of
		// arriving without a stream — a turn that never had one, one whose stream is gone,
		// a conversation picked back up on the polling path — gets it once.
		turn.startPolling()
		const api = this.#chatApi()
		const signal = turn.signal
		let flowResult: unknown
		let flowSucceeded = true
		while (this.#isCurrent(turn)) {
			try {
				const { completed, success, result } = await api.getCompletedResult(jobId, signal)
				if (completed) {
					flowResult = result
					flowSucceeded = success !== false
					break
				}
			} catch (error) {
				if (!this.#isCurrent(turn)) return
				console.error('Could not read the flow job while settling a turn:', error)
			}
			await new Promise((resolve) => setTimeout(resolve, SETTLE_POLL_MS))
		}
		if (!this.#isCurrent(turn)) return
		const read = await this.#reconcileTurn(turn)
		if (!this.#isCurrent(turn)) return
		// Nothing of this turn was read back and nothing was streamed, so the transcript has
		// no answer to show and no later read is coming. The run's own result is the same
		// answer the row would have carried, and showing it is what keeps a finished turn
		// from reading as one that produced nothing. A reload replaces it with the row.
		if (read === 0) {
			// A run that failed says so in an error rather than an answer, and the row has to
			// carry that: it is what the transcript reads the turn's outcome off, which is what
			// offers Retry and what holds a queued message back from running into the same
			// failure.
			const answer = flowSucceeded ? extractChatAnswer(flowResult) : failedRunMessage(flowResult)
			if (typeof answer === 'string' && answer !== '') {
				this.#rowsById[turn.conversationId] = [
					...this.#rowsOf(turn.conversationId),
					{
						id: turn.mintRowId(),
						conversation_id: turn.conversationId,
						message_type: 'assistant',
						content: answer,
						created_at: new Date().toISOString(),
						created_seq: 0,
						job_id: jobId,
						success: flowSucceeded
					} as ChatMessage
				]
			}
		}
		this.#endTurnIfCurrent(turn, { settled: true })
	}

	/**
	 * Read a finished turn's rows back, once the run says it is over. Answers how many it got.
	 *
	 * The worker writes them in transactions it does not wait for, so the first read can
	 * come before the last of them lands, and the answer is routinely the one still missing.
	 * The read is repeated while the turn has rows unaccounted for — or, for a turn that put
	 * nothing on screen itself, while it has seen nothing at all — and then stopped. What is
	 * left standing is what the reader watched arrive, which is the only copy of it there is.
	 * Nothing here concludes anything from the rows not coming.
	 */
	async #reconcileTurn(turn: Turn): Promise<number> {
		let total = 0
		for (let attempt = 1; attempt <= RECONCILE_ATTEMPTS; attempt++) {
			let read = 0
			try {
				read = await this.pollConversationMessages(turn.conversationId, { turn })
			} catch (error) {
				console.error('Could not read a finished turn back:', error)
			}
			if (!this.#isCurrent(turn)) return total
			total += read
			// Rows this turn opened, not the conversation's: they are only ever added, so one
			// an older turn left standing would read as work outstanding and make every turn
			// after it wait the whole read out for nothing. A turn with no stream opens none,
			// and then the rows the worker writes are the only ones it will ever have.
			const outstanding = total === 0 || turn.awaitsRowsIn(this.#rowsOf(turn.conversationId))
			if (!outstanding) break
			// A read that brought nothing is the answer to asking again. Some of what a turn
			// shows is never written down — a tool call the agent abandoned mid-round — and
			// waiting the whole budget out for one of those delays every message queued behind
			// the turn by as long as the budget.
			if (attempt > 1 && read === 0) break
			if (attempt < RECONCILE_ATTEMPTS) {
				await new Promise((resolve) => setTimeout(resolve, RECONCILE_DELAY_MS))
			}
		}
		return total
	}

	/** Answers whether a job was actually started. */
	private async handlePollingMessage(
		messageContent: string,
		turn: Turn,
		userRowId: string,
		additionalInputs?: Record<string, any>
	): Promise<boolean> {
		const currentConversationId = turn.conversationId
		const jobId = await this.#onRunFlow?.(messageContent, currentConversationId, additionalInputs)
		if (!jobId) {
			console.error('No jobId returned from onRunFlow')
			this.#turnFailedToStart(turn)
			return false
		}

		// Store the current job ID so it can be cancelled
		this.#nameTurnJob(turn, userRowId, jobId)
		if (!this.#isCurrent(turn)) return this.#abandonLaunchedRun(turn, jobId)

		if (turn.listPending) {
			await this.refreshConversations()
			turn.listPending = false
		}

		void this.#settleFromJob(turn, jobId)
		return true
	}
}

export const createFlowChatManager = () => new FlowChatManager()
