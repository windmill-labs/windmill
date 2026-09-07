import { describe, it, expect, beforeEach, vi } from 'vitest'

const discard = vi.fn()
const { toast } = vi.hoisted(() => ({ toast: vi.fn() }))
vi.mock('./utils', async (orig) => ({ ...((await orig()) as object), sendUserToast: toast }))
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
	toast.mockClear()
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

	// A rename suspends the old key first, so nothing can queue behind the delete and
	// re-sending it could only repeat a request the server already refused. What is
	// left is a draft under a path the item has left, which has to be reported.
	it('sends the delete once on a rename and reports one that will not land', async () => {
		settles = false
		await settleDraftAfterWrite('variable', sent, { ...sent }, 'u/me/a', 'u/me/b', OPTS)
		expect(discard).toHaveBeenCalledTimes(1)
		expect(toast).toHaveBeenCalledTimes(1)
	})

	// A create's form cell is the detached handle an empty path gets, wired to no
	// key: every request made for one is addressed to `/drafts/update/<kind>/`.
	it('settles nothing for a create', async () => {
		await settleDraftAfterWrite('variable', sent, undefined, '', 'u/me/a', OPTS)
		expect(discard).not.toHaveBeenCalled()
	})

	// A released handle reads as no cell at all. That is not somebody's newer edit —
	// it is a draft nothing is watching — so the cleanup still has to run.
	it('cleans up when the editor has been released', async () => {
		await settleDraftAfterWrite('variable', sent, undefined, 'u/me/a', 'u/me/a', OPTS)
		expect(discard).toHaveBeenCalledWith('variable', 'u/me/a', sent, OPTS)
	})

	// On an unchanged path an edit made during the delete is the user's: the key is
	// left accepting writes, and a draft standing there is the intended outcome
	// rather than something to report.
	it('leaves an edit made during the delete on an unchanged path', async () => {
		settles = false
		await settleDraftAfterWrite('variable', sent, { ...sent }, 'u/me/a', 'u/me/a', OPTS)
		expect(discard).toHaveBeenCalledTimes(1)
		expect(toast).not.toHaveBeenCalled()
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
