/**
 * Per-key debouncer with a max-wait ceiling — lodash's
 * `debounce(fn, { wait, maxWait })` but the dispatch is keyed and each
 * `schedule` call carries its own replacement task.
 *
 * Within a "chain" (the run of pending schedules ending at the next
 * fire) the latest `fn` wins and the timer is pushed `debounceMs` into
 * the future. The push is capped so the chain can never sit longer
 * than `maxDebounceMs` from when it started — a constant trickle of
 * fast writes can't delay a save forever.
 *
 * When the timer fires the chain ends; the next `schedule` starts a
 * fresh chain with its own `chainStart`. Keys are independent.
 *
 * Errors from sync throws and from async rejections are logged and
 * swallowed so a bad task doesn't break future scheduling.
 */
import { SvelteSet } from 'svelte/reactivity'

export type DebouncedTask = () => unknown | Promise<unknown>

export type DebouncerByKey = {
	/** Replace any pending task under `key` with `fn`, set/extend the
	 * timer to `min(now + debounceMs, chainStart + maxDebounceMs)`. */
	schedule(key: string, fn: DebouncedTask): void
	/** Clear the timer and drop the pending task for `key` without
	 * running it. Returns true if there was something to cancel. Use
	 * to hand control of a key over to an imperative path (e.g. an
	 * immediate save that supersedes the queued autosave). */
	cancel(key: string): boolean
	/** Reactively whether `key` currently has a queued (not-yet-fired)
	 * task. Backed by a `SvelteSet`, so reading this inside a `$derived`
	 * / `$effect` re-runs when the key's pending state flips. */
	isPending(key: string): boolean
}

type Entry = {
	timer: ReturnType<typeof setTimeout>
	task: DebouncedTask
	/** Wall-clock ms when the current chain's first schedule landed.
	 * The max-wait ceiling is measured from here, not from "now". */
	chainStart: number
}

export function createDebouncerByKey(opts: {
	debounceMs: number
	maxDebounceMs: number
}): DebouncerByKey {
	const { debounceMs, maxDebounceMs } = opts
	const entries = new Map<string, Entry>()
	// Reactive mirror of `entries`' key set. Updated in lock-step with
	// `entries` so `isPending` can be read from a reactive context. A
	// `SvelteSet` (not a plain `$state` field) gives per-key subscriptions
	// — readers only re-run when their own key flips.
	const pendingKeys = new SvelteSet<string>()

	function fire(key: string): void {
		const entry = entries.get(key)
		if (!entry) return
		entries.delete(key)
		// Drop from `pendingKeys` before running the task: the task hands
		// off to the coalescing runner, which flips the key to "running" in
		// the same synchronous tick, so there's no observable gap to "none".
		pendingKeys.delete(key)
		try {
			const result = entry.task()
			if (result && typeof (result as Promise<unknown>).then === 'function') {
				;(result as Promise<unknown>).catch((e) =>
					console.error('debouncerByKey: task rejected', e)
				)
			}
		} catch (e) {
			console.error('debouncerByKey: task threw', e)
		}
	}

	function schedule(key: string, fn: DebouncedTask): void {
		const now = Date.now()
		const existing = entries.get(key)
		const chainStart = existing?.chainStart ?? now
		const fireAt = Math.min(now + debounceMs, chainStart + maxDebounceMs)
		const delay = Math.max(0, fireAt - now)

		if (existing) clearTimeout(existing.timer)
		const timer = setTimeout(() => fire(key), delay)
		entries.set(key, { timer, task: fn, chainStart })
		pendingKeys.add(key)
	}

	function cancel(key: string): boolean {
		const existing = entries.get(key)
		if (!existing) return false
		clearTimeout(existing.timer)
		entries.delete(key)
		pendingKeys.delete(key)
		return true
	}

	function isPending(key: string): boolean {
		return pendingKeys.has(key)
	}

	return { schedule, cancel, isPending }
}
