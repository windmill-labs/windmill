import { describe, it, expect, beforeEach, vi } from 'vitest'

// Capture onDestroy callbacks so we can simulate component teardown without
// a real component context.
const onDestroyCallbacks: Array<() => void> = []

vi.mock('svelte', async (importOriginal) => {
	const actual = (await importOriginal()) as Record<string, unknown>
	return {
		...actual,
		onDestroy: (fn: () => void) => {
			onDestroyCallbacks.push(fn)
		}
	}
})

// Imported AFTER vi.mock so the module sees the mocked onDestroy.
const { UserDraft, __resetUserDraftForTesting } = await import('./userDraft.svelte')
const { workspaceStore } = await import('./stores')

function flushDestroyCallbacks(): void {
	const callbacks = onDestroyCallbacks.splice(0, onDestroyCallbacks.length)
	for (const cb of callbacks) cb()
}

// UserDraft.use debounces localStorage writes by 500 ms via
// useLocalStorageValue. Tests assert localStorage state synchronously after
// writes, so we use fake timers and call this helper to fast-forward past
// the debounce window before each assertion.
function flushPersist(): void {
	vi.runAllTimers()
}

// Helper: localStorage payloads are always wrapped as { value: <draft> } so
// future metadata fields can be added without breaking existing entries.
function wrapped<V>(value: V): string {
	return JSON.stringify({ value })
}

beforeEach(() => {
	__resetUserDraftForTesting()
	onDestroyCallbacks.length = 0
	localStorage.clear()
	workspaceStore.set('test_ws')
	vi.useFakeTimers()
})

describe('UserDraft.save / get / remove (no observers)', () => {
	it('save writes a wrapped { value } payload under the workspace-scoped key', () => {
		UserDraft.save('flow', 'u/me/myflow', { hello: 'world' })

		const raw = localStorage.getItem('userdraft/w/test_ws/flow/u/me/myflow')
		expect(raw).toBe(wrapped({ hello: 'world' }))
	})

	it('get reads from a wrapped localStorage payload when no observer is registered', () => {
		localStorage.setItem('userdraft/w/test_ws/script/u/me/script1', wrapped('code'))

		expect(UserDraft.get('script', 'u/me/script1')).toBe('code')
	})

	it('get returns undefined when nothing is stored', () => {
		expect(UserDraft.get('flow', 'u/me/missing')).toBeUndefined()
	})

	it('get returns undefined when the stored payload is malformed', () => {
		localStorage.setItem('userdraft/w/test_ws/flow/u/me/bad', 'not-json')
		expect(UserDraft.get('flow', 'u/me/bad')).toBeUndefined()
	})

	it('get returns undefined when the stored payload is unwrapped (pre-migration entry)', () => {
		// Drafts written before the wrapping was introduced look like the raw
		// value rather than { value: ... }. They must be ignored rather than
		// surface as undefined-shaped drafts.
		localStorage.setItem('userdraft/w/test_ws/flow/u/me/raw', JSON.stringify({ hello: 'world' }))
		expect(UserDraft.get('flow', 'u/me/raw')).toBeUndefined()
		expect(UserDraft.has('flow', 'u/me/raw')).toBe(false)
	})

	it('remove clears the localStorage entry', () => {
		UserDraft.save('app', 'u/me/app1', { grid: [] })
		expect(localStorage.getItem('userdraft/w/test_ws/app/u/me/app1')).not.toBeNull()

		UserDraft.remove('app', 'u/me/app1')
		expect(localStorage.getItem('userdraft/w/test_ws/app/u/me/app1')).toBeNull()
	})

	it('uses the workspace from opts when provided', () => {
		UserDraft.save('flow', 'u/me/f', 1, { workspace: 'other_ws' })

		expect(localStorage.getItem('userdraft/w/other_ws/flow/u/me/f')).toBe(wrapped(1))
		// Default workspace key must remain empty.
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/f')).toBeNull()
	})

	it('supports trigger kinds as item kinds', () => {
		UserDraft.save('trigger_kafka', 'u/me/topic1', { brokers: ['localhost:9092'] })

		const raw = localStorage.getItem('userdraft/w/test_ws/trigger_kafka/u/me/topic1')
		expect(raw).toBe(wrapped({ brokers: ['localhost:9092'] }))
	})

	it('throws when neither opts.workspace nor $workspaceStore is set', () => {
		workspaceStore.set(undefined)
		expect(() => UserDraft.save('flow', 'u/me/x', 1)).toThrow(/no workspace/)
	})
})

