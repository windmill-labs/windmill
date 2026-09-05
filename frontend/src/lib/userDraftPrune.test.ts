import { describe, it, expect, beforeEach, vi } from 'vitest'

// The sweep DELETES drafts, so the guard that matters is which rows it picks.
// Stub the two collaborators it decides from — the draft listing and the
// per-kind diff — and assert on what it discards.
const listDrafts = vi.fn()
const getDraftDiffValues = vi.fn()
const discardDraft = vi.fn(async () => ({ success: true }))

vi.mock('./gen', () => ({
	DraftService: { listDrafts: (...a: unknown[]) => listDrafts(...(a as [])) }
}))
vi.mock('./utils_draft_deploy', () => ({
	getDraftDiffValues: (...a: unknown[]) => getDraftDiffValues(...(a as [])),
	discardDraft: (...a: unknown[]) => discardDraft(...(a as []))
}))
vi.mock('./workspaceDrafts.svelte', () => ({ invalidateWorkspaceDrafts: vi.fn() }))
const sendUserToast = vi.fn()
vi.mock('./toast', () => ({ sendUserToast: (...a: unknown[]) => sendUserToast(...(a as [])) }))

let syncState = 'none'
let conflict: unknown = undefined
const recordRemoteSync = vi.fn()
vi.mock('./userDraftDbSyncer.svelte', () => ({
	UserDraftDbSyncer: {
		save: vi.fn(),
		recordRemoteSync: (...a: unknown[]) => recordRemoteSync(...(a as [])),
		getState: () => ({
			get state() {
				return syncState
			}
		}),
		getConflict: () => ({
			get conflict() {
				return conflict
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
const discardedPaths = () => discardDraft.mock.calls.map((c: any[]) => c[1])

beforeEach(() => {
	localStorage.clear()
	vi.clearAllMocks()
	discardDraft.mockResolvedValue({ success: true })
	syncState = 'none'
	conflict = undefined
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
		expect(discardDraft).not.toHaveBeenCalled()
	})

	it('never touches a draft-only item — the draft is the whole item', async () => {
		listDrafts.mockResolvedValue([row({ draft_only: true })])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardDraft).not.toHaveBeenCalled()
	})

	it('never touches another user’s row, or one it cannot write', async () => {
		listDrafts.mockResolvedValue([row({ mine: false }), row({ path: 'u/me/b', can_write: false })])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardDraft).not.toHaveBeenCalled()
	})

	it('leaves a draft alone when its diff cannot be fetched, and retries it later', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockRejectedValue(new Error('boom'))
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardDraft).not.toHaveBeenCalled()
		// A row that could not be judged is not a row that carries changes, so
		// the pass must stay open rather than strand it.
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardedPaths()).toEqual(['u/me/r'])
	})

	it('conditions the delete on the timestamp it judged, so a row that moved is spared', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		conflict = { serverTimestamp: '2026-01-02T00:00:00Z', localLastSync: null }
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(recordRemoteSync).toHaveBeenCalledWith(
			{ workspace: 'main', itemKind: 'resource', path: 'u/me/r' },
			'2026-01-01T00:00:00Z'
		)
		// The discard was attempted but refused, so nothing is reported as cleared.
		expect(sendUserToast).not.toHaveBeenCalled()
	})

	it('leaves alone a draft this tab is editing', async () => {
		liveDraft = true
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardDraft).not.toHaveBeenCalled()
	})

	it('leaves alone a draft with a write queued or in flight', async () => {
		syncState = 'pending'
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardDraft).not.toHaveBeenCalled()
	})

	it('does not count, or seal the pass on, a delete the syncer failed to send', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(diff())
		// The discard's own POST is what fails, so the state flips during it.
		discardDraft.mockImplementation(async () => {
			syncState = 'failed'
			return { success: true }
		})
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(sendUserToast).not.toHaveBeenCalled()
		discardDraft.mockResolvedValue({ success: true })
		// A same-session retry sees the key as busy (a failed save leaves the
		// payload parked), attempts nothing — and must still not seal the pass.
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardedPaths()).toEqual(['u/me/r'])
		// Once the failure clears, the draft left behind is finally retried.
		syncState = 'none'
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardedPaths()).toEqual(['u/me/r', 'u/me/r'])
	})

	it('never touches a legacy workspace-level row', async () => {
		listDrafts.mockResolvedValue([row({ legacy_draft: true })])
		getDraftDiffValues.mockResolvedValue(diff())
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardDraft).not.toHaveBeenCalled()
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
		expect(discardDraft).toHaveBeenCalledTimes(1)
		await pruneMeaninglessDrafts('other', 'me@x.dev')
		expect(discardDraft).toHaveBeenCalledTimes(2)
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
