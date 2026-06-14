/**
 * Per-key coalescing async runner: at most one task running and one pending per
 * key. `submit` runs `fn` now if idle, else REPLACES the pending task (the
 * displaced one never runs); the pending task starts when the running one
 * settles. Collapses bursts of "save latest" to the in-flight call plus the
 * most recent. Keys are independent.
 */
import { SvelteSet } from 'svelte/reactivity'

export type CoalescingTask<T = unknown> = () => T | Promise<T>

/** Rejection reason for a `submitAndWait` (or `cancel`-dropped) task discarded
 * before it ran. Only awaiters see it, not fire-and-forget `submit` callers. */
export class CoalescingDisplacedError extends Error {
	constructor() {
		super('coalescingRunner: pending task displaced before it could run')
		this.name = 'CoalescingDisplacedError'
	}
}

export type CoalescingKeyedRunner = {
	/** Fire-and-forget schedule per the policy above. */
	submit(key: string, fn: CoalescingTask): void
	/** Like `submit`, but the promise resolves/rejects with `fn`'s outcome, or
	 * rejects with `CoalescingDisplacedError` if dropped before running. */
	submitAndWait<T>(key: string, fn: CoalescingTask<T>): Promise<T>
	/** Drop the pending task for `key` (returns whether there was one). Can't
	 * abort an in-flight task. A dropped `submitAndWait` rejects with
	 * `CoalescingDisplacedError`. */
	cancel(key: string): boolean
	/** Reactively whether `key`'s chain is running (SvelteSet-backed). */
	isRunning(key: string): boolean
}

type PendingTask = {
	fn: CoalescingTask
	resolve?: (value: unknown) => void
	reject?: (reason: unknown) => void
}

type Entry = { pending: PendingTask | undefined }

/**
 * @example
 * runner.submit('k', f) // runs immediately
 * runner.submit('k', g) // f running: g pending
 * runner.submit('k', h) // g discarded, h pending; h runs once f settles
 */
export function createCoalescingKeyedRunner(): CoalescingKeyedRunner {
	const state = new Map<string, Entry>()
	// Reactive mirror of keys with a running chain, kept in lock-step with
	// `state` (SvelteSet for per-key `isRunning` subscriptions).
	const runningKeys = new SvelteSet<string>()

	async function chain(key: string, first: PendingTask): Promise<void> {
		let current: PendingTask | undefined = first
		while (current) {
			try {
				const result = await current.fn()
				current.resolve?.(result)
			} catch (e) {
				// Don't kill the chain on failure — later submissions must
				// still run. Awaiters get the error via their promise;
				// fire-and-forget callers get a console.error.
				if (current.reject) current.reject(e)
				else console.error('coalescingRunner: task failed', e)
			}
			const entry = state.get(key)!
			current = entry.pending
			entry.pending = undefined
		}
		state.delete(key)
		runningKeys.delete(key)
	}

	/** Set `task` pending for `key`, displacing (and rejecting) any prior
	 * pending. If the key is idle, start the chain. */
	function setOrDisplace(key: string, task: PendingTask): void {
		const entry = state.get(key)
		if (entry) {
			entry.pending?.reject?.(new CoalescingDisplacedError())
			entry.pending = task
			return
		}
		state.set(key, { pending: undefined })
		runningKeys.add(key)
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

	function isRunning(key: string): boolean {
		return runningKeys.has(key)
	}

	return { submit, submitAndWait, cancel, isRunning }
}
