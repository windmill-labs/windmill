import type { Chat, ChatState, Conversation } from 'windmill-chat'
import { ConversationTurns, isBusy, lastTurnFailed, type DraftSender } from './conversationTurns'

/** Listing reads in a row that may fail before the rows stop saying their turns run. */
const POLL_GIVE_UP = 3
/** Pages of the listing one poll reads at most, looking for the rows it watches. */
const POLL_PAGES = 5
/** How much slower the listing is read once it has stopped answering. */
const RECOVERY_SLOWDOWN = 10

/** What a conversation's row says about it. */
export type ConversationActivity = 'running' | 'error' | 'idle'

/** The part of a listed conversation that says whether a turn runs in it. */
export type ListedConversation = Pick<Conversation, 'id' | 'runningTurn'>

export interface FlowChatPoolState {
	/** The conversation shown. Unset for a new chat that has not run its first turn. */
	selectedId: string | undefined
	/** Only conversations with something to say; absent means idle. */
	activity: Record<string, ConversationActivity>
	/** Answers that arrived in a conversation while another one was shown. */
	unread: Record<string, number>
	/** Conversations holding a queued message. */
	queued: Record<string, true>
}

export interface PooledChat<H> {
	chat: Chat
	host: H
}

export interface FlowChatPoolOptions<H extends DraftSender<A>, A> {
	/** A chat on the flow with no conversation selected. */
	createChat(): Chat
	/** The view of one conversation; its sends go through `turns`. */
	createHost(turns: ConversationTurns<A>): H
	disposeHost(host: H): void
	/**
	 * A page of the flow's conversations of every kind, most recently active first, as the
	 * server lists them now; empty past the last one. Read while a conversation this pool
	 * follows no chat for is running, so its row stops saying so once its turn ends.
	 */
	listRecent(page: number): Promise<readonly ListedConversation[]>
	/** Settled conversations kept in memory beside the shown and the busy ones. */
	keepSettled?: number
	pollMs?: number
}

interface Entry<H, A> extends PooledChat<H> {
	turns: ConversationTurns<A>
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
export class FlowChatPool<H extends DraftSender<A>, A> {
	readonly #options: FlowChatPoolOptions<H, A>
	readonly #entries = new Map<string, Entry<H, A>>()
	/** The chat a new conversation starts on; it joins `#entries` once its first turn names it. */
	#draft: Entry<H, A> | undefined
	/** Turns running in conversations this pool is not following, as a listing reported them. */
	readonly #running = new Map<string, NonNullable<Conversation['runningTurn']>>()
	/** The clock of the listing each conversation's row was last taken from. */
	readonly #listedAt = new Map<string, number>()
	/** Chats on their way out, kept until the send that withdrew them has settled. */
	readonly #retiring = new Set<Entry<H, A>>()
	readonly #unread = new Map<string, number>()
	readonly #listeners = new Set<(state: FlowChatPoolState) => void>()
	#selectedId: string | undefined
	#state: FlowChatPoolState = { selectedId: undefined, activity: {}, unread: {}, queued: {} }
	#poll: ReturnType<typeof setTimeout> | undefined
	#polling = false
	#pollFailures = 0
	/** The listing stopped answering and its rows went quiet: it is read on, slowly, so they
	 * come back when it answers again. */
	#recovering = false
	#clock = 0
	#destroyed = false

	constructor(options: FlowChatPoolOptions<H, A>) {
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
		this.#draft.lastShownAt = ++this.#clock
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
		const leaving = this.#selectedId
		this.#selectedId = conversationId
		entry.lastShownAt = ++this.#clock
		this.#unread.delete(conversationId)
		const turn = this.#running.get(conversationId)
		if (turn && !entry.busy) {
			this.#running.delete(conversationId)
			entry.turns.resume(turn)
		} else if (entry.loaded && !entry.busy) {
			// Held while another conversation was shown: another tab may have written since.
			// A failed read leaves the rows it holds; the next return reads again.
			void entry.chat.refreshMessages().catch(() => {})
		}
		// Not the conversation being left: its composer is still mounted, and hands what the
		// reader wrote in it to its turns only as the panel goes.
		this.#evict(leaving)
		this.#publish()
	}

	/** Marks a listing request; pass the mark to `setListed` with what it returns. */
	listingStarted = (): number => ++this.#clock

