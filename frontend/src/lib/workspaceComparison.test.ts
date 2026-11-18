import { describe, expect, it, vi, beforeEach } from 'vitest'

const compareWorkspaces = vi.fn()
vi.mock('$lib/gen', () => ({
	WorkspaceService: { compareWorkspaces: (...a: unknown[]) => compareWorkspaces(...(a as [])) }
}))
vi.mock('$lib/stores', async () => {
	const { writable } = await import('svelte/store')
	return { usersWorkspaceStore: writable(undefined) }
})

import { fetchWorkspaceComparison, invalidateWorkspaceComparison } from './workspaceComparison'
import { usersWorkspaceStore } from '$lib/stores'

function deferred<T>() {
	let resolve!: (v: T) => void
	const promise = new Promise<T>((res) => (resolve = res))
	return { promise, resolve }
}

beforeEach(() => {
	compareWorkspaces.mockReset()
})

describe('fetchWorkspaceComparison', () => {
	it('a freshness-forced call never adopts an older in-flight request', async () => {
		const first = deferred<unknown>()
		compareWorkspaces.mockImplementationOnce(() => first.promise)
		compareWorkspaces.mockResolvedValueOnce({ summary: { total_diffs: 1 } })

		// Tolerant caller starts a request (e.g. the fork banner)...
		const tolerant = fetchWorkspaceComparison('p', 'f-forced', { maxAgeMs: 30_000 })
		// ...a mutation happens, then a forced caller must get its OWN fetch.
		const forced = fetchWorkspaceComparison('p', 'f-forced', { maxAgeMs: 0 })
		expect(compareWorkspaces).toHaveBeenCalledTimes(2)

		first.resolve({ summary: { total_diffs: 0 } })
		expect((await forced).summary.total_diffs).toBe(1)
		expect((await tolerant).summary.total_diffs).toBe(0)
	})

	it('a superseded older request never overwrites a newer result, even same-millisecond', async () => {
		vi.useFakeTimers()
		try {
			const older = deferred<unknown>()
			compareWorkspaces.mockImplementationOnce(() => older.promise)
			compareWorkspaces.mockResolvedValueOnce({ summary: { total_diffs: 7 } })

			// Same frozen Date.now() for both requests.
			const tolerant = fetchWorkspaceComparison('p', 'f-race', { maxAgeMs: 30_000 })
			const forced = fetchWorkspaceComparison('p', 'f-race', { maxAgeMs: 0 })
			// Newer (forced) resolves FIRST; older resolves after with stale data.
			expect((await forced).summary.total_diffs).toBe(7)
			older.resolve({ summary: { total_diffs: 0 } })
			await tolerant

			// A tolerant read must see the newer result, not the late stale write.
			const reread = await fetchWorkspaceComparison('p', 'f-race', { maxAgeMs: 30_000 })
			expect(reread.summary.total_diffs).toBe(7)
			expect(compareWorkspaces).toHaveBeenCalledTimes(2)
		} finally {
			vi.useRealTimers()
		}
	})

	it('invalidation evicts by EITHER side and fences in-flight requests', async () => {
		compareWorkspaces.mockResolvedValueOnce({ summary: { total_diffs: 0 } })
		compareWorkspaces.mockResolvedValueOnce({ summary: { total_diffs: 3 } })

		// Banner prewarms the cache pre-deploy...
		await fetchWorkspaceComparison('p-side', 'f-inval', { maxAgeMs: 30_000 })
		// ...a deploy in the PARENT invalidates too...
		invalidateWorkspaceComparison('p-side')
		// ...so even a first-ever tolerant read cannot reuse the stale tally.
		const fresh = await fetchWorkspaceComparison('p-side', 'f-inval', { maxAgeMs: 30_000 })
		expect(fresh.summary.total_diffs).toBe(3)
		expect(compareWorkspaces).toHaveBeenCalledTimes(2)
	})

	it('a pre-invalidation in-flight request is not joined and cannot land in the cache', async () => {
		const stale = deferred<unknown>()
		compareWorkspaces.mockImplementationOnce(() => stale.promise)
		compareWorkspaces.mockResolvedValueOnce({ summary: { total_diffs: 9 } })

		const preMutation = fetchWorkspaceComparison('p', 'f-fence', { maxAgeMs: 30_000 })
		invalidateWorkspaceComparison('f-fence')
		// Tolerant post-mutation read: must NOT join the fenced request.
		const post = fetchWorkspaceComparison('p', 'f-fence', { maxAgeMs: 30_000 })
		expect(compareWorkspaces).toHaveBeenCalledTimes(2)

		stale.resolve({ summary: { total_diffs: 0 } })
		await preMutation
		expect((await post).summary.total_diffs).toBe(9)
		// The stale request's late completion never landed in the cache.
		const reread = await fetchWorkspaceComparison('p', 'f-fence', { maxAgeMs: 30_000 })
		expect(reread.summary.total_diffs).toBe(9)
		expect(compareWorkspaces).toHaveBeenCalledTimes(2)
	})

	it('a superseded request fenced by invalidation cannot repopulate the cache late', async () => {
		const superseded = deferred<unknown>()
		compareWorkspaces.mockImplementationOnce(() => superseded.promise)
		compareWorkspaces.mockResolvedValueOnce({ summary: { total_diffs: 5 } })
		compareWorkspaces.mockResolvedValueOnce({ summary: { total_diffs: 6 } })

		// Request A pends; forced request B replaces it in the inflight map and completes.
		const a = fetchWorkspaceComparison('p', 'f-late', { maxAgeMs: 30_000 })
		await fetchWorkspaceComparison('p', 'f-late', { maxAgeMs: 0 })
		// Invalidation happens while A (no longer tracked in inflight) still pends.
		invalidateWorkspaceComparison('f-late')
		// A resolves late with pre-mutation data — it must NOT land in the cache.
		superseded.resolve({ summary: { total_diffs: 0 } })
		await a
		const read = await fetchWorkspaceComparison('p', 'f-late', { maxAgeMs: 30_000 })
		expect(read.summary.total_diffs).toBe(6)
	})

	it('tolerant callers join a recent in-flight request', async () => {
		const first = deferred<unknown>()
		compareWorkspaces.mockImplementationOnce(() => first.promise)

		const a = fetchWorkspaceComparison('p', 'f-join', { maxAgeMs: 30_000 })
		const b = fetchWorkspaceComparison('p', 'f-join', { maxAgeMs: 30_000 })
		expect(compareWorkspaces).toHaveBeenCalledTimes(1)

		first.resolve({ summary: { total_diffs: 2 } })
		expect((await a).summary.total_diffs).toBe(2)
		expect((await b).summary.total_diffs).toBe(2)
	})
	it('an account switch clears cached comparisons', async () => {
		usersWorkspaceStore.set({ email: 'first@x.dev' } as any)
		compareWorkspaces.mockResolvedValueOnce({ summary: { total_diffs: 3 } })
		await fetchWorkspaceComparison('p', 'f-owner', { maxAgeMs: 30_000 })
		await fetchWorkspaceComparison('p', 'f-owner', { maxAgeMs: 30_000 })
		expect(compareWorkspaces).toHaveBeenCalledTimes(1)

		compareWorkspaces.mockResolvedValueOnce({ summary: { total_diffs: 9 } })
		usersWorkspaceStore.set({ email: 'second@x.dev' } as any)
		const fresh = await fetchWorkspaceComparison('p', 'f-owner', { maxAgeMs: 30_000 })
		expect(fresh.summary.total_diffs).toBe(9)
		expect(compareWorkspaces).toHaveBeenCalledTimes(2)
	})

	it('a request started under the previous account never joins or lands after a switch', async () => {
		const old = deferred<unknown>()
		compareWorkspaces.mockImplementationOnce(() => old.promise)
		compareWorkspaces.mockResolvedValueOnce({ summary: { total_diffs: 5 } })

		usersWorkspaceStore.set({ email: 'a@x.dev' } as any)
		fetchWorkspaceComparison('p', 'f-switch', { maxAgeMs: 30_000 })
		usersWorkspaceStore.set({ email: 'b@x.dev' } as any)
		const after = fetchWorkspaceComparison('p', 'f-switch', { maxAgeMs: 30_000 })
		// The new account must not have joined the old account's request.
		expect(compareWorkspaces).toHaveBeenCalledTimes(2)

		old.resolve({ summary: { total_diffs: 0 } })
		expect((await after).summary.total_diffs).toBe(5)
		// And the old account's late result must not have landed in the cache.
		const reread = await fetchWorkspaceComparison('p', 'f-switch', { maxAgeMs: 30_000 })
		expect(reread.summary.total_diffs).toBe(5)
		expect(compareWorkspaces).toHaveBeenCalledTimes(2)
	})
})
