/**
 * Shared fetch layer for the fork↔parent workspace comparison
 * (`compareWorkspaces`). The comparison is the expensive tally the fork banner,
 * the compare page, and the chat `diff` tool all need — routing every consumer
 * through this module means concurrent tolerant requests coalesce and a
 * consumer that accepts a slightly stale result (`maxAgeMs`) can reuse the
 * fetch another surface just made instead of recomputing it.
 */
import { get } from 'svelte/store'
import { WorkspaceService, type WorkspaceComparison } from '$lib/gen'
import { usersWorkspaceStore } from '$lib/stores'

interface CacheEntry {
	fetchedAt: number
	generation: number
	comparison: WorkspaceComparison
}

interface InflightEntry {
	startedAt: number
	generation: number
	promise: Promise<WorkspaceComparison>
}

const cache = new Map<string, CacheEntry>()
const inflight = new Map<string, InflightEntry>()
// Millisecond timestamps collide under concurrency — ordering between
// requests rides on this monotonic generation instead.
let requestGeneration = 0
// Per-WORKSPACE floor: any request started at or below this generation is
// pre-invalidation and must neither be joined nor land in the cache. Keyed by
// workspace id (either side of a pair), so even requests the inflight map no
// longer tracks are fenced.
const invalidGenFloor = new Map<string, number>()
// Raised when the authenticated identity changes — fences EVERY earlier
// request at once, including ones whose workspace ids nothing tracks anymore.
let globalGenFloor = 0

function generationFloor(parentWorkspaceId: string, forkWorkspaceId: string): number {
	return Math.max(
		globalGenFloor,
		invalidGenFloor.get(parentWorkspaceId) ?? 0,
		invalidGenFloor.get(forkWorkspaceId) ?? 0
	)
}

// Comparisons are permission-filtered per user but keyed only by workspace
// pair — an SPA logout/login must never serve one account's tallies (or let
// its in-flight fetches land) for another.
let cacheOwner: string | undefined = undefined

function ensureCacheOwner(): void {
	const owner = get(usersWorkspaceStore)?.email
	if (owner === cacheOwner) return
	cacheOwner = owner
	cache.clear()
	inflight.clear()
	invalidGenFloor.clear()
	globalGenFloor = requestGeneration
}
// Comparisons are big; keep only the few pairs a session actually browses.
const MAX_CACHE_ENTRIES = 8

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
	return (await fetchWorkspaceComparisonMeta(parentWorkspaceId, forkWorkspaceId, opts)).comparison
}

export interface WorkspaceComparisonMeta {
	comparison: WorkspaceComparison
	/** When the underlying request STARTED — a reused result is as old as its
	 * fetch, not as old as the reuse. Callers layering their own freshness
	 * window must age from this, or windows compound. */
	fetchedAt: number
	/** Pass to `isComparisonCurrent` to learn whether an invalidation has
	 * outdated this result since. */
	generation: number
}

/** True while no `invalidateWorkspaceComparison` (or identity change) has
 * fenced the request that produced `generation`. */
export function isComparisonCurrent(
	parentWorkspaceId: string,
	forkWorkspaceId: string,
	generation: number
): boolean {
	ensureCacheOwner()
	return generation > generationFloor(parentWorkspaceId, forkWorkspaceId)
}

export async function fetchWorkspaceComparisonMeta(
	parentWorkspaceId: string,
	forkWorkspaceId: string,
	opts: { maxAgeMs?: number } = {}
): Promise<WorkspaceComparisonMeta> {
	ensureCacheOwner()
	const k = key(parentWorkspaceId, forkWorkspaceId)
	const maxAgeMs = opts.maxAgeMs ?? 0
	const cached = cache.get(k)
	if (cached && Date.now() - cached.fetchedAt < maxAgeMs) {
		return {
			comparison: cached.comparison,
			fetchedAt: cached.fetchedAt,
			generation: cached.generation
		}
	}
	const pending = inflight.get(k)
	if (
		pending &&
		maxAgeMs > 0 &&
		Date.now() - pending.startedAt < maxAgeMs &&
		pending.generation > generationFloor(parentWorkspaceId, forkWorkspaceId)
	) {
		return {
			comparison: await pending.promise,
			fetchedAt: pending.startedAt,
			generation: pending.generation
		}
	}
	const startedAt = Date.now()
	const generation = ++requestGeneration
	const run = (async () => {
		const comparison = await WorkspaceService.compareWorkspaces({
			workspace: parentWorkspaceId,
			targetWorkspaceId: forkWorkspaceId
		})
		// A superseded (older-generation) or pre-invalidation request must not
		// clobber a newer result.
		const existing = cache.get(k)
		if (
			generation > generationFloor(parentWorkspaceId, forkWorkspaceId) &&
			(!existing || existing.generation < generation)
		) {
			cache.delete(k)
			cache.set(k, { fetchedAt: startedAt, generation, comparison })
			// Insertion-ordered Map: evict the oldest pairs past the cap.
			while (cache.size > MAX_CACHE_ENTRIES) {
				cache.delete(cache.keys().next().value as string)
			}
		}
		return comparison
	})()
	inflight.set(k, { startedAt, generation, promise: run })
	try {
		return { comparison: await run, fetchedAt: startedAt, generation }
	} finally {
		if (inflight.get(k)?.promise === run) inflight.delete(k)
	}
}

/** Drop cached comparisons involving this workspace on EITHER side — a
 * deploy in a parent moves its forks' tallies too. Also fences in-flight
 * requests: nobody new joins them and their late results never land in the
 * cache. (Workspace ids cannot contain ':', so the matches are exact.) */
export function invalidateWorkspaceComparison(workspaceId: string): void {
	const matches = (k: string) => k.startsWith(`${workspaceId}:`) || k.endsWith(`:${workspaceId}`)
	for (const k of [...cache.keys()]) {
		if (matches(k)) cache.delete(k)
	}
	for (const k of [...inflight.keys()]) {
		if (matches(k)) inflight.delete(k)
	}
	// Fence EVERY request started before this point — including ones the
	// inflight map no longer tracks (superseded requests still resolve late).
	invalidGenFloor.set(workspaceId, requestGeneration)
}
