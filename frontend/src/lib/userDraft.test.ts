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
const { UserDraft, normalizeForCompare, localDraftDiffers, __resetUserDraftForTesting } =
	await import('./userDraft.svelte')
const { workspaceStore } = await import('./stores')
const { deleteGlobalDraft } = await import('./components/copilot/chat/global/userDraftAdapter')

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

// Helper: read a localStorage entry, strip the GC `lastWrittenAt` stamp so
// assertions can stay focused on value + rev metadata. Real entries always
// carry `lastWrittenAt` once written; the GC tests below assert on it
// directly via `localStorage.getItem`.
function storedShape(key: string): string | null {
	const raw = localStorage.getItem(key)
	if (raw == null) return null
	const parsed = JSON.parse(raw)
	delete parsed.lastWrittenAt
	return JSON.stringify(parsed)
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

		expect(storedShape('userdraft/w/test_ws/flow/u/me/myflow')).toBe(wrapped({ hello: 'world' }))
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

		expect(storedShape('userdraft/w/other_ws/flow/u/me/f')).toBe(wrapped(1))
		// Default workspace key must remain empty.
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/f')).toBeNull()
	})

	it('supports trigger kinds as item kinds', () => {
		UserDraft.save('trigger_kafka', 'u/me/topic1', { brokers: ['localhost:9092'] })

		expect(storedShape('userdraft/w/test_ws/trigger_kafka/u/me/topic1')).toBe(
			wrapped({ brokers: ['localhost:9092'] })
		)
	})

	it('throws when neither opts.workspace nor $workspaceStore is set', () => {
		workspaceStore.set(undefined)
		expect(() => UserDraft.save('flow', 'u/me/x', 1)).toThrow(/no workspace/)
	})
})

