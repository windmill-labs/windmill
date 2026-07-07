// Perceived-smoothness layer for streamed assistant text.
//
// Providers (notably Anthropic for some model/tier combos) deliver text in
// coarse bursts — tens of tokens batched into one delta every ~450 ms — so the
// raw stream reads as freeze→jump→freeze. This module decouples *display* from
// *arrival*: pushed text lands in a non-reactive buffer, and a paint loop
// reveals a slice at a time through `onReveal`, so the bursts read as continuous
// typing. It is deliberately free of Svelte — the only coupling to reactive
// state is the `onReveal` callback — so the pacing is unit-testable with an
// injected clock and scheduler.

type Schedule = (cb: () => void) => unknown
type Cancel = (handle: unknown) => void

export interface TypewriterRevealOptions {
	/** Called with each revealed slice; the owner appends it to reactive state. */
	onReveal: (chunk: string) => void
	/** Reveal synchronously on push with no pacing (reduced-motion / SSR). */
	instant?: boolean
	/** Target lag between arrival and display, in ms. The one meaningful knob. */
	smoothingMs?: number
	/** Backlog at/above which the whole buffer is dumped in one emit (fast path). */
	maxBacklogChars?: number
	/** Minimum gap between emits, in ms — caps downstream re-parse/reflow frequency. */
	minEmitIntervalMs?: number
	/** Upper bound on the per-emit elapsed time, so a backgrounded-tab resume
	 *  catches up over several emits instead of one large jump. */
	maxCatchupMs?: number
	// Injectables for tests:
	now?: () => number
	schedule?: Schedule
	cancel?: Cancel
}

const defaultNow: () => number =
	typeof performance !== 'undefined' && typeof performance.now === 'function'
		? () => performance.now()
		: () => Date.now()

const hasRAF = typeof requestAnimationFrame !== 'undefined'
const defaultSchedule: Schedule = hasRAF
	? (cb) => requestAnimationFrame(cb)
	: (cb) => setTimeout(cb, 16)
const defaultCancel: Cancel = hasRAF
	? (h) => cancelAnimationFrame(h as number)
	: (h) => clearTimeout(h as ReturnType<typeof setTimeout>)

export class TypewriterReveal {
	private readonly onReveal: (chunk: string) => void
	private readonly instant: boolean
	private readonly smoothingMs: number
	private readonly maxBacklogChars: number
	private readonly minEmitIntervalMs: number
	private readonly maxCatchupMs: number
	private readonly now: () => number
	private readonly schedule: Schedule
	private readonly cancel: Cancel

	// A stable buffer + an index into it: reveal advances `revealed` (O(1) per
	// emit, no re-split). Everything before `revealed` has been emitted.
	private buffer = ''
	private revealed = 0
	private lastEmit: number | null = null
	private handle: unknown = null
	private running = false

	constructor(opts: TypewriterRevealOptions) {
		this.onReveal = opts.onReveal
		this.instant = opts.instant ?? false
		this.smoothingMs = opts.smoothingMs ?? 500
		this.maxBacklogChars = opts.maxBacklogChars ?? 1500
		this.minEmitIntervalMs = opts.minEmitIntervalMs ?? 33
		this.maxCatchupMs = opts.maxCatchupMs ?? 100
		this.now = opts.now ?? defaultNow
		this.schedule = opts.schedule ?? defaultSchedule
		this.cancel = opts.cancel ?? defaultCancel
	}

	/** Enqueue received text. In instant mode it is revealed synchronously. */
	push(text: string): void {
		if (!text) return
		if (this.instant) {
			this.onReveal(text)
			return
		}
		this.buffer += text
		this.ensureRunning()
	}

