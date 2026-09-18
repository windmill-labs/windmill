import type { Chat, ChatMessage, ChatState, Conversation, RunningTurn } from 'windmill-chat'

/**
 * Whether the turn the user message at `index` started failed: its last row before the
 * next user message reports `success: false`. The last row, not any row: a tool call can
 * fail and the agent still answer, and that turn completed.
 */
export function turnFailed(messages: readonly ChatMessage[], index: number): boolean {
	let last: ChatMessage | undefined
	for (let i = index + 1; i < messages.length; i++) {
		const message = messages[i]
		if (message.role === 'user') break
		last = message
	}
	return last?.success === false
}

/** Whether the latest turn failed, per `turnFailed`. False before any turn. */
export function lastTurnFailed(messages: readonly ChatMessage[]): boolean {
	for (let i = messages.length - 1; i >= 0; i--) {
		if (messages[i].role === 'user') return turnFailed(messages, i)
	}
	return false
}

export function isBusy(status: ChatState['status']): boolean {
	return status === 'submitted' || status === 'streaming'
}

/** Reads of one run that may fail before its row stops saying the turn is running. */
const POLL_GIVE_UP = 3

/** What a conversation's row says about it. */
export type ConversationActivity = 'running' | 'error' | 'idle'

export interface FlowChatPoolState {
	/** The conversation shown. Unset for a new chat that has not run its first turn. */
	selectedId: string | undefined
	/** Only conversations with something to say; absent means idle. */
	activity: Record<string, ConversationActivity>
	/** Answers that arrived in a conversation while another one was shown. */
	unread: Record<string, number>
}

export interface PooledChat<H> {
	chat: Chat
	host: H
}

export interface FlowChatPoolOptions<H> {
	/** A chat on the flow with no conversation selected. */
	createChat(): Chat
	createHost(chat: Chat): H
	disposeHost(host: H): void
	/** Whether the host holds text that was typed and never sent: queued behind the turn,
	 * or handed back by a turn that refused it. Such a chat is never released. */
	hasUnsentDraft(host: H): boolean
	/** Follows a turn another page started, through the host so what it queues waits for it. */
	resumeTurn(host: H, turn: RunningTurn): void
	/** Whether a run has ended, for a running conversation this page holds no chat for. */
	isRunFinished(jobId: string): Promise<boolean>
	/** Settled conversations kept in memory beside the shown and the busy ones. */
	keepSettled?: number
	pollMs?: number
}

interface Entry<H> extends PooledChat<H> {
	unsubscribe: () => void
	lastShownAt: number
	/** Assistant messages already counted, so a message counts once as it settles. */
	counted: Set<string>
	/** Its first page has been read: answers already there were never unread. */
	loaded: boolean
	busy: boolean
	/** The pool's clock when its last turn ended. */
	settledAt: number
}

/**
 * The conversations of one flow chat, each on its own `Chat` so that each can run a turn
 * while another is shown. A `Chat` follows one conversation for its whole life and is
 * never switched, which is what lets its turn keep running in the background.
 *
 * Plain TypeScript on the `windmill-chat` API, with the view host left generic: nothing
 * here is specific to Svelte or to the app, so it can move into the SDK as is.
 */
export class FlowChatPool<H> {
	readonly #options: FlowChatPoolOptions<H>
	readonly #entries = new Map<string, Entry<H>>()
	/** The chat a new conversation starts on; it joins `#entries` once its first turn names it. */
	#draft: Entry<H> | undefined
	/** Turns running in conversations this pool is not following, as the list reported them. */
	readonly #running = new Map<string, RunningTurn>()
	/** Failed reads in a row, per conversation, for a run this pool has no chat for. */
	readonly #pollFailures = new Map<string, number>()
	readonly #unread = new Map<string, number>()
	readonly #listeners = new Set<(state: FlowChatPoolState) => void>()
	#selectedId: string | undefined
	#state: FlowChatPoolState = { selectedId: undefined, activity: {}, unread: {} }
	#poll: ReturnType<typeof setInterval> | undefined
	#clock = 0
	#destroyed = false

	constructor(options: FlowChatPoolOptions<H>) {
		this.#options = options
		this.newChat()
	}

	getState = (): FlowChatPoolState => this.#state

