import { describe, it, expect, beforeEach, vi } from 'vitest'

const discard = vi.fn()
// Whether the key ends up settled on the delete after a flush — false stands for a
// form that keeps queueing writes behind it.
let settles = true
vi.mock('./gen', () => ({ DraftService: { updateDraft: vi.fn() } }))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))
vi.mock('./localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))
vi.mock('./userDraftDbSyncer.svelte', () => ({
	UserDraftDbSyncer: {
		save: vi.fn(),
		flush: vi.fn(async () => {}),
		lastLandedWasDelete: vi.fn(() => settles),
		getState: vi.fn(() => ({ state: 'none' }))
	}
}))

import { settleDraftAfterWrite, UserDraft } from './userDraft.svelte'

beforeEach(() => {
	settles = true
	discard.mockClear()
	vi.spyOn(UserDraft, 'discard').mockImplementation(discard as any)
})

const OPTS = { workspace: 'ws' }
const sent = { path: 'u/me/a', value: 'sent' }

/**
 * The forms stay editable while their save is in flight, so the cell can hold a
 * newer edit by the time the write returns. Resetting it unconditionally — what
 * every editor did — swallows that edit with no trace.
 */
describe('settleDraftAfterWrite', () => {
	it('resets the cell when it still holds what was written', () => {
		settleDraftAfterWrite('variable', sent, { ...sent }, 'u/me/a', 'u/me/a', OPTS)
		expect(discard).toHaveBeenCalledWith('variable', 'u/me/a', sent, OPTS)
	})

	it('keeps an edit made while the write was in flight', () => {
		settleDraftAfterWrite(
			'variable',
			sent,
			{ path: 'u/me/a', value: 'typed' },
			'u/me/a',
			'u/me/a',
			OPTS
		)
		expect(discard).not.toHaveBeenCalled()
	})

	// A renamed save is the exception: a freshly acquired cell reads only its own
	// default, so the edit cannot follow the item and the alternative to resetting
	// is an orphan draft under a path the item no longer occupies.
	it('resets even a diverged cell when the write moved the item', () => {
		settleDraftAfterWrite(
			'variable',
			sent,
			{ path: 'u/me/a', value: 'typed' },
			'u/me/a',
			'u/me/b',
			OPTS
		)
		expect(discard).toHaveBeenCalledWith('variable', 'u/me/a', sent, OPTS)
	})

	// The form stays editable across the delete's own request, so an edit made then
	// parks a write that would put the draft back — under a path the item has left,
	// once the callers re-key. The delete is re-sent while that is the case, and
	// bounded, because a form being typed into can always add one more.
	it('re-sends the delete while the key will not settle on it, and gives up bounded', async () => {
		settles = false
		await settleDraftAfterWrite('variable', sent, { ...sent }, 'u/me/a', 'u/me/b', OPTS)
		expect(discard).toHaveBeenCalledTimes(3)
	})

	it('sends it once when the key settles', async () => {
		await settleDraftAfterWrite('variable', sent, { ...sent }, 'u/me/a', 'u/me/b', OPTS)
		expect(discard).toHaveBeenCalledTimes(1)
	})

	it('compares through the draft normalization, so a nested edit is not missed', () => {
		const written = { path: 'u/me/a', args: { host: 'h' } }
		settleDraftAfterWrite(
			'resource',
			written,
			{ path: 'u/me/a', args: { host: 'h' } },
			'u/me/a',
			'u/me/a',
			OPTS
		)
		expect(discard).toHaveBeenCalledTimes(1)
		discard.mockClear()
		settleDraftAfterWrite(
			'resource',
			written,
			{ path: 'u/me/a', args: { host: 'edited' } },
			'u/me/a',
			'u/me/a',
			OPTS
		)
		expect(discard).not.toHaveBeenCalled()
	})
})
