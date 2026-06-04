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
export type DebouncedTask = () => unknown | Promise<unknown>

export type DebouncerByKey = {
	/** Replace any pending task under `key` with `fn`, set/extend the
	 * timer to `min(now + debounceMs, chainStart + maxDebounceMs)`. */
	schedule(key: string, fn: DebouncedTask): void
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

	function fire(key: string): void {
		const entry = entries.get(key)
		if (!entry) return
		entries.delete(key)
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
	}

	return { schedule }
}