	subscribe = (listener: (state: FlowChatPoolState) => void): (() => void) => {
		this.#listeners.add(listener)
		listener(this.#state)
		return () => {
			this.#listeners.delete(listener)
		}
	}

	/** The chat shown now. */
	get selected(): PooledChat<H> {
		// The shown conversation is never evicted, and forgetting it shows a new chat.
		return (this.#selectedId === undefined ? this.#draft : this.#entries.get(this.#selectedId))!
	}

	/** The chat of a conversation, when this pool holds one. */
	get(conversationId: string): PooledChat<H> | undefined {
		return this.#entries.get(conversationId)
	}

	/** Shows a new chat, reusing the one already waiting for its first message. */
	newChat = (): PooledChat<H> => {
		if (!this.#draft) this.#draft = this.#track(undefined)
		this.#selectedId = undefined
		this.#publish()
		return this.#draft
	}

	/** Shows a conversation, following its running turn when another page started it. */
	select = (conversationId: string): void => {
		let entry = this.#entries.get(conversationId)
		if (!entry) {
			entry = this.#track(conversationId)
			this.#entries.set(conversationId, entry)
		}
		this.#selectedId = conversationId
		entry.lastShownAt = ++this.#clock
		this.#unread.delete(conversationId)
		const turn = this.#running.get(conversationId)
		if (turn && !entry.busy) {
			this.#running.delete(conversationId)
			this.#options.resumeTurn(entry.host, turn)
		} else if (entry.loaded && !entry.busy) {
			// Held while another conversation was shown: another tab may have written since.
			void entry.chat.refreshMessages()
		}
		this.#evict()
		this.#publish()
	}

	/** Marks a listing request; pass the mark to `setListed` with what it returns. */
	listingStarted = (): number => ++this.#clock

	/**
	 * What a listing requested at `since` said. A conversation it reports running that no
	 * chat here is following gets its run polled, so its row stops saying so when the run
	 * ends. A turn that ended here after the request is not running, whatever it said.
	 */
	setListed = (conversations: readonly Conversation[], since: number): void => {
		for (const conversation of conversations) {
			const followed = this.#entries.get(conversation.id)
			const stale = followed && (followed.busy || followed.settledAt > since)
			if (conversation.runningTurn && !stale) {
				this.#running.set(conversation.id, conversation.runningTurn)
			} else {
				this.#running.delete(conversation.id)
			}
		}
		// The shown conversation follows its turn now rather than waiting for a poll to end it.
		if (this.#selectedId !== undefined && this.#running.has(this.#selectedId)) {
			this.select(this.#selectedId)
		}
		this.#schedulePoll()
		this.#publish()
	}

	/** Drops a conversation that no longer exists; a new chat is shown in its place. */
	forget = (conversationId: string): void => {
		const entry = this.#entries.get(conversationId)
		if (entry) this.#release(entry)
		this.#entries.delete(conversationId)
		this.#running.delete(conversationId)
		this.#unread.delete(conversationId)
		if (this.#selectedId === conversationId) this.newChat()
		else this.#publish()
	}

	destroy = (): void => {
		this.#destroyed = true
		clearInterval(this.#poll)
		for (const entry of this.#entries.values()) this.#release(entry)
		if (this.#draft) this.#release(this.#draft)
		this.#entries.clear()
		this.#draft = undefined
		this.#listeners.clear()
	}

	#track(conversationId: string | undefined): Entry<H> {
		const chat = this.#options.createChat()
		// Selected before the host exists: a host treats a change of conversation as the
		// reader leaving one, and this chat never leaves its conversation.
		if (conversationId !== undefined) void chat.selectConversation(conversationId)
		const entry: Entry<H> = {
			chat,
			host: this.#options.createHost(chat),
			unsubscribe: () => {},
			lastShownAt: ++this.#clock,
			counted: new Set(),
			loaded: conversationId === undefined,
			busy: false,
			settledAt: 0
		}
		entry.unsubscribe = chat.subscribe((state) => this.#onChatState(entry, state))
		return entry
	}

	#onChatState(entry: Entry<H>, state: ChatState): void {
		if (this.#destroyed) return
		if (entry === this.#draft && state.conversationId !== undefined) {
			// The new chat's first turn named its conversation.
			this.#draft = undefined
			this.#entries.set(state.conversationId, entry)
			if (this.#selectedId === undefined) this.#selectedId = state.conversationId
		}
		const id = state.conversationId
		if (id === undefined) {
			// The chat gave its conversation back: a new chat's first message never ran, so the
			// conversation the id named was never created. Held under that id, the entry would
			// answer for a conversation that does not exist and mint a second one on the next
			// message, so it goes back to being the chat a new conversation starts on.
			if (entry !== this.#draft) this.#undoNewConversation(entry)
			return
		}
		const busy = isBusy(state.status)
		if (busy) this.#running.delete(id)
		else if (entry.busy) entry.settledAt = ++this.#clock
		entry.busy = busy
		const shown = id === this.#selectedId
		for (const message of state.messages) {
			if (message.role !== 'assistant' || message.pending || entry.counted.has(message.id)) continue
			entry.counted.add(message.id)
			if (!shown && entry.loaded && !state.loadingMessages) {
				this.#unread.set(id, (this.#unread.get(id) ?? 0) + 1)
			}
		}
		if (!state.loadingMessages) entry.loaded = true
		if (!busy) this.#evict()
		this.#publish()
	}

	/** Takes an entry back out of the list of conversations, as the chat that starts one. */
	#undoNewConversation(entry: Entry<H>): void {
		for (const [key, held] of this.#entries) {
			if (held !== entry) continue
			this.#entries.delete(key)
			this.#unread.delete(key)
			this.#running.delete(key)
			if (this.#selectedId === key) this.#selectedId = undefined
		}
		// A new chat opened meanwhile is the draft now; this one has nothing left to show.
		if (this.#draft) this.#release(entry)
		else this.#draft = entry
		this.#publish()
	}

	#activity(id: string): ConversationActivity {
		const state = this.#entries.get(id)?.chat.getState()
		if (this.#running.has(id) || (state && isBusy(state.status))) return 'running'
		if (state && (state.status === 'error' || lastTurnFailed(state.messages))) return 'error'
		return 'idle'
	}

	/** Settled chats past the budget go, least recently shown first; their unread count stays. */
	#evict(): void {
		const settled = [...this.#entries.entries()].filter(
			([id, entry]) =>
				id !== this.#selectedId &&
				!isBusy(entry.chat.getState().status) &&
				!this.#options.hasUnsentDraft(entry.host)
		)
		settled.sort(([, a], [, b]) => b.lastShownAt - a.lastShownAt)
		for (const [id, entry] of settled.slice(this.#options.keepSettled ?? 5)) {
			this.#release(entry)
			this.#entries.delete(id)
		}
	}

	#release(entry: Entry<H>): void {
		entry.unsubscribe()
		this.#options.disposeHost(entry.host)
		entry.chat.destroy()
	}

	#schedulePoll(): void {
		if (this.#running.size === 0) {
			clearInterval(this.#poll)
			this.#poll = undefined
			return
		}
		if (this.#poll) return
		this.#poll = setInterval(() => void this.#pollRuns(), this.#options.pollMs ?? 3000)
	}

	async #pollRuns(): Promise<void> {
		await Promise.all(
			[...this.#running].map(async ([id, turn]) => {
				const finished = await this.#options
					.isRunFinished(turn.jobId)
					.then((done) => {
						this.#pollFailures.delete(id)
						return done
					})
					.catch(() => {
						// A run whose job cannot be read — purged, refused, gone — would otherwise
						// keep its row running and its poll going for the life of the page. After a
						// few tries the row goes quiet; opening the conversation reads its rows.
						const failures = (this.#pollFailures.get(id) ?? 0) + 1
						this.#pollFailures.set(id, failures)
						return failures >= POLL_GIVE_UP
					})
				if (finished && this.#running.get(id) === turn) {
					this.#running.delete(id)
					this.#pollFailures.delete(id)
				}
			})
		)
		if (this.#destroyed) return
		this.#schedulePoll()
		this.#publish()
	}

	#publish(): void {
		if (this.#destroyed) return
		const ids = new Set([...this.#entries.keys(), ...this.#running.keys(), ...this.#unread.keys()])
		const activity: Record<string, ConversationActivity> = {}
		for (const id of ids) {
			const value = this.#activity(id)
			if (value !== 'idle') activity[id] = value
		}
		this.#state = {
			selectedId: this.#selectedId,
			activity,
			unread: Object.fromEntries(this.#unread)
		}
		for (const listener of this.#listeners) listener(this.#state)
	}
}