	/**
	 * What a listing requested at `since` said. A conversation it reports running that no
	 * chat here is following is watched through `listRecent` until its row stops saying so.
	 * Neither a turn that ended here after the request nor a row a later listing already
	 * reported is taken from it, whichever of the two responses lands last.
	 */
	setListed = (conversations: readonly ListedConversation[], since: number): void => {
		for (const conversation of conversations) {
			const id = conversation.id
			if ((this.#listedAt.get(id) ?? 0) > since) continue
			this.#listedAt.set(id, since)
			const followed = this.#entries.get(id)
			const stale = followed && (followed.busy || followed.settledAt > since)
			if (conversation.runningTurn && !stale) this.#running.set(id, conversation.runningTurn)
			else this.#running.delete(id)
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
		clearTimeout(this.#poll)
		for (const entry of this.#entries.values()) this.#release(entry)
		for (const entry of this.#retiring) this.#release(entry)
		if (this.#draft) this.#release(this.#draft)
		this.#retiring.clear()
		this.#entries.clear()
		this.#draft = undefined
		this.#listeners.clear()
	}

	#track(conversationId: string | undefined): Entry<H, A> {
		const chat = this.#options.createChat()
		// Selected before the host exists: a host treats a change of conversation as the
		// reader leaving one, and this chat never leaves its conversation.
		if (conversationId !== undefined) void chat.selectConversation(conversationId)
		const turns = new ConversationTurns<A>(chat, () => entry.host)
		const entry: Entry<H, A> = {
			chat,
			turns,
			host: this.#options.createHost(turns),
			unsubscribe: () => {},
			lastShownAt: ++this.#clock,
			counted: new Set(),
			loaded: conversationId === undefined,
			busy: false,
			settledAt: 0
		}
		const unsubscribeChat = chat.subscribe((state) => this.#onChatState(entry, state))
		// What waits in a chat decides whether it may be released, and marks its row.
		const unsubscribeTurns = turns.subscribe(() => {
			if (!this.#destroyed) this.#publish()
		})
		entry.unsubscribe = () => {
			unsubscribeChat()
			unsubscribeTurns()
		}
		return entry
	}

	#onChatState(entry: Entry<H, A>, state: ChatState): void {
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
	#undoNewConversation(entry: Entry<H, A>): void {
		for (const [key, held] of this.#entries) {
			if (held !== entry) continue
			this.#entries.delete(key)
			this.#unread.delete(key)
			this.#running.delete(key)
			if (this.#selectedId === key) this.#selectedId = undefined
		}
		if (!this.#draft) {
			this.#draft = entry
			this.#publish()
			return
		}
		// A new chat opened meanwhile is the draft now, so this chat has nowhere to show. It
		// is released only once its send has reported what it could not do — the refusal
		// reaches it after this — and what the reader wrote moves to the chat in its place.
		const kept = this.#draft
		this.#retiring.add(entry)
		setTimeout(() => {
			if (this.#destroyed || !this.#retiring.delete(entry)) return
			kept.turns.adopt(entry.turns.takeHeld())
			this.#release(entry)
			this.#publish()
		}, 0)
		this.#publish()
	}

	#activity(id: string): ConversationActivity {
		const state = this.#entries.get(id)?.chat.getState()
		if (this.#running.has(id) || (state && isBusy(state.status))) return 'running'
		if (state && (state.status === 'error' || lastTurnFailed(state.messages))) return 'error'
		return 'idle'
	}

	/** Settled chats past the budget go, least recently shown first; their unread count stays. */
	#evict(spared?: string): void {
		const settled = [...this.#entries.entries()].filter(
			([id, entry]) =>
				id !== this.#selectedId && !isBusy(entry.chat.getState().status) && entry.turns.releasable
		)
		settled.sort(([, a], [, b]) => b.lastShownAt - a.lastShownAt)
		for (const [id, entry] of settled.slice(this.#options.keepSettled ?? 5)) {
			if (id === spared) continue
			this.#release(entry)
			this.#entries.delete(id)
		}
	}

	#release(entry: Entry<H, A>): void {
		entry.unsubscribe()
		this.#options.disposeHost(entry.host)
		entry.turns.dispose()
		entry.chat.destroy()
	}

	#schedulePoll(): void {
		const every = this.#options.pollMs ?? 3000
		if (this.#running.size === 0 && !this.#recovering) {
			clearTimeout(this.#poll)
			this.#poll = undefined
			return
		}
		if (this.#poll || this.#polling) return
		this.#poll = setTimeout(
			() => void this.#relist(),
			this.#running.size === 0 ? every * RECOVERY_SLOWDOWN : every
		)
	}

	/** One listing for every running row this pool follows no chat for. */
	async #relist(): Promise<void> {
		this.#poll = undefined
		this.#polling = true
		const since = this.listingStarted()
		const watched = [...this.#running.keys()]
		const rows: ListedConversation[] = []
		let failed = false
		try {
			// Page 1 holds the running rows but for a turn that has written nothing for a while,
			// behind conversations active since: the pages after it are read until every watched
			// row is found or the listing ends.
			for (let page = 1; page <= POLL_PAGES; page++) {
				const batch = await this.#options.listRecent(page)
				rows.push(...batch)
				const seen = new Set(rows.map((row) => row.id))
				if (batch.length === 0 || watched.every((id) => seen.has(id))) break
			}
			this.#pollFailures = 0
		} catch {
			failed = true
			// A listing that keeps failing would otherwise keep rows running for the life of the
			// page. After a few tries the rows go quiet, and the listing is read on at a slower
			// cadence so they come back once it answers again.
			if (++this.#pollFailures >= POLL_GIVE_UP) {
				this.#running.clear()
				this.#pollFailures = 0
				this.#recovering = true
			}
		} finally {
			this.#polling = false
		}
		if (this.#destroyed) return
		if (!failed) {
			this.#recovering = false
			// A watched row on no page read is gone from the listing, or has been quiet while
			// more conversations than those pages hold were active: either way it goes quiet
			// here, and opening it reads its own rows. A later listing that reported it stands.
			const seen = new Set(rows.map((row) => row.id))
			for (const id of watched) {
				if (!seen.has(id) && (this.#listedAt.get(id) ?? 0) <= since) this.#running.delete(id)
			}
			this.setListed(rows, since)
			return
		}
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
		const queued: Record<string, true> = {}
		for (const [id, entry] of this.#entries) {
			if (entry.turns.queued.text) queued[id] = true
		}
		this.#state = {
			selectedId: this.#selectedId,
			activity,
			unread: Object.fromEntries(this.#unread),
			queued
		}
		for (const listener of this.#listeners) listener(this.#state)
	}
}
