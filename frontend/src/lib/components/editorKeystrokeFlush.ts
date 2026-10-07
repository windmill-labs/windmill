// Mounted code editors holding keystrokes their change debounce has not yet written to
// `code`. Lets an operation about to move or read drafts make those keystrokes land first.
const pending = new Set<() => void>()

export function trackPendingKeystrokes(flush: () => void): () => void {
	pending.add(flush)
	return () => pending.delete(flush)
}

/** Write every editor's debounced keystrokes to its `code` now. */
export function flushPendingKeystrokes(): void {
	for (const flush of [...pending]) flush()
}
