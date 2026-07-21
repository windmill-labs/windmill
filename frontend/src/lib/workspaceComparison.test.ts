import { describe, expect, it, vi, beforeEach } from 'vitest'

const compareWorkspaces = vi.fn()
vi.mock('$lib/gen', () => ({
	WorkspaceService: { compareWorkspaces: (...a: unknown[]) => compareWorkspaces(...(a as [])) }
}))

import { fetchWorkspaceComparison } from './workspaceComparison'

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
})
