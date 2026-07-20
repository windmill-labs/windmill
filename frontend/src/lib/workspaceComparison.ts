/**
 * Shared fetch layer for the fork↔parent workspace comparison
 * (`compareWorkspaces`). The comparison is the expensive tally the fork banner,
 * the compare page, and the chat `diff` tool all need — routing every consumer
 * through this module means concurrent requests coalesce (single-flight) and a
 * consumer that tolerates a slightly stale result (`maxAgeMs`) can reuse the
 * fetch another surface just made instead of recomputing it.
 */
import { WorkspaceService, type WorkspaceComparison } from '$lib/gen'

interface CacheEntry {
	fetchedAt: number
	comparison: WorkspaceComparison
}

const cache = new Map<string, CacheEntry>()
const inflight = new Map<string, Promise<WorkspaceComparison>>()

function key(parentWorkspaceId: string, forkWorkspaceId: string): string {
	return `${parentWorkspaceId}:${forkWorkspaceId}`
}

/**
 * Fetch (or reuse) the comparison of `forkWorkspaceId` against its parent.
 * `maxAgeMs` (default 0) is the oldest cached *result* the caller accepts —
 * read-mostly consumers pass a tolerance to piggyback on a recent fetch.
 * Concurrent calls for the same pair always share the in-flight request, even
 * with `maxAgeMs: 0`, so a caller that must observe a mutation it just made
 * should issue the call after the mutation settles (the compare page re-polls
 * for this reason: the backend tallies the comparison asynchronously anyway).
 */
export async function fetchWorkspaceComparison(
	parentWorkspaceId: string,
	forkWorkspaceId: string,
	opts: { maxAgeMs?: number } = {}
): Promise<WorkspaceComparison> {
	const k = key(parentWorkspaceId, forkWorkspaceId)
	const maxAgeMs = opts.maxAgeMs ?? 0
	const cached = cache.get(k)
	if (cached && Date.now() - cached.fetchedAt < maxAgeMs) {
		return cached.comparison
	}
	const pending = inflight.get(k)
	if (pending) return pending
	const run = (async () => {
		const comparison = await WorkspaceService.compareWorkspaces({
			workspace: parentWorkspaceId,
			targetWorkspaceId: forkWorkspaceId
		})
		cache.set(k, { fetchedAt: Date.now(), comparison })
		return comparison
	})()
	inflight.set(k, run)
	try {
		return await run
	} finally {
		inflight.delete(k)
	}
}
