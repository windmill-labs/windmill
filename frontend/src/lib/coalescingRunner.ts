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
export type CoalescingTask = () => unknown | Promise<unknown>

export type CoalescingKeyedRunner = {
	/** Schedule `fn` under `key` per the policy above. Synchronous; `fn`
	 * is invoked on the current tick when the key is idle. */
	submit(key: string, fn: CoalescingTask): void
}

type Entry = { pending: CoalescingTask | undefined }

export function createCoalescingKeyedRunner(): CoalescingKeyedRunner {
	const state = new Map<string, Entry>()

	async function chain(key: string, first: CoalescingTask): Promise<void> {
		let current: CoalescingTask | undefined = first
		while (current) {
			try {
				await current()
			} catch (e) {
				// Don't kill the chain on a task failure — bursty callers
				// rely on later submissions still running.
				console.error('coalescingRunner: task failed', e)
			}
			const entry = state.get(key)!
			current = entry.pending
			entry.pending = undefined
		}
		state.delete(key)
	}

	function submit(key: string, fn: CoalescingTask): void {
		const entry = state.get(key)
		if (entry) {
			// Drop whatever was queued — only the latest submission matters.
			entry.pending = fn
			return
		}
		state.set(key, { pending: undefined })
		void chain(key, fn)
	}

	return { submit }
}
