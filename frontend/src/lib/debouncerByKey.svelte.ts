/**
 * Per-key debouncer with a max-wait ceiling (keyed lodash
 * `debounce(fn, { wait, maxWait })`). Within a chain the latest `fn` wins and
 * the timer is pushed `debounceMs` out, capped at `chainStart + maxDebounceMs`
 * so a steady trickle can't defer a save forever. The fire ends the chain; the
 * next `schedule` starts a fresh one. Keys independent; task errors are logged
 * and swallowed.
 */
import { SvelteSet } from 'svelte/reactivity'

export type DebouncedTask = () => unknown | Promise<unknown>

export type DebouncerByKey = {
	/** Replace `key`'s pending task with `fn` and set/extend the timer to
	 * `min(now + debounceMs, chainStart + maxDebounceMs)`. */
	schedule(key: string, fn: DebouncedTask): void
	/** Drop `key`'s pending task and timer (returns whether there was one).
	 * Use to hand a key to an imperative path (e.g. an immediate save). */
	cancel(key: string): boolean
	/** Reactively whether `key` has a queued task (SvelteSet-backed). */
	isPending(key: string): boolean
}

type Entry = {
	timer: ReturnType<typeof setTimeout>
	task: DebouncedTask
	/** Wall-clock ms of the chain's first schedule; the max-wait ceiling
	 * is measured from here, not "now". */
	chainStart: number
}

export function createDebouncerByKey(opts: {
	debounceMs: number
	maxDebounceMs: number
}): DebouncerByKey {
	const { debounceMs, maxDebounceMs } = opts
	const entries = new Map<string, Entry>()
	// Reactive mirror of `entries`' keys, kept in lock-step (SvelteSet for
	// per-key `isPending` subscriptions).
	const pendingKeys = new SvelteSet<string>()

	function fire(key: string): void {
		const entry = entries.get(key)
		if (!entry) return
		entries.delete(key)
		// Drop from `pendingKeys` before running: the task synchronously flips
		// the key to "running" in the coalescing runner, so there's no gap.
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
