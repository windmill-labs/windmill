import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'

// Service layer mocked: a "saved" response drives the syncer's confirmed-save
// path, which is what `onSaved` / `stripNewDraftFlagOnSave` hang off of.
const updateDraft = vi.fn(async (..._args: any[]) => ({
	status: 'saved' as const,
	current_timestamp: '2020-01-01T00:00:00Z'
}))

vi.mock('./gen', () => ({
	DraftService: { updateDraft: (...a: unknown[]) => updateDraft(...(a as [])) }
}))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))
// `getLocalDraftHint` gates the seed branch; drive it per-test via `hints.value`.
const hints = vi.hoisted(() => ({ value: undefined as boolean | undefined }))
vi.mock('./localDraftHints.svelte', () => ({
	setLocalDraftHint: vi.fn(),
	getLocalDraftHint: () => hints.value
}))

import { UserDraftDbSyncer } from './userDraftDbSyncer.svelte'
import { stripNewDraftFlagOnSave, shouldSeedNewDraft } from './newDraftFlag'

/** Minimal `window` stand-in: `history.replaceState` rewrites `location.href`,
 * mirroring what jsdom does, so the helper's strip is observable. */
function stubWindow(href: string): { current: () => string } {
	const win: any = {
		location: { href },
		history: {
			state: null as unknown,
			replaceState(state: unknown, _title: string, url: string) {
				this.state = state
				win.location.href = new URL(url, win.location.href).toString()
			}
		}
	}
	vi.stubGlobal('window', win)
	return { current: () => win.location.href }
}

afterEach(() => {
	vi.unstubAllGlobals()
	vi.clearAllMocks()
	hints.value = undefined
	updateDraft.mockResolvedValue({ status: 'saved', current_timestamp: '2020-01-01T00:00:00Z' })
})

describe('shouldSeedNewDraft', () => {
	const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/draft_a' }

	it('is false without the flag', () => {
		expect(shouldSeedNewDraft(new URLSearchParams(''), q.workspace, q.itemKind, q.path)).toBe(false)
	})

	it('is true for a fresh, never-persisted new draft', () => {
		hints.value = undefined
		expect(
			shouldSeedNewDraft(new URLSearchParams('new_draft=true'), q.workspace, q.itemKind, q.path)
		).toBe(true)
	})

	it('is false once the draft is persisted this session (stale flag → load, not re-seed)', () => {
		hints.value = true
		expect(
			shouldSeedNewDraft(new URLSearchParams('new_draft=true'), q.workspace, q.itemKind, q.path)
		).toBe(false)
	})
})

describe('UserDraftDbSyncer.onSaved', () => {
	it('fires after a confirmed non-delete save', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/draft_a' }
		const listener = vi.fn()
		UserDraftDbSyncer.onSaved(q, listener)
		await UserDraftDbSyncer.save({ ...q, value: { content: 'x' }, immediate: true })
		expect(listener).toHaveBeenCalledTimes(1)
	})

	it('does NOT fire on a delete (value: null) save', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/draft_b' }
		const listener = vi.fn()
		UserDraftDbSyncer.onSaved(q, listener)
		await UserDraftDbSyncer.save({ ...q, value: null, immediate: true })
		expect(listener).not.toHaveBeenCalled()
	})

	it('stops firing after unsubscribe', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/draft_c' }
		const listener = vi.fn()
		const unsub = UserDraftDbSyncer.onSaved(q, listener)
		unsub()
		await UserDraftDbSyncer.save({ ...q, value: { content: 'x' }, immediate: true })
		expect(listener).not.toHaveBeenCalled()
	})
})

describe('stripNewDraftFlagOnSave', () => {
	beforeEach(() => {
		stubWindow('http://localhost/scripts/edit/u/me/draft_d?new_draft=true&template=foo')
	})

	it('keeps ?new_draft until a save lands, then strips only that flag', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/draft_d' }
		stripNewDraftFlagOnSave(q)
		// Before any save the flag is untouched.
		expect(window.location.href).toContain('new_draft=true')
		await UserDraftDbSyncer.save({ ...q, value: { content: 'x' }, immediate: true })
		expect(window.location.href).not.toContain('new_draft')
		// Sibling seeding params are preserved.
		expect(window.location.href).toContain('template=foo')
	})

	it('does not strip on a delete save', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/draft_e' }
		stubWindow('http://localhost/scripts/edit/u/me/draft_e?new_draft=true')
		stripNewDraftFlagOnSave(q)
		await UserDraftDbSyncer.save({ ...q, value: null, immediate: true })
		expect(window.location.href).toContain('new_draft=true')
	})

	it('cleanup unsubscribes so a later save does not strip', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/draft_f' }
		stubWindow('http://localhost/scripts/edit/u/me/draft_f?new_draft=true')
		const cleanup = stripNewDraftFlagOnSave(q)
		cleanup()
		await UserDraftDbSyncer.save({ ...q, value: { content: 'x' }, immediate: true })
		expect(window.location.href).toContain('new_draft=true')
	})
})
