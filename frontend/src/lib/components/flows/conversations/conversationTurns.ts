import {
	TurnRunningError,
	type Chat,
	type ChatMessage,
	type ChatState,
	type RunningTurn,
	type SendMessageOptions
} from 'windmill-chat'

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

/** A refusal of a send over a turn the chat sees waiting is retried this often, this far apart;
 * a run that keeps refusing (steps after its agent) is then left to queue. */
const PARK_RETRIES = 3
const PARK_RETRY_MS = 1000

export function isBusy(status: ChatState['status']): boolean {
	return status === 'submitted' || status === 'streaming'
}

/** A message written and not yet sent. What it carries besides its text is opaque here. */
export interface Draft<A> {
	text: string
	attachments: readonly A[]
}

export function emptyDraft<A>(): Draft<A> {
	return { text: '', attachments: [] }
}

export function isEmptyDraft<A>(draft: Draft<A>): boolean {
	return draft.text === '' && draft.attachments.length === 0
}

/** `later` after `earlier`, one line each: typing again adds to what waits. */
function mergeDrafts<A>(earlier: Draft<A>, later: Draft<A>): Draft<A> {
	return {
		text: [earlier.text, later.text].filter(Boolean).join('\n'),
		attachments: [...earlier.attachments, ...later.attachments]
	}
}

/** What turns a draft into a send, and hears of a send the chat refused. */
export interface DraftSender<A> {
	/**
	 * The chat's send for `draft`, read as it goes out, so a queued message takes the inputs
	 * of that moment. `replayInputs` are a failed turn's own run arguments, for a retry.
	 * Undefined refuses it, and the draft is handed back.
	 */
	prepareSend(
		draft: Draft<A>,
		replayInputs?: Record<string, unknown>
	): SendMessageOptions | undefined
	/** The chat refused a send outright (an upload that failed, Stop during it); the draft
	 * is handed back after this. */
	sendFailed(error: unknown, draft: Draft<A>): void
	/** The running turn's agent waits on an approval or a question shown here. */
	awaitsAnswer?(): boolean
}

/**
 * The turns of one conversation's chat, and what the reader wrote around them: the queued
 * message waiting for the running turn, and text a refused send handed back for a composer
 * to take. Both exist nowhere else, so a chat holding either is never released.
 */
export class ConversationTurns<A> {
	readonly chat: Chat
	readonly #sender: () => DraftSender<A>
	#queued: Draft<A> = emptyDraft()
	/** A retry's own run arguments, kept only while that retry is all there is to send: merged
	 * with anything else it is no longer that turn's replay, and goes out on current inputs. */
	#queuedReplay: Record<string, unknown> | undefined
	#returned: Draft<A> = emptyDraft()
	/** Settles when the chat has released the last turn sent or followed here. */
	#turnDone: Promise<unknown> = Promise.resolve()
	/** Sends whose outcome has not reached this yet: a refusal hands their draft back. */
	#sending = 0
	/** Stops so far, so a send can tell it was stopped before the chat refused it. */
	#stops = 0
	#status: ChatState['status']
	readonly #listeners = new Set<() => void>()
	readonly #unsubscribe: () => void
	#disposed = false
	/** The job of the turn a message was sent over, until the server takes or refuses it. What
	 * is sent meanwhile is queued behind it: a refused message goes back ahead of the queue. */
	#sentOver: string | undefined
	#waiting = false
	#parkRetries = 0

	constructor(chat: Chat, sender: () => DraftSender<A>) {
		this.chat = chat
		this.#sender = sender
		this.#status = chat.getState().status
		this.#unsubscribe = chat.subscribe((state) => this.#onState(state))
	}

	/** The queued message; empty when nothing waits. */
	get queued(): Draft<A> {
		return this.#queued
	}