describe('UserDraft.use() — observer sync', () => {
	it('loads the existing localStorage value on first use', () => {
		localStorage.setItem('userdraft/w/test_ws/flow/u/me/loaded', wrapped('preloaded'))

		const handle = UserDraft.use<string>('flow', 'u/me/loaded')
		expect(handle.draft).toBe('preloaded')
	})

	it('two handles on the same key share the same underlying state', () => {
		const a = UserDraft.use<number>('flow', 'u/me/shared')
		const b = UserDraft.use<number>('flow', 'u/me/shared')

		a.draft = 42
		expect(b.draft).toBe(42)

		b.draft = 99
		expect(a.draft).toBe(99)
	})

	it('save() propagates to live use() handles (in-memory)', () => {
		const handle = UserDraft.use<number>('flow', 'u/me/observed')
		expect(handle.draft).toBeUndefined()

		// First write through a live entry is treated as the "initial value"
		// (saveInitialValue=false) and is NOT persisted — observers still see it.
		UserDraft.save('flow', 'u/me/observed', 7)
		expect(handle.draft).toBe(7)
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/observed')).toBeNull()

		// Subsequent writes persist.
		UserDraft.save('flow', 'u/me/observed', 9)
		expect(handle.draft).toBe(9)
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/observed')).toBe(wrapped(9))
	})

	it('remove() clears localStorage without touching the in-memory handle', () => {
		// Seed localStorage so the live handle initialises from it.
		localStorage.setItem('userdraft/w/test_ws/flow/u/me/removed', wrapped(1))
		const handle = UserDraft.use<number>('flow', 'u/me/removed')
		expect(handle.draft).toBe(1)

		UserDraft.remove('flow', 'u/me/removed')
		// Live handle keeps its current value — remove() only wipes the
		// persisted side. This is what lets callers run UserDraft.remove
		// during navigation without flickering the editor UI.
		expect(handle.draft).toBe(1)
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/removed')).toBeNull()
	})

	it('the second write through the handle setter persists to localStorage', () => {
		const handle = UserDraft.use<string>('flow', 'u/me/setter')

		// First write is the baseline — not persisted.
		handle.draft = 'initial'
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/setter')).toBeNull()

		// Second (and onwards) persists.
		handle.draft = 'persisted'
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/setter')).toBe(wrapped('persisted'))
	})

	it('setting handle.draft = undefined after edits removes the localStorage entry', () => {
		const handle = UserDraft.use<string>('flow', 'u/me/clear')
		handle.draft = 'initial' // baseline, not persisted
		handle.draft = 'edited' // persisted
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/clear')).not.toBeNull()

		handle.draft = undefined
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/clear')).toBeNull()
		expect(handle.draft).toBeUndefined()
	})

	it('two handles in different workspaces are isolated', () => {
		const a = UserDraft.use<number>('flow', 'u/me/iso', { workspace: 'ws_a' })
		const b = UserDraft.use<number>('flow', 'u/me/iso', { workspace: 'ws_b' })

		a.draft = 1
		b.draft = 2

		expect(a.draft).toBe(1)
		expect(b.draft).toBe(2)
	})

	it('save() falls back to localStorage when no handle is registered', () => {
		UserDraft.save('flow', 'u/me/noobs', 'fallback')
		// First use() afterwards loads the persisted value.
		const handle = UserDraft.use<string>('flow', 'u/me/noobs')
		expect(handle.draft).toBe('fallback')
	})
})

