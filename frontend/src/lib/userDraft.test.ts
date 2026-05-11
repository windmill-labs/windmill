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

beforeEach(() => {
	__resetUserDraftForTesting()
	onDestroyCallbacks.length = 0
	localStorage.clear()
	workspaceStore.set('test_ws')
})

describe('UserDraft.save / get / remove (no observers)', () => {
	it('save writes to localStorage under the workspace-scoped key', () => {
		UserDraft.save('flow', 'u/me/myflow', { value: { hello: 'world' } })

		const raw = localStorage.getItem('userdraft/w/test_ws/flow/u/me/myflow')
		expect(raw).toBe(JSON.stringify({ value: { hello: 'world' } }))
	})

	it('get reads from localStorage when no observer is registered', () => {
		localStorage.setItem(
			'userdraft/w/test_ws/script/u/me/script1',
			JSON.stringify({ value: 'code' })
		)

		expect(UserDraft.get('script', 'u/me/script1')).toEqual({ value: 'code' })
	})

	it('get returns undefined when nothing is stored', () => {
		expect(UserDraft.get('flow', 'u/me/missing')).toBeUndefined()
	})

	it('get returns undefined when stored value is malformed', () => {
		localStorage.setItem('userdraft/w/test_ws/flow/u/me/bad', 'not-json')
		expect(UserDraft.get('flow', 'u/me/bad')).toBeUndefined()
	})

	it('remove clears the localStorage entry', () => {
		UserDraft.save('app', 'u/me/app1', { value: { grid: [] } })
		expect(localStorage.getItem('userdraft/w/test_ws/app/u/me/app1')).not.toBeNull()

		UserDraft.remove('app', 'u/me/app1')
		expect(localStorage.getItem('userdraft/w/test_ws/app/u/me/app1')).toBeNull()
	})

	it('uses the workspace from opts when provided', () => {
		UserDraft.save('flow', 'u/me/f', { value: 1 }, { workspace: 'other_ws' })

		expect(localStorage.getItem('userdraft/w/other_ws/flow/u/me/f')).toBe(
			JSON.stringify({ value: 1 })
		)
		// Default workspace key must remain empty.
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/f')).toBeNull()
	})

	it('supports trigger kinds as item kinds', () => {
		UserDraft.save('schedule_kafka', 'u/me/topic1', { value: { brokers: ['localhost:9092'] } })

		const raw = localStorage.getItem('userdraft/w/test_ws/schedule_kafka/u/me/topic1')
		expect(raw).toBe(JSON.stringify({ value: { brokers: ['localhost:9092'] } }))
	})

	it('throws when neither opts.workspace nor $workspaceStore is set', () => {
		workspaceStore.set(undefined)
		expect(() => UserDraft.save('flow', 'u/me/x', { value: 1 })).toThrow(/no workspace/)
	})
})

describe('UserDraft.use() — observer sync', () => {
	it('loads the existing localStorage value on first use', () => {
		localStorage.setItem(
			'userdraft/w/test_ws/flow/u/me/loaded',
			JSON.stringify({ value: 'preloaded' })
		)

		const handle = UserDraft.use<{ value: string }>('flow', 'u/me/loaded')
		expect(handle.draft).toEqual({ value: 'preloaded' })
	})

	it('two handles on the same key share the same underlying state', () => {
		const a = UserDraft.use<{ value: number }>('flow', 'u/me/shared')
		const b = UserDraft.use<{ value: number }>('flow', 'u/me/shared')

		a.draft = { value: 42 }
		expect(b.draft).toEqual({ value: 42 })

		b.draft = { value: 99 }
		expect(a.draft).toEqual({ value: 99 })
	})

	it('save() propagates to live use() handles', () => {
		const handle = UserDraft.use<{ value: number }>('flow', 'u/me/observed')
		expect(handle.draft).toBeUndefined()

		UserDraft.save('flow', 'u/me/observed', { value: 7 })
		expect(handle.draft).toEqual({ value: 7 })

		// And the underlying localStorage entry is updated too.
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/observed')).toBe(
			JSON.stringify({ value: 7 })
		)
	})

	it('remove() propagates to live use() handles and clears localStorage', () => {
		UserDraft.save('flow', 'u/me/removed', { value: 1 })
		const handle = UserDraft.use<{ value: number }>('flow', 'u/me/removed')
		expect(handle.draft).toEqual({ value: 1 })

		UserDraft.remove('flow', 'u/me/removed')
		expect(handle.draft).toBeUndefined()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/removed')).toBeNull()
	})

	it('writes through the handle setter persist to localStorage', () => {
		const handle = UserDraft.use<{ value: string }>('flow', 'u/me/setter')
		handle.draft = { value: 'persisted' }

		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/setter')).toBe(
			JSON.stringify({ value: 'persisted' })
		)
	})

	it('setting handle.draft = undefined removes the localStorage entry', () => {
		const handle = UserDraft.use<{ value: string }>('flow', 'u/me/clear')
		handle.draft = { value: 'temp' }
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/clear')).not.toBeNull()

		handle.draft = undefined
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/clear')).toBeNull()
		expect(handle.draft).toBeUndefined()
	})

	it('two handles in different workspaces are isolated', () => {
		const a = UserDraft.use<{ value: number }>('flow', 'u/me/iso', { workspace: 'ws_a' })
		const b = UserDraft.use<{ value: number }>('flow', 'u/me/iso', { workspace: 'ws_b' })

		a.draft = { value: 1 }
		b.draft = { value: 2 }

		expect(a.draft).toEqual({ value: 1 })
		expect(b.draft).toEqual({ value: 2 })
	})

	it('save() falls back to localStorage when no handle is registered', () => {
		UserDraft.save('flow', 'u/me/noobs', { value: 'fallback' })
		// First use() afterwards loads the persisted value.
		const handle = UserDraft.use<{ value: string }>('flow', 'u/me/noobs')
		expect(handle.draft).toEqual({ value: 'fallback' })
	})
})

