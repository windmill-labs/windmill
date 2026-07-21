/**
 * Shared fetch layer for the fork↔parent workspace comparison
 * (`compareWorkspaces`). The comparison is the expensive tally the fork banner,
 * the compare page, and the chat `diff` tool all need — routing every consumer
 * through this module means concurrent tolerant requests coalesce and a
 * consumer that accepts a slightly stale result (`maxAgeMs`) can reuse the
 * fetch another surface just made instead of recomputing it.
 */
import { WorkspaceService, type WorkspaceComparison } from '$lib/gen'

interface CacheEntry {
	fetchedAt: number
	generation: number
	comparison: WorkspaceComparison
}

interface InflightEntry {
	startedAt: number
	promise: Promise<WorkspaceComparison>
}

const cache = new Map<string, CacheEntry>()
const inflight = new Map<string, InflightEntry>()
// Millisecond timestamps collide under concurrency — ordering between
// requests rides on this monotonic generation instead.
let requestGeneration = 0

function key(parentWorkspaceId: string, forkWorkspaceId: string): string {
	return `${parentWorkspaceId}:${forkWorkspaceId}`
}

/**
 * Fetch (or reuse) the comparison of `forkWorkspaceId` against its parent.
 * `maxAgeMs` (default 0) is the oldest result the caller accepts — applied to
 * cached results AND to joining an in-flight request (a request is as old as
 * its start). `maxAgeMs: 0` therefore always issues a fresh fetch: a caller
 * forcing freshness after a mutation must never adopt a request that began
 * before the mutation.
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
	if (pending && maxAgeMs > 0 && Date.now() - pending.startedAt < maxAgeMs) {
		return pending.promise
	}
	const startedAt = Date.now()
	const generation = ++requestGeneration
	const run = (async () => {
		const comparison = await WorkspaceService.compareWorkspaces({
			workspace: parentWorkspaceId,
			targetWorkspaceId: forkWorkspaceId
		})
		// A superseded (older-generation) request must not clobber a newer result.
		const existing = cache.get(k)
		if (!existing || existing.generation < generation) {
			cache.set(k, { fetchedAt: startedAt, generation, comparison })
		}
		return comparison
	})()
	inflight.set(k, { startedAt, promise: run })
	try {
		return await run
	} finally {
		if (inflight.get(k)?.promise === run) inflight.delete(k)
	}
}