	/** Reveal everything still buffered now and stop. Call before reading the
	 *  owner's reactive state into committed state, so the read sees the full text. */
	flush(): void {
		this.stop()
		if (this.instant) return
		if (this.revealed < this.buffer.length) {
			this.onReveal(this.buffer.slice(this.revealed))
		}
		// Everything is revealed now, so drop the buffer rather than carry an
		// ever-growing turn's worth of text: onMessageEnd fires flush() at every
		// tool-call boundary, and without this the buffer would keep every prior
		// segment until the turn's reset(). The next push starts a fresh buffer.
		this.buffer = ''
		this.revealed = 0
	}

	/** Drop un-revealed backlog and stop. Call at turn boundaries. */
	reset(): void {
		this.stop()
		this.buffer = ''
		this.revealed = 0
		this.lastEmit = null
	}

	private ensureRunning(): void {
		if (this.running) return
		this.running = true
		// Re-anchor on resume from idle: a long gap since the last emit must not
		// count as elapsed reveal time (the maxCatchupMs clamp only covers a
		// still-running loop whose frame fired late).
		this.lastEmit = null
		this.handle = this.schedule(this.tick)
	}

	private stop(): void {
		if (this.handle != null) {
			this.cancel(this.handle)
			this.handle = null
		}
		this.running = false
	}

	private tick = (): void => {
		this.handle = null
		const t = this.now()
		const backlog = this.buffer.length - this.revealed
		if (backlog <= 0) {
			this.running = false
			return
		}

		if (backlog >= this.maxBacklogChars) {
			// Fast path: a cached/non-streaming reply dumped as one big delta shows
			// instantly instead of being stretched. Steady streaming settles well
			// under the cap, so smoothing only ever applies to genuinely bursty input.
			this.emit(backlog)
			this.lastEmit = t
		} else {
			const first = this.lastEmit === null
			// First emit after (re)start reveals a small nominal slice one frame
			// after arrival, keeping first paint essentially immediate.
			const sinceLast = first ? this.minEmitIntervalMs : t - this.lastEmit!
			if (sinceLast < this.minEmitIntervalMs) {
				// Throttle: too soon since the last emit — wait another frame.
				this.handle = this.schedule(this.tick)
				return
			}
			const elapsedMs = Math.min(sinceLast, this.maxCatchupMs)
			const rate = backlog / this.smoothingMs // chars per ms; grows with backlog
			const n = Math.min(backlog, Math.max(1, Math.floor(rate * elapsedMs)))
			const emitted = this.emit(n)
			this.lastEmit = t
			if (emitted === 0) {
				// Nothing revealable yet (a lone trailing high surrogate awaiting its
				// low half). Suspend; the next push restarts the loop.
				this.running = false
				return
			}
		}

		if (this.revealed < this.buffer.length) {
			this.handle = this.schedule(this.tick)
		} else {
			this.running = false
		}
	}

	// Reveal up to `n` chars from `revealed`, never splitting a surrogate pair.
	// Returns the number of chars actually emitted (0 only when the buffer ends on
	// a lone high surrogate whose low half hasn't arrived).
	private emit(n: number): number {
		let end = Math.min(this.revealed + n, this.buffer.length)
		if (end < this.buffer.length) {
			const c = this.buffer.charCodeAt(end)
			// Landed on a low surrogate → cut before its high half.
			if (c >= 0xdc00 && c <= 0xdfff) end -= 1
		}
		if (end <= this.revealed) {
			// A floor-1 slice landed inside a pair; take the whole pair so we still
			// make progress rather than stalling on the same boundary each tick.
			end = Math.min(this.revealed + 2, this.buffer.length)
		}
		if (end === this.buffer.length && end - 1 >= this.revealed) {
			// Hold back a lone trailing high surrogate: its low half may still be
			// streaming in, and revealing it alone would emit a broken code unit.
			const last = this.buffer.charCodeAt(end - 1)
			if (last >= 0xd800 && last <= 0xdbff) end -= 1
		}
		if (end <= this.revealed) return 0
		const chunk = this.buffer.slice(this.revealed, end)
		this.revealed = end
		this.onReveal(chunk)
		return chunk.length
	}
}