describe('UserDraft.use() — defaultValue', () => {
	it('returns defaultValue when localStorage has no entry', () => {
		const handle = UserDraft.use<{ value: string }>('flow', 'u/me/withdefault', {
			defaultValue: { value: 'fallback' }
		})

		expect(handle.draft).toEqual({ value: 'fallback' })
	})

	it('does not persist the defaultValue on first read', () => {
		UserDraft.use<{ value: string }>('flow', 'u/me/lazyDefault', {
			defaultValue: { value: 'fallback' }
		})

		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/lazyDefault')).toBeNull()
	})

	it('localStorage value wins over defaultValue', () => {
		localStorage.setItem(
			'userdraft/w/test_ws/flow/u/me/overridden',
			JSON.stringify({ value: 'persisted' })
		)

		const handle = UserDraft.use<{ value: string }>('flow', 'u/me/overridden', {
			defaultValue: { value: 'fallback' }
		})

		expect(handle.draft).toEqual({ value: 'persisted' })
	})

	it('a write through the setter persists even though defaultValue was set', () => {
		const handle = UserDraft.use<{ value: string }>('flow', 'u/me/writeDefault', {
			defaultValue: { value: 'fallback' }
		})

		handle.draft = { value: 'modified' }
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/writeDefault')).toBe(
			JSON.stringify({ value: 'modified' })
		)
	})
})

describe('UserDraft — empty path (new-item, in-memory only)', () => {
	it('use() with empty path uses defaultValue and does not persist', () => {
		const handle = UserDraft.use<{ value: number }>('flow', '', {
			defaultValue: { value: 0 }
		})

		handle.draft = { value: 99 }

		expect(handle.draft).toEqual({ value: 99 })
		// No localStorage key with empty path should ever be written.
		expect(localStorage.getItem('userdraft/w/test_ws/flow/')).toBeNull()
	})

	it('two handles with empty path share in-memory state per workspace', () => {
		const a = UserDraft.use<{ value: number }>('flow', '')
		const b = UserDraft.use<{ value: number }>('flow', '')

		a.draft = { value: 1 }
		expect(b.draft).toEqual({ value: 1 })

		b.draft = { value: 2 }
		expect(a.draft).toEqual({ value: 2 })
	})

	it('save() with empty path is a no-op against localStorage but updates live handles', () => {
		const handle = UserDraft.use<{ value: number }>('flow', '')

		UserDraft.save('flow', '', { value: 5 })
		expect(handle.draft).toEqual({ value: 5 })
		expect(localStorage.getItem('userdraft/w/test_ws/flow/')).toBeNull()
	})

	it('get() with empty path returns the in-memory value when a handle is live, else undefined', () => {
		expect(UserDraft.get('flow', '')).toBeUndefined()

		const handle = UserDraft.use<{ value: number }>('flow', '')
		handle.draft = { value: 11 }
		expect(UserDraft.get('flow', '')).toEqual({ value: 11 })
	})

	it('remove() with empty path is a no-op against localStorage but clears live handles', () => {
		const handle = UserDraft.use<{ value: number }>('flow', '')
		handle.draft = { value: 1 }

		UserDraft.remove('flow', '')
		expect(handle.draft).toBeUndefined()
		expect(localStorage.getItem('userdraft/w/test_ws/flow/')).toBeNull()
	})
})

describe('UserDraft.use() — reference counting & cleanup', () => {
	it('destroys the entry when the last handle is released', () => {
		// First handle acquires the entry.
		const a = UserDraft.use<{ value: number }>('flow', 'u/me/ref')
		a.draft = { value: 1 }

		// Second handle increments the count.
		const b = UserDraft.use<{ value: number }>('flow', 'u/me/ref')
		expect(b.draft).toEqual({ value: 1 })

		// onDestroy for both handles got registered.
		expect(onDestroyCallbacks.length).toBe(2)

		// Releasing one handle keeps the entry alive — save() still updates handle a.
		const firstCb = onDestroyCallbacks.shift()!
		firstCb()

		UserDraft.save('flow', 'u/me/ref', { value: 2 })
		expect(a.draft).toEqual({ value: 2 })

		// Releasing the second handle drops the entry; subsequent save()
		// must go straight to localStorage rather than mutating in-memory
		// state (which no longer exists).
		const secondCb = onDestroyCallbacks.shift()!
		secondCb()

		UserDraft.save('flow', 'u/me/ref', { value: 3 })
		expect(localStorage.getItem('userdraft/w/test_ws/flow/u/me/ref')).toBe(
			JSON.stringify({ value: 3 })
		)
	})

	it('a fresh use() after cleanup re-reads the latest persisted value', () => {
		const a = UserDraft.use<{ value: string }>('flow', 'u/me/cycle')
		a.draft = { value: 'first' }
		flushDestroyCallbacks()

		// After all handles release, a brand-new use() must pick up the
		// value persisted to localStorage from the previous round.
		const b = UserDraft.use<{ value: string }>('flow', 'u/me/cycle')
		expect(b.draft).toEqual({ value: 'first' })
	})
})
