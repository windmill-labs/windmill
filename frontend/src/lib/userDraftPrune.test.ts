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
vi.mock('./toast', () => ({ sendUserToast: vi.fn() }))
vi.mock('./userDraftDbSyncer.svelte', () => ({ UserDraftDbSyncer: { save: vi.fn() } }))

import { pruneMeaninglessDrafts } from './userDraftPrune'

const row = (over: Record<string, unknown> = {}) => ({
	kind: 'resource',
	path: 'u/me/r',
	draft_only: false,
	legacy_draft: false,
	mine: true,
	can_write: true,
	created_at: '',
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

	it('ignores the empty fields a moved-on schema added', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockResolvedValue(
			diff({ draft: { value: { host: 'h', port: '', ssl: false, tags: [] } } })
		)
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardedPaths()).toEqual(['u/me/r'])
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

	it('leaves a draft alone when its diff cannot be fetched', async () => {
		listDrafts.mockResolvedValue([row()])
		getDraftDiffValues.mockRejectedValue(new Error('boom'))
		await pruneMeaninglessDrafts('main', 'me@x.dev')
		expect(discardDraft).not.toHaveBeenCalled()
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
