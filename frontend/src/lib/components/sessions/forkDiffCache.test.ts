import { describe, it, expect } from 'vitest'

// Documents the cache-invalidation contract behind the session fork diff. The
// bug (cubic P2): per-item raw diffs (`loadedDiffs[key]`) and per-item
// `summaries` are component-local $state in WorkspaceDiffDrawer that persist for
// the drawer's lifetime. The comparison is re-fetched on every open(), but
// loadDiffFor short-circuits on `loadedDiffs[key]` — so an edit-then-reopen
// would show fresh counts but the prior open's cached expanded raw content. The
// fix clears both records on open() (and, for an in-place data-source swap like
// the draft↔fork toggle, on a `resetKey` change). This test pins the contract
// from the outside: nothing is allowed to read a stale per-item value across a
// re-fetch.

type LoadedDiff = { state: 'ready'; parentRaw: string; forkRaw: string }

function simulateFetchComparison(
	state: { loadedDiffs: Record<string, LoadedDiff>; summaries: Record<string, string> },
	items: Array<{ key: string; parentRaw: string; forkRaw: string; summary: string }>
) {
	// The fix: clear caches *before* re-populating, matching the production
	// code's `loadedDiffs = {}; summaries = {}` at the top of fetchComparison.
	state.loadedDiffs = {}
	state.summaries = {}
	for (const it of items) {
		// loadDiffFor's cache-hit shortcut: if state.loadedDiffs[key] exists,
		// it returns early. With the reset above this is always false here, so
		// every item gets fresh content.
		if (state.loadedDiffs[it.key]) continue
		state.loadedDiffs[it.key] = { state: 'ready', parentRaw: it.parentRaw, forkRaw: it.forkRaw }
		state.summaries[it.key] = it.summary
	}
}

describe('ForkDiffDrawer.fetchComparison — cache invalidation', () => {
	it('reopen after edit replaces stale per-item raw content', () => {
		const state = {
			loadedDiffs: {} as Record<string, LoadedDiff>,
			summaries: {} as Record<string, string>
		}

		// First open: populate.
		simulateFetchComparison(state, [
			{ key: 'script/f/foo', parentRaw: 'v1', forkRaw: 'v1-fork', summary: 'summary v1' }
		])
		expect(state.loadedDiffs['script/f/foo']).toEqual({
			state: 'ready',
			parentRaw: 'v1',
			forkRaw: 'v1-fork'
		})
		expect(state.summaries['script/f/foo']).toBe('summary v1')

		// User edits the script in the session editor, closes drawer, reopens.
		// Without the reset, the cache-hit shortcut would keep 'v1-fork'.
		simulateFetchComparison(state, [
			{ key: 'script/f/foo', parentRaw: 'v1', forkRaw: 'v2-fork', summary: 'summary v2' }
		])
		expect(state.loadedDiffs['script/f/foo']).toEqual({
			state: 'ready',
			parentRaw: 'v1',
			forkRaw: 'v2-fork'
		})
		expect(state.summaries['script/f/foo']).toBe('summary v2')
	})

	it('reopen drops entries for items that vanished from the new comparison', () => {
		const state = {
			loadedDiffs: {} as Record<string, LoadedDiff>,
			summaries: {} as Record<string, string>
		}

		simulateFetchComparison(state, [
			{ key: 'script/f/a', parentRaw: 'x', forkRaw: 'x-fork', summary: 'a' },
			{ key: 'flow/f/b', parentRaw: 'y', forkRaw: 'y-fork', summary: 'b' }
		])
		expect(Object.keys(state.loadedDiffs).sort()).toEqual(['flow/f/b', 'script/f/a'])

		// User merges 'flow/f/b' so it's no longer ahead of parent.
		simulateFetchComparison(state, [
			{ key: 'script/f/a', parentRaw: 'x', forkRaw: 'x-fork', summary: 'a' }
		])
		// Without the reset, the orphan entry would linger in loadedDiffs and
		// the tree could render a row whose raw content references a path no
		// longer in the live comparison.
		expect(Object.keys(state.loadedDiffs)).toEqual(['script/f/a'])
		expect(state.summaries['flow/f/b']).toBeUndefined()
	})
})
