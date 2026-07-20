import { describe, it, expect, afterEach, vi } from 'vitest'

const updateDraft = vi.fn(async (..._args: any[]) => ({
	status: 'saved' as const,
	current_timestamp: '2020-01-01T00:00:00Z'
}))

vi.mock('./gen', () => ({
	DraftService: { updateDraft: (...a: unknown[]) => updateDraft(...(a as [])) }
}))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))
vi.mock('./localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))

import { UserDraftDbSyncer } from './userDraftDbSyncer.svelte'

afterEach(() => {
	vi.clearAllMocks()
	UserDraftDbSyncer.autosaveEnabled = true
})

/**
 * Read-only consumers (the chat `diff` tool) flush with `honorAutosaveToggle`
 * and then read `hasUnsavedDisabledChanges` to know the persisted state is
 * stale. Pins the contract pair: a toggle-honoring flush must NOT persist
 * auto-save-off edits (and must keep reporting them), while an explicit flush
 * persists them and clears the signal.
 */
describe('UserDraftDbSyncer toggle-honoring flush', () => {
	it('keeps auto-save-off edits parked and reported; explicit flush clears them', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/toggle_off' }
		UserDraftDbSyncer.autosaveEnabled = false
		await UserDraftDbSyncer.save({ ...q, value: { content: 'x' }, auto: true, canBeDisabled: true })

		await UserDraftDbSyncer.flush(q, { honorAutosaveToggle: true })
		expect(updateDraft).not.toHaveBeenCalled()
		expect(UserDraftDbSyncer.hasUnsavedDisabledChanges(q)).toBe(true)

		await UserDraftDbSyncer.flush(q)
		expect(updateDraft).toHaveBeenCalledTimes(1)
		expect(UserDraftDbSyncer.hasUnsavedDisabledChanges(q)).toBe(false)
	})
})

/** The diff snapshot cache invalidates through this hook — it must fire for
 * upserts AND deletes, the moment the write lands. */
describe('UserDraftDbSyncer.onAnySaved', () => {
	it('fires for landed upserts and deletes, and unsubscribes cleanly', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/hooked' }
		const events: string[] = []
		const off = UserDraftDbSyncer.onAnySaved((e) => events.push(`${e.itemKind}:${e.path}`))

		await UserDraftDbSyncer.save({ ...q, value: { content: 'x' }, immediate: true })
		await UserDraftDbSyncer.save({ ...q, value: null, immediate: true })
		expect(events).toEqual(['script:u/me/hooked', 'script:u/me/hooked'])

		off()
		await UserDraftDbSyncer.save({ ...q, value: { content: 'y' }, immediate: true })
		expect(events).toHaveLength(2)
	})
})
