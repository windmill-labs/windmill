/**
 * Per-key serial executor where only the newest task matters.
 *
 * Tasks queued under one key run strictly one after another, so a slow task's
 * writes can never land after a newer task's. A task superseded before its
 * turn comes is skipped outright; one already running can ask `superseded()`
 * at its own checkpoints and stop writing.
 */
export function createLatestWins() {
	const gens = new Map<string, number>()
	const tails = new Map<string, Promise<void>>()
	return {
		run(key: string, task: (superseded: () => boolean) => Promise<void> | void): void {
			const gen = (gens.get(key) ?? 0) + 1
			gens.set(key, gen)
			const superseded = () => gens.get(key) !== gen
			const tail = tails.get(key) ?? Promise.resolve()
			tails.set(
				key,
				tail.then(async () => {
					if (superseded()) return
					try {
						await task(superseded)
					} catch (e) {
						// One failed task must not strand the ones queued behind it.
						console.error('latestWins: task failed', e)
					}
				})
			)
		}
	}
}
