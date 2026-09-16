import { randomUUID } from '$lib/utils/uuid'
import { emptyTurnState, type TurnState } from './turnTranscript'
import {
	prefersInstantReveal,
	TypewriterReveal
} from '$lib/components/copilot/chat/typewriterReveal'

/** How often a running turn asks the server for the rows it has written so far. */
const POLL_MS = 500
/** How long a turn is polled before the reader is assumed to have left it running. */
const POLL_MAX_MS = 2 * 60 * 1000

export type RevealKind = 'answer' | 'reasoning'

export interface TurnOptions {
	conversationId: string
	/** Paced text leaving the turn, a chunk at a time. Called for the turn's whole life. */
	onReveal: (kind: RevealKind, chunk: string) => void
	/** One tick of the catch-up read while the turn runs. */
	onPoll: () => void
	/** This turn created the conversation, so the sidebar's list has a row it has not seen. */
	createdConversation?: boolean
}

/**
 * One turn of one conversation: the run it started, the handle that stops it, and the
 * display state it is writing.
 *
 * It exists to be identified. A turn's work is a stream, a poll and a settle, each of which
 * outlives the moment it was started in, and each of which writes back to the chat. Holding
 * that work's turn means the chat can ask "is this still the turn I am on?" — one question,
 * asked with `===` — instead of every write-back re-deriving the answer from an abort flag
 * it happens to have in scope. A turn that has been replaced or ended answers no, and its
 * work lands nowhere.
 *
 * Everything a turn has to undo is held here, so ending one is a single call rather than
 * an abort, two timers and a pair of reveals that each caller has to remember.
 */
export class Turn {
	readonly conversationId: string
	readonly #controller = new AbortController()

	/** Stops the work this turn started. Read by everything it hands to the network. */
	get signal(): AbortSignal {
		return this.#controller.signal
	}

	/** The flow job, once the run has one. A turn holding the chat before it has a job — an
	 *  attachment upload, a reload working out whether a run is live — has none, and Stop
	 *  has nothing to cancel. */
	jobId = $state<string | undefined>(undefined)

	/** The model's thinking, as revealed. Read by the chat while the turn is the open one. */
	reasoning = $state('')
	/** From the first thinking token until a tool call closes the row. */
	reasoningActive = $state(false)

	/**
	 * Where the live answer resumes after a dropped connection. Kept for the turn rather
	 * than for one attempt, so a reconnect picks up after what is already on screen.
	 */
	streamOffset: number | undefined = undefined

	/** Which row is open and what is in it. The rows themselves belong to the transcript. */
	transcript: TurnState

	/**
	 * The conversation this turn created is not in the sidebar's list yet. Cleared by the
	 * read that picks it up, so a turn long enough to be read many times asks for the list
	 * once rather than on every tick.
	 */
	listPending = $state(false)

	/**
	 * The rows this turn opened, by the ids it gave them.
	 *
	 * What the last read of a turn waits on. Rows are only ever added to a conversation, so
	 * without this a row an older turn left behind — one whose written copy never arrived —
	 * would read as this turn's work outstanding, and every turn after it would wait out the
	 * full read for something that is never coming.
	 */
	readonly #minted = new Set<string>()

	/** A row id belonging to this turn. */
	mintRowId(): string {
		const id = 'temp-' + randomUUID()
		this.#minted.add(id)
		return id
	}

	/** Whether any row this turn opened is still standing in for one the server has. */
	awaitsRowsIn(rows: { id: string }[]): boolean {
		return rows.some((row) => this.#minted.has(row.id))
	}

	#replyReveal: TypewriterReveal
	#reasoningReveal: TypewriterReveal
	#pollInterval: ReturnType<typeof setInterval> | undefined
	#pollDeadline: ReturnType<typeof setTimeout> | undefined
	#onPoll: () => void
	#ended = false
	#endedByUser = false

	constructor(options: TurnOptions) {
		this.conversationId = options.conversationId
		this.transcript = emptyTurnState(options.conversationId)
		this.listPending = options.createdConversation ?? false
		this.#onPoll = options.onPoll
		const instant = prefersInstantReveal()
		this.#replyReveal = new TypewriterReveal({
			onReveal: (chunk) => options.onReveal('answer', chunk),
			instant
		})
		this.#reasoningReveal = new TypewriterReveal({
			onReveal: (chunk) => options.onReveal('reasoning', chunk),
			instant
		})
	}

	/**
	 * The reader pressed Stop on this turn, rather than it being replaced or the chat going
	 * away. What separates "this run was not wanted" from "nobody is watching any more":
	 * leaving a chat has never cancelled a run, and Stop has always meant to.
	 */
	get endedByUser(): boolean {
		return this.#endedByUser
	}

	/** Stopped, or replaced by a later turn that ended this one. */
	get ended(): boolean {
		return this.#ended || this.#controller.signal.aborted
	}

	pushAnswer(chunk: string) {
		if (this.ended) return
		this.#replyReveal.push(chunk)
	}

	pushReasoning(chunk: string) {
		if (this.ended) return
		this.#reasoningReveal.push(chunk)
	}

	/** Put what the pacing still holds on screen now. What is buffered belongs to the row
	 *  being closed — a tool card, or the end of the turn — and would otherwise be dropped. */
	flushReveals() {
		this.#replyReveal.flush()
		this.#reasoningReveal.flush()
	}

	/** The catch-up read, for the rows a turn writes that the stream does not carry. Safe to
	 *  call more than once; the deadline bounds a turn whose reader has gone. */
	startPolling() {
		if (this.#pollInterval || this.ended) return
		this.#pollInterval = setInterval(() => this.#onPoll(), POLL_MS)
		this.#pollDeadline = setTimeout(() => this.stopPolling(), POLL_MAX_MS)
	}

	/** Both timers, always together: a deadline left armed by a turn that stopped early
	 *  would fire into whatever is polling by then. */
	stopPolling() {
		if (this.#pollInterval) {
			clearInterval(this.#pollInterval)
			this.#pollInterval = undefined
		}
		if (this.#pollDeadline) {
			clearTimeout(this.#pollDeadline)
			this.#pollDeadline = undefined
		}
	}

	/**
	 * Stop everything this turn started. The run itself is not cancelled here, whoever ends
	 * the turn: leaving a chat has never stopped a flow, and the one caller that does mean to
	 * cancel owns the request itself — this only records that it was them, in `endedByUser`.
	 */
	end(options?: { byUser?: boolean }) {
		this.#ended = true
		if (options?.byUser) this.#endedByUser = true
		this.stopPolling()
		this.#replyReveal.reset()
		this.#reasoningReveal.reset()
		this.transcript = emptyTurnState(this.conversationId)
		this.reasoning = ''
		this.reasoningActive = false
		this.#controller.abort()
	}
}