describe('UserDraft.use() — defaultValue', () => {
	it('returns defaultValue when localStorage has no entry', () => {
		const handle = UserDraft.use<string>('flow', 'u/me/withdefault', { defaultValue: 'fallback' })

		expect(handle.draft).toBe('fallback')
	})

	it('does not persist the defaultValue on first read', () => {
		UserDraft.use<string>('flow', 'u/me/lazyDefault', { defaultValue: 'fallback' })

		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/lazyDefault')).toBeNull()
	})

	it('localStorage value wins over defaultValue', () => {
		localStorage.setItem('userdraft/w/test_ws/flow/u/me/overridden', wrapped('persisted'))

		const handle = UserDraft.use<string>('flow', 'u/me/overridden', {
			defaultValue: 'fallback'
		})

		expect(handle.draft).toBe('persisted')
	})

	it('second write through the setter persists even though defaultValue was set', () => {
		const handle = UserDraft.use<string>('flow', 'u/me/writeDefault', {
			defaultValue: 'fallback'
		})

		// First write is the initial-value baseline.
		handle.draft = 'initial'
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/writeDefault')).toBeNull()

		handle.draft = 'modified'
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/writeDefault')).toBe(
			wrapped('modified')
		)
	})
})

describe('UserDraft — empty path (new-item drafts persist across reloads)', () => {
	it('use() with empty path persists subsequent edits to localStorage', () => {
		const handle = UserDraft.use<number>('flow', '', { defaultValue: 0 })

		// First write under saveInitialValue=false counts as the baseline and
		// is skipped — only the user's subsequent edits persist.
		handle.draft = 99
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/')).toBeNull()
		handle.draft = 100
		flushPersist()
		// The "+ Flow / + Script / …" buttons are expected to call
		// `UserDraft.remove(kind, '')` to wipe before navigating; an
		// unguarded /add reload therefore restores the previous session.
		expect(localStorage.getItem('userdraft/w/test_ws/flow/')).toBe(wrapped(100))
	})

	it('two handles with empty path share state per workspace', () => {
		const a = UserDraft.use<number>('flow', '')
		const b = UserDraft.use<number>('flow', '')

		a.draft = 1
		expect(b.draft).toBe(1)

		b.draft = 2
		expect(a.draft).toBe(2)
	})

	it('save() with empty path writes to localStorage when no handle is live', () => {
		UserDraft.save('flow', '', 5)
		expect(localStorage.getItem('userdraft/w/test_ws/flow/')).toBe(wrapped(5))
	})

	it('get() with empty path falls back to localStorage when no handle is live', () => {
		localStorage.setItem('userdraft/w/test_ws/flow/', wrapped(11))
		expect(UserDraft.get('flow', '')).toBe(11)
	})

	it('remove() with empty path clears localStorage', () => {
		localStorage.setItem('userdraft/w/test_ws/flow/', wrapped(1))
		UserDraft.remove('flow', '')
		expect(localStorage.getItem('userdraft/w/test_ws/flow/')).toBeNull()
	})
})

