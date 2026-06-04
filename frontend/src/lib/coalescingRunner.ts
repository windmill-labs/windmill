/**
 * Per-key coalescing async runner.
 *
 * For each key, at most one task is running and at most one is pending.
 * `submit(key, fn)` runs `fn` immediately when the key is idle; otherwise
 * `fn` REPLACES any previously-pending task for that key (the displaced
 * one is dropped and never runs). When the running task settles, the
 * pending task — if any — starts next. Different keys are independent.
 *
 * Use to collapse bursts of "save the latest version" calls down to
 * exactly two runs: the one in flight plus the most recent.
 */
export type CoalescingTask<T = unknown> = () => T | Promise<T>

/** Thrown into the rejection of a `submitAndWait` promise (and a
 * `cancel`-dropped pending task) when the task is discarded before it
 * had a chance to run. Fire-and-forget `submit` callers don't see this
 * — only awaiters do. */
export class CoalescingDisplacedError extends Error {
	constructor() {
		super('coalescingRunner: pending task displaced before it could run')
		this.name = 'CoalescingDisplacedError'
	}
}

export type CoalescingKeyedRunner = {
	/** Fire-and-forget. Schedule `fn` under `key` per the policy above.
	 * Synchronous; `fn` is invoked on the current tick when the key is
	 * idle. If a previously-submitted task is still pending for this
	 * key, it is silently dropped. */
	submit(key: string, fn: CoalescingTask): void
	/** Same scheduling as `submit`, but returns a promise that resolves
	 * with `fn`'s return value when `fn` actually runs, rejects with
	 * `fn`'s throw if `fn` fails, or rejects with `CoalescingDisplacedError`
	 * if this submission is dropped (by `cancel`, or by a later
	 * `submit` / `submitAndWait` for the same key) before it runs. */
	submitAndWait<T>(key: string, fn: CoalescingTask<T>): Promise<T>
	/** Drop the pending (queued) task for `key` without running it.
	 * Returns true if there was something to cancel. Does NOT affect
	 * any task currently in flight — there's no way to abort it. If
	 * the dropped task was submitted via `submitAndWait`, its promise
	 * rejects with `CoalescingDisplacedError`. */
	cancel(key: string): boolean
}

type PendingTask = {
	fn: CoalescingTask
	resolve?: (value: unknown) => void
	reject?: (reason: unknown) => void
}

type Entry = { pending: PendingTask | undefined }

/**
 *
 * @example
 * const runner = createCoalescingKeyedRunner()
 * // f, g, h are async functions
 * runner.submit('key1', f) // run is synchronous but f is async. Here f is ran immediately.
 * runner.submit('key1', g) // f is still running: g is postponed
 * runner.submit('key2', someFn) // someFn runs immediately (different key, unrelated to the rest)
 * runner.submit('key1', h) // f is still running: g is discarded, h is postponed
 * // A while later: f finished : h runs now
 */
export function createCoalescingKeyedRunner(): CoalescingKeyedRunner {
	const state = new Map<string, Entry>()

	async function chain(key: string, first: PendingTask): Promise<void> {
		let current: PendingTask | undefined = first
		while (current) {
			try {
				const result = await current.fn()
				current.resolve?.(result)
			} catch (e) {
				// Don't kill the chain on a task failure — bursty callers
				// rely on later submissions still running. submitAndWait
				// callers see the error via their promise; fire-and-forget
				// submit callers get a console.error so the failure isn't
				// silently swallowed.
				if (current.reject) current.reject(e)
				else console.error('coalescingRunner: task failed', e)
			}
			const entry = state.get(key)!
			current = entry.pending
			entry.pending = undefined
		}
		state.delete(key)
	}

	/** Set `task` as the pending entry for `key`, displacing whatever
	 * was there (and rejecting its promise if it had one). If the key
	 * is idle, set up the entry and start the chain. */
	function setOrDisplace(key: string, task: PendingTask): void {
		const entry = state.get(key)
		if (entry) {
			entry.pending?.reject?.(new CoalescingDisplacedError())
			entry.pending = task
			return
		}
		state.set(key, { pending: undefined })
		void chain(key, task)
	}

	function submit(key: string, fn: CoalescingTask): void {
		setOrDisplace(key, { fn })
	}

	function submitAndWait<T>(key: string, fn: CoalescingTask<T>): Promise<T> {
		return new Promise<T>((resolve, reject) => {
			setOrDisplace(key, {
				fn: fn as CoalescingTask,
				resolve: resolve as (v: unknown) => void,
				reject
			})
		})
	}

	function cancel(key: string): boolean {
		const entry = state.get(key)
		if (!entry?.pending) return false
		const dropped = entry.pending
		entry.pending = undefined
		dropped.reject?.(new CoalescingDisplacedError())
		return true
	}

	return { submit, submitAndWait, cancel }
}