describe('UserDraft live editor draft registry', () => {
	it('stores the live editor storage path and effective path per workspace and kind', () => {
		UserDraft.setLiveEditorDraft({
			itemKind: 'script',
			storagePath: '',
			effectivePath: 'u/me/generated_script'
		})

		expect(UserDraft.getLiveEditorDraft('script')).toEqual({
			workspace: 'test_ws',
			itemKind: 'script',
			storagePath: '',
			effectivePath: 'u/me/generated_script'
		})
	})

	it('keeps live editor registrations isolated by workspace', () => {
		UserDraft.setLiveEditorDraft({
			workspace: 'ws_a',
			itemKind: 'flow',
			storagePath: '',
			effectivePath: 'u/me/a'
		})
		UserDraft.setLiveEditorDraft({
			workspace: 'ws_b',
			itemKind: 'flow',
			storagePath: '',
			effectivePath: 'u/me/b'
		})

		expect(UserDraft.getLiveEditorDraft('flow', { workspace: 'ws_a' })?.effectivePath).toBe(
			'u/me/a'
		)
		expect(UserDraft.getLiveEditorDraft('flow', { workspace: 'ws_b' })?.effectivePath).toBe(
			'u/me/b'
		)
	})

	it('clears only the matching live editor storage path when provided', () => {
		UserDraft.setLiveEditorDraft({
			itemKind: 'raw_app',
			storagePath: '',
			effectivePath: 'u/me/live_app'
		})

		UserDraft.clearLiveEditorDraft('raw_app', { storagePath: 'u/me/other' })
		expect(UserDraft.getLiveEditorDraft('raw_app')).toBeDefined()

		UserDraft.clearLiveEditorDraft('raw_app', { storagePath: '' })
		expect(UserDraft.getLiveEditorDraft('raw_app')).toBeUndefined()
	})

	it('can remove persisted global draft storage without blanking the live editor', () => {
		const draft = { path: 'u/me/live_script', content: 'export async function main() {}' }
		localStorage.setItem('userdraft/w/test_ws/script/', wrapped(draft))
		const handle = UserDraft.use<typeof draft>('script', '')
		UserDraft.setLiveEditorDraft({
			itemKind: 'script',
			storagePath: '',
			effectivePath: 'u/me/live_script'
		})

		deleteGlobalDraft('test_ws', 'script', 'u/me/live_script', undefined, {
			preserveLiveDraft: true
		})
		flushPersist()

		expect(handle.draft).toEqual(draft)
		expect(localStorage.getItem('userdraft/w/test_ws/script/')).toBeNull()
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

	it('save() propagates to live use() handles and persists immediately', () => {
		const handle = UserDraft.use<number>('flow', 'u/me/observed')
		expect(handle.draft).toBeUndefined()

		UserDraft.save('flow', 'u/me/observed', 7)
		expect(handle.draft).toBe(7)
		expect(storedShape('userdraft/w/test_ws/flow/u/me/observed')).toBe(wrapped(7))

		UserDraft.save('flow', 'u/me/observed', 9)
		expect(handle.draft).toBe(9)
		expect(storedShape('userdraft/w/test_ws/flow/u/me/observed')).toBe(wrapped(9))
	})

	it('get() returns a cloneable snapshot of live handle values', () => {
		const handle = UserDraft.use<{ path: string; nested: { value: number } }>('script', '')
		handle.draft = { path: 'u/me/live', nested: { value: 1 } }

		const draft = UserDraft.get<{ path: string; nested: { value: number } }>('script', '')
		expect(draft).toEqual({ path: 'u/me/live', nested: { value: 1 } })
		expect(draft).not.toBe(handle.draft)
		expect(() => structuredClone(draft)).not.toThrow()
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

	it('discard() clears LS, resets the handle to the fallback, and does NOT re-persist', () => {
		// Seed: handle holds a divergent local autosave.
		localStorage.setItem('userdraft/w/test_ws/flow/u/me/discard', wrapped('local-edit'))
		const handle = UserDraft.use<string>('flow', 'u/me/discard')
		expect(handle.draft).toBe('local-edit')

		// Reset to a known backend baseline.
		UserDraft.discard('flow', 'u/me/discard', 'backend-baseline')
		flushPersist()

		// In-memory handle reflects the fallback immediately.
		expect(handle.draft).toBe('backend-baseline')
		// LS is cleared and stays cleared — the fallback must NOT round-trip
		// back into storage (that would make the next reload "restore" the
		// fallback as if it were a real autosave).
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/discard')).toBeNull()
	})

	it('discard() with undefined fallback clears both LS and in-memory state', () => {
		localStorage.setItem('userdraft/w/test_ws/flow/u/me/wipe', wrapped('local-edit'))
		const handle = UserDraft.use<string>('flow', 'u/me/wipe')
		expect(handle.draft).toBe('local-edit')

		UserDraft.discard('flow', 'u/me/wipe', undefined)
		flushPersist()

		expect(handle.draft).toBeUndefined()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/wipe')).toBeNull()
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
		expect(storedShape('userdraft/w/test_ws/flow/u/me/setter')).toBe(wrapped('persisted'))
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
		expect(storedShape('userdraft/w/test_ws/flow/u/me/writeDefault')).toBe(wrapped('modified'))
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
		expect(storedShape('userdraft/w/test_ws/flow/')).toBe(wrapped(100))
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
		expect(storedShape('userdraft/w/test_ws/flow/')).toBe(wrapped(5))
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
		expect(storedShape('userdraft/w/test_ws/flow/u/me/atomic')).toBe(
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
		expect(storedShape('userdraft/w/test_ws/flow/u/me/setmeta')).toBe(
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
		expect(storedShape('userdraft/w/test_ws/flow/u/me/preserve')).toBe(
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

		expect(storedShape('userdraft/w/test_ws/flow/u/me/savepreserve')).toBe(
			JSON.stringify({ value: 'new', remoteRev: 5 })
		)
	})

	it('UserDraft.save persists immediately when a live handle exists', () => {
		const handle = UserDraft.use<string>('flow', 'u/me/live-save')

		UserDraft.save('flow', 'u/me/live-save', 'external')

		expect(handle.draft).toBe('external')
		expect(storedShape('userdraft/w/test_ws/flow/u/me/live-save')).toBe(wrapped('external'))
	})

	it('UserDraft.save preserves live rev metadata while forcing persistence', () => {
		const handle = UserDraft.use<string>('flow', 'u/me/live-save-meta')
		handle.setDraftAndMeta('baseline', { remoteRev: 5 })
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/live-save-meta')).toBeNull()

		UserDraft.save('flow', 'u/me/live-save-meta', 'external')

		expect(handle.draft).toBe('external')
		expect(storedShape('userdraft/w/test_ws/flow/u/me/live-save-meta')).toBe(
			JSON.stringify({ value: 'external', remoteRev: 5 })
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
		expect(storedShape('userdraft/w/test_ws/flow/u/me/forceack')).toBe(
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
		// External save() calls persist immediately, even with a live handle.
		expect(storedShape('userdraft/w/test_ws/flow/u/me/ref')).toBe(wrapped(2))

		// Releasing the second handle drops the entry; subsequent save()
		// must go straight to localStorage rather than mutating in-memory
		// state (which no longer exists).
		const secondCb = onDestroyCallbacks.shift()!
		secondCb()

		UserDraft.save('flow', 'u/me/ref', 3)
		// UserDraft.save without a live entry writes synchronously.
		expect(storedShape('userdraft/w/test_ws/flow/u/me/ref')).toBe(wrapped(3))
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
		expect(storedShape('userdraft/w/test_ws/flow/u/me/debounce')).toBe(wrapped('three'))
	})
})

describe('UserDraft.useMany()', () => {
	it('acquires one handle per spec in the synchronous initial reconcile', () => {
		// `useMany`'s sync reconcile populates handles[0..] before returning,
		// so callers (and `use()`'s 1-len wrapper) can use them immediately
		// without waiting for an `$effect` tick.
		const handles = UserDraft.useMany<number>(() => [
			{ itemKind: 'flow', path: 'u/me/many', workspace: 'a' },
			{ itemKind: 'flow', path: 'u/me/many', workspace: 'b' }
		])
		expect(handles.length).toBe(2)

		// Each spec gets its own entry in the workspace-keyed store.
		handles[0].draft = 0 // baseline
		handles[0].draft = 1 // persisted
		handles[1].draft = 0
		handles[1].draft = 9
		flushPersist()
		expect(storedShape('userdraft/w/a/flow/u/me/many')).toBe(wrapped(1))
		expect(storedShape('userdraft/w/b/flow/u/me/many')).toBe(wrapped(9))

		// One component-level onDestroy releases every acquired entry.
		expect(onDestroyCallbacks.length).toBe(1)
	})
})

describe('gcUserDrafts', () => {
	let gcUserDrafts: (maxAgeMs?: number) => void
	let USER_DRAFT_GC_MAX_AGE_MS: number
	const DAY = 24 * 60 * 60 * 1000

	beforeEach(async () => {
		;({ gcUserDrafts, USER_DRAFT_GC_MAX_AGE_MS } = await import('./userDraft.svelte'))
	})

	it('sweeps entries whose lastWrittenAt is older than the cutoff', () => {
		vi.setSystemTime(new Date('2026-06-01T00:00:00Z'))
		const old = Date.now() - 31 * DAY
		const fresh = Date.now() - 1 * DAY
		localStorage.setItem(
			'userdraft/w/test_ws/flow/u/me/old',
			JSON.stringify({ value: 1, lastWrittenAt: old })
		)
		localStorage.setItem(
			'userdraft/w/test_ws/flow/u/me/fresh',
			JSON.stringify({ value: 2, lastWrittenAt: fresh })
		)
		// Unrelated keys are left alone.
		localStorage.setItem('some_other_key', 'unrelated')

		gcUserDrafts()

		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/old')).toBeNull()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/fresh')).not.toBeNull()
		expect(localStorage.getItem('some_other_key')).toBe('unrelated')
	})

	it('backfills lastWrittenAt on entries lacking it, instead of sweeping them immediately', () => {
		// Pre-GC-feature entry (legacy migration output, or just an old entry
		// from earlier in this PR's lifecycle): no `lastWrittenAt`. First GC
		// pass should stamp it as "now" rather than wipe it on sight.
		localStorage.setItem('userdraft/w/test_ws/flow/u/me/legacy', JSON.stringify({ value: 'data' }))
		vi.setSystemTime(new Date('2026-06-01T00:00:00Z'))

		gcUserDrafts()

		const raw = localStorage.getItem('userdraft/w/test_ws/flow/u/me/legacy')
		expect(raw).not.toBeNull()
		const parsed = JSON.parse(raw!)
		expect(parsed.lastWrittenAt).toBe(Date.now())
		expect(parsed.value).toBe('data')
	})

	it('exposes a 30-day default retention window', () => {
		expect(USER_DRAFT_GC_MAX_AGE_MS).toBe(30 * 24 * 60 * 60 * 1000)
	})

	it('respects a custom maxAgeMs', () => {
		vi.setSystemTime(new Date('2026-06-01T00:00:00Z'))
		localStorage.setItem(
			'userdraft/w/test_ws/flow/u/me/two_hours_ago',
			JSON.stringify({ value: 1, lastWrittenAt: Date.now() - 2 * 60 * 60 * 1000 })
		)

		gcUserDrafts(60 * 60 * 1000) // 1h cutoff

		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/two_hours_ago')).toBeNull()
	})
})

describe('normalizeForCompare', () => {
	it('returns undefined for undefined input', () => {
		expect(normalizeForCompare(undefined)).toBeUndefined()
	})

	it('drops keys whose value is undefined (mirrors JSON.stringify persistence)', () => {
		const out = normalizeForCompare({ a: 1, b: undefined, c: { d: undefined, e: 2 } })
		expect(out).toEqual({ a: 1, c: { e: 2 } })
		expect(Object.keys(out as object)).not.toContain('b')
		expect(Object.keys((out as any).c)).not.toContain('d')
	})

	it('falls back to the original value when not serializable (cyclic)', () => {
		const cyclic: any = { a: 1 }
		cyclic.self = cyclic
		expect(normalizeForCompare(cyclic)).toBe(cyclic)
	})
})

describe('localDraftDiffers', () => {
	it('returns false when there is no local draft', () => {
		expect(localDraftDiffers(undefined, { a: 1 })).toBe(false)
		expect(localDraftDiffers(null, { a: 1 })).toBe(false)
	})

	it('treats a draft that round-trips equal to the config as NOT differing', () => {
		// The Schedule bug: getXCfg() emits conditionally-undefined keys, but
		// the persisted draft went through JSON.stringify which dropped them.
		const freshCfg = { path: 'u/me/s', schedule: '0 0 * * *', on_failure: undefined }
		const persisted = JSON.parse(JSON.stringify(freshCfg)) // { path, schedule }
		expect(localDraftDiffers(persisted, freshCfg)).toBe(false)
	})

	it('returns true for a genuine difference', () => {
		expect(localDraftDiffers({ a: 1 }, { a: 2 })).toBe(true)
		expect(localDraftDiffers({ a: 1, extra: 'x' }, { a: 1 })).toBe(true)
	})
})

describe('UserDraft.saveIfChanged', () => {
	const KEY = 'userdraft/w/test_ws/trigger_schedule/u/me/s'

	it('does not persist a draft equal to the deployed baseline', () => {
		const deployed = { path: 'u/me/s', schedule: '0 0 * * *', on_failure: undefined }
		// value is the post-load reactive cfg — same shape, undefined keys present
		UserDraft.saveIfChanged('trigger_schedule', 'u/me/s', { ...deployed }, deployed)
		expect(localStorage.getItem(KEY)).toBeNull()
	})

	it('treats a value that round-trips equal to deployed as unchanged', () => {
		const deployed = { path: 'u/me/s', schedule: '0 0 * * *', on_failure: undefined }
		const value = JSON.parse(JSON.stringify(deployed)) // { path, schedule }
		UserDraft.saveIfChanged('trigger_schedule', 'u/me/s', value, deployed)
		expect(localStorage.getItem(KEY)).toBeNull()
	})

	it('persists when the value differs from the deployed baseline', () => {
		const deployed = { path: 'u/me/s', schedule: '0 0 * * *' }
		const value = { path: 'u/me/s', schedule: '5 0 * * *' }
		UserDraft.saveIfChanged('trigger_schedule', 'u/me/s', value, deployed)
		expect(storedShape(KEY)).toBe(wrapped(value))
	})

	it('removes a pre-existing draft once the value reverts to deployed', () => {
		const deployed = { path: 'u/me/s', schedule: '0 0 * * *' }
		UserDraft.save('trigger_schedule', 'u/me/s', { path: 'u/me/s', schedule: '5 0 * * *' })
		expect(localStorage.getItem(KEY)).not.toBeNull()
		UserDraft.saveIfChanged('trigger_schedule', 'u/me/s', { ...deployed }, deployed)
		expect(localStorage.getItem(KEY)).toBeNull()
	})

	it('persists when there is no deployed baseline (undefined)', () => {
		const value = { path: 'u/me/s', schedule: '0 0 * * *' }
		UserDraft.saveIfChanged('trigger_schedule', 'u/me/s', value, undefined)
		expect(storedShape(KEY)).toBe(wrapped(value))
	})
})

describe('UserDraft.list / clear / setDraftAndMeta', () => {
	it('enumerates persisted-only drafts for the requested workspace and kinds', () => {
		UserDraft.setDraftAndMeta('script', 'f/a', { path: 'f/a', content: 'a' }, { remoteRev: 'h1' })
		UserDraft.setDraftAndMeta(
			'flow',
			'f/b',
			{ path: 'f/b', value: { modules: [] } },
			{ remoteRev: 2 },
			{ workspace: 'other_ws' }
		)
		UserDraft.setDraftAndMeta('resource', 'f/c', { path: 'f/c' }, {})

		expect(UserDraft.list({ itemKinds: ['script'] })).toEqual([
			{
				workspace: 'test_ws',
				itemKind: 'script',
				path: 'f/a',
				value: { path: 'f/a', content: 'a' },
				meta: { remoteRev: 'h1' },
				persisted: true,
				live: false
			}
		])
		expect(UserDraft.list({ workspace: 'other_ws' })).toEqual([
			expect.objectContaining({
				workspace: 'other_ws',
				itemKind: 'flow',
				path: 'f/b',
				persisted: true,
				live: false
			})
		])
	})

	it('keeps multiple path-addressed drafts and the empty-path scratch draft distinct', () => {
		UserDraft.setDraftAndMeta('script', '', { path: '', content: 'scratch' }, {})
		UserDraft.setDraftAndMeta('script', 'f/new-a', { path: 'f/new-a', content: 'a' }, {})
		UserDraft.setDraftAndMeta('script', 'f/new-b', { path: 'f/new-b', content: 'b' }, {})

		const entries = UserDraft.list<{ path: string; content: string }>({ itemKinds: ['script'] })

		expect(entries).toHaveLength(3)
		expect(entries).toEqual(
			expect.arrayContaining([
				expect.objectContaining({
					itemKind: 'script',
					path: '',
					value: { path: '', content: 'scratch' }
				}),
				expect.objectContaining({
					itemKind: 'script',
					path: 'f/new-a',
					value: { path: 'f/new-a', content: 'a' }
				}),
				expect.objectContaining({
					itemKind: 'script',
					path: 'f/new-b',
					value: { path: 'f/new-b', content: 'b' }
				})
			])
		)
	})

	it('enumerates live-only drafts before the debounce persists them', () => {
		const handle = UserDraft.use<{ path: string; content: string }>('script', 'f/live')
		handle.setDraftAndMeta({ path: 'f/live', content: 'live' }, { remoteRev: 'h1' })

		expect(localStorage.getItem('userdraft/w/test_ws/script/f/live')).toBeNull()
		expect(UserDraft.list({ itemKinds: ['script'] })).toEqual([
			{
				workspace: 'test_ws',
				itemKind: 'script',
				path: 'f/live',
				value: { path: 'f/live', content: 'live' },
				meta: { remoteRev: 'h1' },
				persisted: false,
				live: true
			}
		])
	})

	it('dedupes entries that are both persisted and live', () => {
		UserDraft.setDraftAndMeta(
			'script',
			'f/both',
			{ path: 'f/both', content: 'persisted' },
			{
				remoteRev: 'h1'
			}
		)
		const handle = UserDraft.use<{ path: string; content: string }>('script', 'f/both')
		handle.draft = { path: 'f/both', content: 'live' }

		expect(UserDraft.list({ itemKinds: ['script'] })).toEqual([
			{
				workspace: 'test_ws',
				itemKind: 'script',
				path: 'f/both',
				value: { path: 'f/both', content: 'live' },
				meta: { remoteRev: 'h1' },
				persisted: true,
				live: true
			}
		])
	})

	it('clear removes persisted storage and live state without re-persisting', () => {
		UserDraft.setDraftAndMeta(
			'script',
			'f/clear',
			{ path: 'f/clear', content: 'x' },
			{
				remoteRev: 'h1'
			}
		)
		const handle = UserDraft.use<{ path: string; content: string }>('script', 'f/clear')
		expect(handle.draft).toEqual({ path: 'f/clear', content: 'x' })

		UserDraft.clear('script', 'f/clear')
		flushPersist()

		expect(handle.draft).toBeUndefined()
		expect(localStorage.getItem('userdraft/w/test_ws/script/f/clear')).toBeNull()
		expect(UserDraft.list({ itemKinds: ['script'] })).toEqual([])
	})

	it('clear cancels pending debounced live writes', () => {
		const handle = UserDraft.use<{ path: string; content: string }>('script', 'f/pending-clear')
		handle.draft = { path: 'f/pending-clear', content: 'initial' }
		handle.draft = { path: 'f/pending-clear', content: 'pending' }

		UserDraft.clear('script', 'f/pending-clear')
		expect(handle.draft).toBeUndefined()
		expect(localStorage.getItem('userdraft/w/test_ws/script/f/pending-clear')).toBeNull()

		flushPersist()
		expect(localStorage.getItem('userdraft/w/test_ws/script/f/pending-clear')).toBeNull()
	})

	it('clear does not let an old debounced remove delete a later direct write', () => {
		const key = 'userdraft/w/test_ws/script/f/rewrite-after-clear'
		const handle = UserDraft.use<{ path: string; content: string }>(
			'script',
			'f/rewrite-after-clear'
		)
		handle.draft = { path: 'f/rewrite-after-clear', content: 'initial' }
		handle.draft = { path: 'f/rewrite-after-clear', content: 'pending' }

		UserDraft.clear('script', 'f/rewrite-after-clear')
		flushDestroyCallbacks()
		UserDraft.setDraftAndMeta(
			'script',
			'f/rewrite-after-clear',
			{ path: 'f/rewrite-after-clear', content: 'new' },
			{}
		)

		flushPersist()
		expect(storedShape(key)).toBe(wrapped({ path: 'f/rewrite-after-clear', content: 'new' }))
	})

	it('list hides persisted drafts when a live handle has cleared the value', () => {
		UserDraft.setDraftAndMeta(
			'script',
			'f/live-clear',
			{ path: 'f/live-clear', content: 'persisted' },
			{}
		)
		const handle = UserDraft.use<{ path: string; content: string }>('script', 'f/live-clear')
		handle.draft = { path: 'f/live-clear', content: 'edited' }
		handle.draft = undefined

		expect(localStorage.getItem('userdraft/w/test_ws/script/f/live-clear')).not.toBeNull()
		expect(UserDraft.list({ itemKinds: ['script'] })).toEqual([])
	})

	it('setDraftAndMeta updates live handles atomically and preserves metadata on later draft writes', () => {
		const handle = UserDraft.use<{ path: string; content: string }>('script', 'f/meta')

		UserDraft.setDraftAndMeta(
			'script',
			'f/meta',
			{ path: 'f/meta', content: 'first' },
			{
				remoteRev: 'h1',
				remoteDraftRev: 'd1'
			}
		)
		handle.draft = { path: 'f/meta', content: 'second' }

		expect(handle.draft).toEqual({ path: 'f/meta', content: 'second' })
		expect(handle.meta).toEqual({ remoteRev: 'h1', remoteDraftRev: 'd1' })
		expect(UserDraft.list({ itemKinds: ['script'] })[0]).toEqual(
			expect.objectContaining({
				value: { path: 'f/meta', content: 'second' },
				meta: { remoteRev: 'h1', remoteDraftRev: 'd1' }
			})
		)
	})

	it('static setDraftAndMeta persists first writes even when a live handle exists', () => {
		const handle = UserDraft.use<{ path: string; content: string }>('script', 'f/static-live')

		UserDraft.setDraftAndMeta(
			'script',
			'f/static-live',
			{ path: 'f/static-live', content: 'first' },
			{ remoteRev: 'h1' }
		)

		expect(handle.draft).toEqual({ path: 'f/static-live', content: 'first' })
		expect(storedShape('userdraft/w/test_ws/script/f/static-live')).toBe(
			JSON.stringify({
				value: { path: 'f/static-live', content: 'first' },
				remoteRev: 'h1'
			})
		)
	})

	it('lists live drafts with runtime-only values without throwing', () => {
		const handle = UserDraft.use<Record<string, unknown>>('script', 'f/runtime')
		handle.draft = {
			path: 'f/runtime',
			content: 'x',
			callback: () => 'not serializable'
		}

		expect(() => UserDraft.list({ itemKinds: ['script'] })).not.toThrow()
		expect(UserDraft.list({ itemKinds: ['script'] })).toEqual([
			expect.objectContaining({
				itemKind: 'script',
				path: 'f/runtime',
				value: { path: 'f/runtime', content: 'x' }
			})
		])
	})
})