describe('UserDraft — rev metadata for staleness checks', () => {
	it('setDraftAndMeta atomically stores value + rev, and the first write is still skipped', () => {
		const handle = UserDraft.use<string>('flow', 'u/me/atomic')

		// Single atomic write — under saveInitialValue=false this counts as the
		// initial baseline and shouldn't hit localStorage yet.
		handle.setDraftAndMeta('backendValue', {
			remoteRev: 42,
			remoteDraftRev: '2026-01-01T00:00:00Z'
		})
		expect(handle.draft).toBe('backendValue')
		expect(handle.meta).toEqual({ remoteRev: 42, remoteDraftRev: '2026-01-01T00:00:00Z' })
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/atomic')).toBeNull()

		// A subsequent user edit persists *with* the rev metadata.
		handle.draft = 'userEdit'
		flushPersist()
		const raw = localStorage.getItem('userdraft/w/test_ws/flow/u/me/atomic')
		expect(raw).toBe(
			JSON.stringify({
				value: 'userEdit',
				remoteRev: 42,
				remoteDraftRev: '2026-01-01T00:00:00Z'
			})
		)
	})

	it('setMeta updates only the rev fields, preserving the value', () => {
		const handle = UserDraft.use<string>('flow', 'u/me/setmeta')
		handle.setDraftAndMeta('initial', { remoteRev: 1 }) // baseline, not persisted
		handle.draft = 'edited' // persisted with remoteRev: 1

		handle.setMeta({ remoteRev: 2 })
		expect(handle.draft).toBe('edited')
		expect(handle.meta).toEqual({ remoteRev: 2 })
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/setmeta')).toBe(
			JSON.stringify({ value: 'edited', remoteRev: 2 })
		)
	})

	it('handle.draft setter preserves rev metadata across user edits', () => {
		const handle = UserDraft.use<{ count: number }>('flow', 'u/me/preserve')
		handle.setDraftAndMeta({ count: 0 }, { remoteRev: 'v1' })
		handle.draft = { count: 1 } // first edit, persisted
		handle.draft = { count: 2 } // another edit

		expect(handle.meta).toEqual({ remoteRev: 'v1' })
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/preserve')).toBe(
			JSON.stringify({ value: { count: 2 }, remoteRev: 'v1' })
		)
	})

	it('UserDraft.getMeta reads from localStorage when no live handle exists', () => {
		localStorage.setItem(
			'userdraft/w/test_ws/flow/u/me/getmeta',
			JSON.stringify({ value: 'x', remoteRev: 7, remoteDraftRev: '2026-01-02' })
		)
		expect(UserDraft.getMeta('flow', 'u/me/getmeta')).toEqual({
			remoteRev: 7,
			remoteDraftRev: '2026-01-02'
		})
	})

	it('UserDraft.getMeta returns empty object when there is no entry', () => {
		expect(UserDraft.getMeta('flow', 'u/me/none')).toEqual({})
	})

	it('UserDraft.save preserves persisted rev metadata when no live handle exists', () => {
		localStorage.setItem(
			'userdraft/w/test_ws/flow/u/me/savepreserve',
			JSON.stringify({ value: 'old', remoteRev: 5 })
		)
		UserDraft.save('flow', 'u/me/savepreserve', 'new')

		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/savepreserve')).toBe(
			JSON.stringify({ value: 'new', remoteRev: 5 })
		)
	})

	it('handle.meta is empty for a draft persisted without rev (forward compat with older entries)', () => {
		localStorage.setItem(
			'userdraft/w/test_ws/flow/u/me/legacy',
			JSON.stringify({ value: 'no-rev' })
		)
		const handle = UserDraft.use<string>('flow', 'u/me/legacy')
		expect(handle.draft).toBe('no-rev')
		expect(handle.meta).toEqual({})
	})

	it('setMeta({ force: true }) persists immediately, bypassing the first-write skip', () => {
		localStorage.setItem(
			'userdraft/w/test_ws/flow/u/me/forceack',
			JSON.stringify({ value: 'edited', remoteRev: 'v1' })
		)
		const handle = UserDraft.use<string>('flow', 'u/me/forceack')

		// Without force, this is the entry's first state mutation and gets
		// swallowed by saveInitialValue=false — localStorage would still
		// hold the old remoteRev.
		handle.setMeta({ remoteRev: 'v2' }, { force: true })

		expect(handle.meta).toEqual({ remoteRev: 'v2' })
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/forceack')).toBe(
			JSON.stringify({ value: 'edited', remoteRev: 'v2' })
		)
	})
})

describe('checkStaleness', () => {
	let checkStaleness: (
		meta: { remoteRev?: string | number; remoteDraftRev?: string | number },
		currentRev: string | number | undefined,
		currentDraftRev?: string | number | undefined
	) => 'draft' | 'version' | null

	beforeEach(async () => {
		// Re-import to dodge ESM caching surprises across test files.
		;({ checkStaleness } = await import('./userDraft.svelte'))
	})

	it('returns null for legacy entries with no recorded rev', () => {
		expect(checkStaleness({}, 'h1', '2026-01-01')).toBeNull()
	})

	it('returns null when meta matches current revs exactly', () => {
		expect(checkStaleness({ remoteRev: 'h1', remoteDraftRev: 'd1' }, 'h1', 'd1')).toBeNull()
		expect(checkStaleness({ remoteRev: 'h1' }, 'h1', undefined)).toBeNull()
	})

	it('returns "draft" when a newer DB draft was pushed on the remote', () => {
		expect(checkStaleness({ remoteRev: 'h1', remoteDraftRev: 'd1' }, 'h1', 'd2')).toBe('draft')
	})

	it('returns "draft" when the remote gained a DB draft that we didn\'t baseline against', () => {
		expect(checkStaleness({ remoteRev: 'h1' }, 'h1', 'd1')).toBe('draft')
	})

	it('returns "version" when the deployed rev moved and draft revs match', () => {
		expect(checkStaleness({ remoteRev: 'h1' }, 'h2', undefined)).toBe('version')
	})

	it('returns "version" when the baseline draft was deleted on the remote (no current draft)', () => {
		expect(checkStaleness({ remoteRev: 'h1', remoteDraftRev: 'd1' }, 'h1', undefined)).toBe(
			'version'
		)
	})

	it('prefers "draft" over "version" when both have changed', () => {
		expect(checkStaleness({ remoteRev: 'h1', remoteDraftRev: 'd1' }, 'h2', 'd2')).toBe('draft')
	})
})

