import { describe, it, expect, beforeEach, vi } from 'vitest'

const discard = vi.fn()
vi.mock('./gen', () => ({ DraftService: { updateDraft: vi.fn() } }))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))
vi.mock('./localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))

import { settleDraftAfterWrite, UserDraft } from './userDraft.svelte'

beforeEach(() => {
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