	/** Text written here and not sent: queued, or handed back and not yet taken. */
	get holdsText(): boolean {
		return !isEmptyDraft(this.#queued) || !isEmptyDraft(this.#returned)
	}

	/** Nothing here exists anywhere else: no text held, and no send that could still hand
	 * its draft back. The chat publishes `idle` before it rejects a send it withdrew. */
	get releasable(): boolean {
		return !this.holdsText && this.#sending === 0
	}

	/** Calls `listener` whenever the queued or handed-back text changes. */
	subscribe(listener: () => void): () => void {
		this.#listeners.add(listener)
		return () => {
			this.#listeners.delete(listener)
		}
	}

	/**
	 * Sends `draft`, over a running turn too: the server takes it when that turn is parked. One
	 * the server refuses (the turn works), or that carries attachments, is queued behind the
	 * turn. False when nothing was sent: no text, or the sender refused it and it was handed
	 * back. A conversation still answering a message sent elsewhere has that turn followed
	 * here, with this one queued behind it.
	 */
	send = async (draft: Draft<A>, replayInputs?: Record<string, unknown>): Promise<boolean> => {
		const text = draft.text.trim()
		// An AI agent step refuses a run with no `user_message`.
		if (!text) return false
		const message = { text, attachments: draft.attachments }
		const state = this.chat.getState()
		const busy = isBusy(state.status)
		// Sent at once over a turn known to be parked, which the server takes over; a refusal,
		// when the agent woke meanwhile, queues it behind that turn below. A working turn would
		// refuse it, so it is queued here without asking.
		const parked = !!state.waitingOnBackground || !!this.#sender().awaitsAnswer?.()
		const overRunningTurn =
			busy &&
			parked &&
			message.attachments.length === 0 &&
			state.history === 'server' &&
			this.#sentOver === undefined
		// Behind a message still sent over a turn, even one that has ended since.
		if ((busy && !overRunningTurn) || this.#sentOver !== undefined) {
			this.queue(message, replayInputs)
			return true
		}
		const options = this.#sender().prepareSend(message, replayInputs)
		if (!options) {
			this.#handBack(message)
			return false
		}
		const stops = this.#stops
		this.#sending++
		if (overRunningTurn) this.#sentOver = state.runningJobId ?? ''
		// A run that fails is reported through the chat's `onError` and as a failed message;
		// the promise only rejects when the chat refuses the turn outright.
		const turn = this.chat
			.sendMessage(text, overRunningTurn ? { ...options, overRunningTurn } : options)
			.catch((e) => {
				if (overRunningTurn) this.#sentOver = undefined
				if (this.#disposed) return
				// Stop pressed while the run was asked for: the message goes back, whatever the
				// refusal says, rather than out after the turn it names.
				const stopped = this.#stops !== stops
				// A chat that fell back to local history cannot follow the running turn, and a
				// queue that did not wait for it would be refused again as soon as it went out.
				const followable = this.chat.getState().history === 'server'
				if (e instanceof TurnRunningError && !stopped && followable) {
					// Ahead of what was typed while it waited for the refusal: it was written first.
					const alone = isEmptyDraft(this.#queued)
					this.#queued = mergeDrafts(message, this.#queued)
					this.#queuedReplay = alone ? replayInputs : undefined
					this.#notify()
					// Sent over a turn this chat follows already, whose release the queue waits on.
					if (!overRunningTurn) this.resume(e.turn)
					// The agent says it waits before its park is written, and a send in between
					// is refused: try again shortly while it still says so.
					else if (this.#parkRetries < PARK_RETRIES) {
						this.#parkRetries++
						setTimeout(() => this.#flushOverParked(), PARK_RETRY_MS)
					}
					return
				}
				if (!stopped) this.#sender().sendFailed(e, message)
				// What was queued behind it comes back first: the chat publishes `idle` when it
				// withdraws the turn, and a queue left in place would go out as if the turn had run.
				this.dequeue()
				this.#handBack(message)
			})
			.finally(() => {
				this.#sending--
			})
		// Over a running turn, the queue waits for that turn too: it is released when this one is
		// taken, and goes on when this one is refused.
		this.#turnDone = overRunningTurn ? Promise.all([this.#turnDone, turn]) : turn
		await turn
		return true
	}

	/** Follows a turn this chat did not start, so what is queued waits for it. */
	resume(turn: RunningTurn): void {
		this.#turnDone = this.chat.resumeTurn(turn)
	}

	queue(draft: Draft<A>, replayInputs?: Record<string, unknown>): void {
		const text = draft.text.trim()
		if (!text && draft.attachments.length === 0) return
		const alone = isEmptyDraft(this.#queued)
		this.#queued = mergeDrafts(this.#queued, { text, attachments: draft.attachments })
		this.#queuedReplay = alone ? replayInputs : undefined
		this.#notify()
	}

	/** Hands the queued message back for the composer. */
	dequeue(): void {
		const queued = this.#takeQueued()
		if (!isEmptyDraft(queued)) this.#handBack(queued)
	}

	/** Stop means stop: what was queued goes back rather than out after some later turn. */
	stop(): void {
		this.#stops++
		this.dequeue()
		void this.chat.stop()
	}

	/** The handed-back text, for the composer taking it. */
	takeReturned(): Draft<A> {
		const returned = this.#returned
		if (isEmptyDraft(returned)) return returned
		this.#returned = emptyDraft()
		this.#notify()
		return returned
	}

	/** Everything written here and not sent, for the chat taking this one's place. */
	takeHeld(): Draft<A> {
		const held = mergeDrafts(this.#returned, this.#queued)
		this.#returned = emptyDraft()
		this.#queued = emptyDraft()
		this.#queuedReplay = undefined
		this.#notify()
		return held
	}

	adopt(draft: Draft<A>): void {
		if (!isEmptyDraft(draft)) this.#handBack(draft)
	}

	/** Drops what is queued: a flush still waiting on the turn would otherwise start a run
	 * for a chat that is gone. */
	dispose(): void {
		this.#disposed = true
		this.#queued = emptyDraft()
		this.#queuedReplay = undefined
		this.#unsubscribe()
		this.#listeners.clear()
	}

	#onState(state: ChatState): void {
		const was = this.#status
		this.#status = state.status
		// Taken: the chat follows the new turn. The old one ending while the server answers does not
		// release it: the chat refuses another send until this one settles.
		if (
			this.#sentOver !== undefined &&
			state.runningJobId !== undefined &&
			state.runningJobId !== this.#sentOver
		) {
			this.#sentOver = undefined
		}
		// The agent answered and now waits on background jobs: what was queued while it worked
		// goes out over it. Not on an approval or a question, which it would drop unseen.
		const waiting = !!state.waitingOnBackground
		const startsWaiting = waiting && !this.#waiting
		this.#waiting = waiting
		if (startsWaiting) this.#parkRetries = 0
		if (startsWaiting && isBusy(state.status)) queueMicrotask(() => this.#flushOverParked())
		if (!isBusy(was) || isBusy(state.status)) return
		// The turn settled. The queued message goes out once the turn is released, not now:
		// the chat publishes `idle` from inside its own `sendMessage`, which still counts the
		// turn as open until it returns, and a send made before that would be refused as a
		// second turn. After a failure it goes back instead, where the reader would rather look
		// at the error than pile on. A failed flow settles as `idle` too, with its error as the
		// answer, so the messages decide.
		if (state.status === 'idle' && !lastTurnFailed(state.messages)) {
			void this.#turnDone.then(() => this.#flush())
		} else {
			this.dequeue()
		}
	}

	#flush(): void {
		// Same rule as `send`, read before the queue is drained: a turn with no message cannot
		// run, and taking the queue for it would drop the attachments on the floor.
		if (!this.#queued.text || this.#disposed) return
		const replayInputs = this.#queuedReplay
		void this.send(this.#takeQueued(), replayInputs)
	}

	#flushOverParked(): void {
		const state = this.chat.getState()
		if (this.#sentOver === undefined && isBusy(state.status) && state.waitingOnBackground) {
			this.#flush()
		}
	}

	#takeQueued(): Draft<A> {
		const queued = this.#queued
		if (isEmptyDraft(queued)) return queued
		this.#queued = emptyDraft()
		this.#queuedReplay = undefined
		this.#notify()
		return queued
	}

	#handBack(draft: Draft<A>): void {
		// In front, as a composer puts it back: a refused message is handed back after what was
		// queued behind it, and it was written first.
		this.#returned = mergeDrafts(draft, this.#returned)
		this.#notify()
	}

	#notify(): void {
		for (const listener of this.#listeners) listener()
	}
}