describe('UserDraft.use() — reference counting & cleanup', () => {
	it('destroys the entry when the last handle is released', () => {
		// First handle acquires the entry.
		const a = UserDraft.use<number>('flow', 'u/me/ref')
		a.draft = 1 // baseline write — not persisted

		// Second handle increments the count.
		const b = UserDraft.use<number>('flow', 'u/me/ref')
		expect(b.draft).toBe(1)

		// onDestroy for both handles got registered.
		expect(onDestroyCallbacks.length).toBe(2)

		// Releasing one handle keeps the entry alive — save() still updates handle a.
		const firstCb = onDestroyCallbacks.shift()!
		firstCb()

		UserDraft.save('flow', 'u/me/ref', 2)
		expect(a.draft).toBe(2)
		// Now persisted (second write after the baseline).
		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/ref')).toBe(wrapped(2))

		// Releasing the second handle drops the entry; subsequent save()
		// must go straight to localStorage rather than mutating in-memory
		// state (which no longer exists).
		const secondCb = onDestroyCallbacks.shift()!
		secondCb()

		UserDraft.save('flow', 'u/me/ref', 3)
		// UserDraft.save without a live entry writes synchronously.
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/ref')).toBe(wrapped(3))
	})

	it('a fresh use() after cleanup re-reads the latest persisted value', () => {
		const a = UserDraft.use<string>('flow', 'u/me/cycle')
		a.draft = 'initial' // baseline — not persisted
		a.draft = 'edited' // persisted (after debounce)
		flushPersist()
		flushDestroyCallbacks()

		// After all handles release, a brand-new use() must pick up the
		// value persisted to localStorage from the previous round.
		const b = UserDraft.use<string>('flow', 'u/me/cycle')
		expect(b.draft).toBe('edited')
	})

	it('coalesces a typing storm into a single localStorage write per 500 ms window', () => {
		const handle = UserDraft.use<string>('flow', 'u/me/debounce')
		handle.draft = 'baseline' // first write — skipped under saveInitialValue=false

		// Three quick edits inside the 500 ms window: in-memory updates every
		// time, but localStorage stays untouched until the timer fires.
		handle.draft = 'one'
		vi.advanceTimersByTime(100)
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/debounce')).toBeNull()
		handle.draft = 'two'
		vi.advanceTimersByTime(100)
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/debounce')).toBeNull()
		handle.draft = 'three'
		expect(handle.draft).toBe('three')
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/debounce')).toBeNull()

		// After the window elapses, only the latest value lands.
		vi.advanceTimersByTime(500)
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/debounce')).toBe(wrapped('three'))
	})

	it('manualRelease=true skips onDestroy registration and gates cleanup behind handle.release()', () => {
		// Caller opts out of the auto-onDestroy — used by routes that create
		// handles dynamically (e.g. ResourceEditor's per-workspace handles).
		const h = UserDraft.use<string>('flow', 'u/me/manual', { manualRelease: true })
		expect(onDestroyCallbacks.length).toBe(0)
		h.draft = 'initial' // baseline
		h.draft = 'edited' // persisted

		// Without release(), the entry stays alive — a co-resident handle
		// sees the same in-memory state.
		const h2 = UserDraft.use<string>('flow', 'u/me/manual', { manualRelease: true })
		expect(h2.draft).toBe('edited')
		h.release()
		// h2 still holds the entry. Releasing both clears it.
		UserDraft.save('flow', 'u/me/manual', 'edited2')
		expect(h2.draft).toBe('edited2')

		h2.release()
		// Second release on the same handle is a no-op — refcount stays at 0.
		h2.release()
	})
})
