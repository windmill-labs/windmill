import { describe, it, expect, beforeEach, vi } from 'vitest'

// The sweep DELETES drafts, so the guard that matters is which rows it picks.
// Stub the two collaborators it decides from — the draft listing and the
// per-kind diff — and assert on what it discards.
const listDrafts = vi.fn()
const getDraftDiffValues = vi.fn()
const updateDraft = vi.fn(async () => ({ status: 'saved', current_timestamp: 'x' }))

vi.mock('./gen', () => ({
	DraftService: {
		listDrafts: (...a: unknown[]) => listDrafts(...(a as [])),
		updateDraft: (...a: unknown[]) => updateDraft(...(a as []))
	}
}))
// Only `getDraftDiffValues` is stubbed; `canDiffDraftKind` is the real one, so
// the kind filter is pinned against the actual overlay table.
vi.mock('./utils_draft_deploy', async (orig) => ({
	...(await orig<Record<string, unknown>>()),
	getDraftDiffValues: (...a: unknown[]) => getDraftDiffValues(...(a as []))
}))
vi.mock('./localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))
vi.mock('./workspaceDrafts.svelte', () => ({ invalidateWorkspaceDrafts: vi.fn() }))
const sendUserToast = vi.fn()
vi.mock('./toast', () => ({ sendUserToast: (...a: unknown[]) => sendUserToast(...(a as [])) }))

// The sweep reads exactly one thing from the syncer — whether this tab is
// mid-write on the key — and writes nothing back to it.
let syncState = 'none'
vi.mock('./userDraftDbSyncer.svelte', () => ({
	UserDraftDbSyncer: {
		getState: () => ({
			get state() {
				return syncState
			}
		})
	}
}))

let liveDraft = false
vi.mock('./userDraft.svelte', async (orig) => ({
	...(await orig<Record<string, unknown>>()),
	UserDraft: { has: () => liveDraft }
}))

import { pruneMeaninglessDrafts } from './userDraftPrune'

const row = (over: Record<string, unknown> = {}) => ({
	kind: 'resource',
	path: 'u/me/r',
	draft_only: false,
	legacy_draft: false,
	mine: true,
	can_write: true,
	created_at: '2026-01-01T00:00:00Z',
	...over
})
const diff = (over: Record<string, unknown> = {}) => ({
	deployed: { value: { host: 'h' } },
	draft: { value: { host: 'h' } },
	hasDraft: true,
	noDeployed: false,
	...over
})
const discardedPaths = () => updateDraft.mock.calls.map((c: any[]) => c[0].path as string)

beforeEach(() => {
	localStorage.clear()
	vi.clearAllMocks()
	updateDraft.mockResolvedValue({ status: 'saved', current_timestamp: 'x' })
	syncState = 'none'
	liveDraft = false
})

describe('pruneMeaninglessDrafts', () => {
	it('discards a draft whose diff against the deployed value is empty', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardedPaths()).toEqual(['u/me/r'])
	})

	it('keeps a draft that carries a real change', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff({ draft: { value: { host: 'other' } } }))
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('never touches a draft-only item — the draft is the whole item', async () => {
		listDrafts.mockResolvedValue([row({ draft_only: true })])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('never touches another user’s row, or one it cannot write', async () => {
		listDrafts.mockResolvedValue([row({ mine: false }), row({ path: 'u/me/b', can_write: false })])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('leaves a draft alone when its diff cannot be fetched, and retries it later', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockRejectedValue(new Error('boom'))
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).not.toHaveBeenCalled()
		// A row that could not be judged is not a row that carries changes, so
		// the pass must stay open rather than strand it.
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardedPaths()).toEqual(['u/me/r'])
	})

	it('conditions the delete on the timestamp it judged, so a row that moved is spared', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		updateDraft.mockResolvedValue({ status: 'conflict', current_timestamp: 'newer' })
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).toHaveBeenCalledWith({
			workspace: 'main',
			kind: 'resource',
			path: 'u/me/r',
			requestBody: { value: null, last_sync: '2026-01-01T00:00:00Z', force: false }
		})
		// Refused, so nothing is reported as cleared — and the sweep leaves no
		// state behind for the editor's own autosave to trip over.
		expect(sendUserToast).not.toHaveBeenCalled()
	})

	it('does not keep retrying a row the server will never judge', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockRejectedValue({ status: 404 })
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).not.toHaveBeenCalled()
		// Sealed: a 4xx is final, unlike the transient case above.
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('gives up after a bounded number of unresolved passes', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockRejectedValue(new Error('network'))
		for (let i = 0; i < 3; i++) await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(listDrafts).toHaveBeenCalledTimes(3)
		// Sealed on the third: an unresolvable row cannot re-list forever.
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(listDrafts).toHaveBeenCalledTimes(3)
	})

	it('skips a kind no diff can be computed for, and still seals', async () => {
		listDrafts.mockResolvedValue([row({ kind: 'trigger_webhook', path: 'u/me/hook' })])
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(getDraftDiffValues).not.toHaveBeenCalled()
		// Unjudgeable is permanent, not transient: leaving the pass open would
		// re-run the sweep on every page load forever.
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('leaves alone a draft this tab is editing', async () => {
		liveDraft = true
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('leaves alone a draft with a write queued or in flight', async () => {
		syncState = 'pending'
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('does not count, or seal the pass on, a delete that failed to send', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		updateDraft.mockRejectedValueOnce(new Error('network'))
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(sendUserToast).not.toHaveBeenCalled()
		// The pass stayed open, so the draft left behind is retried.
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardedPaths()).toEqual(['u/me/r', 'u/me/r'])
	})

	it('never touches a legacy workspace-level row', async () => {
		listDrafts.mockResolvedValue([row({ legacy_draft: true })])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('only sweeps the kinds whose editors are gated', async () => {
		listDrafts.mockResolvedValue([
			row({ kind: 'script', path: 'u/me/s' }),
			row({ kind: 'flow', path: 'u/me/f' }),
			row({ kind: 'app', path: 'u/me/a' }),
			row({ kind: 'trigger_schedule', path: 'u/me/sched' }),
			row()
		])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardedPaths().sort()).toEqual(['u/me/r', 'u/me/sched'])
		// The expensive payload fetches are never made for the ungated kinds.
		expect(getDraftDiffValues).toHaveBeenCalledTimes(2)
	})

	it('leaves the pass open when a row was skipped as busy', async () => {
		liveDraft = true
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		liveDraft = false
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardedPaths()).toEqual(['u/me/r'])
	})

	it('runs once per workspace and user', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(updateDraft).toHaveBeenCalledTimes(1)
		await pruneMeaninglessDrafts('other', 'me@x.dev')
		expect(updateDraft).toHaveBeenCalledTimes(2)
	})

	it('retries next mount when the listing failed', async () => {
		listDrafts.mockRejectedValueOnce(new Error('offline'))
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardedPaths()).toEqual(['u/me/r'])
	})
})
