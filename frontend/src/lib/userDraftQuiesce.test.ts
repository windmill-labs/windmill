import { describe, it, expect, afterEach, vi } from 'vitest'

// Mocked so a test can hold a POST in flight: that window is the whole point of `quiesce`,
// which exists to wait one out rather than return while it is still going.
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

function deferred<T = void>() {
	let resolve!: (v: T) => void
	const promise = new Promise<T>((res) => (resolve = res))
	return { promise, resolve }
}

afterEach(() => {
	vi.clearAllMocks()
	updateDraft.mockResolvedValue({ status: 'saved', current_timestamp: '2020-01-01T00:00:00Z' })
})

/**
 * Resolving a draft conflict installs a fresh baseline. Anything of the old version still in the
 * air when that lands would be made acceptable by it, and a rejected one re-raises the conflict
 * just resolved — so the resolution waits for the key to go quiet first. What it must NOT do is
 * discard the payload while waiting: a refused save keeps it here, and until the resolution
 * commits it is the only copy of the edit outside the editor's own memory.
 */
describe('UserDraftDbSyncer.quiesce', () => {
	it('waits for a save already in flight', async () => {
		const q = { workspace: 'w', itemKind: 'variable' as const, path: 'u/me/quiesce_a' }
		const inFlight = deferred()
		let settledBeforeQuiesceReturned = false
		updateDraft.mockImplementationOnce(async () => {
			await inFlight.promise
			settledBeforeQuiesceReturned = true
			return { status: 'saved', current_timestamp: '2020-01-01T00:00:00Z' }
		})

		// In flight, not awaited: the POST is held open by the mock above.
		const saving = UserDraftDbSyncer.save({ ...q, value: { v: 'in flight' }, immediate: true })

		let quiesced = false
		const quiescing = UserDraftDbSyncer.quiesce(q).then(() => (quiesced = true))
		// A full macrotask, not a microtask: anything that merely yields would have resolved by
		// now, so this is what tells "waited for the POST" apart from "waited for nothing".
		await new Promise((r) => setTimeout(r, 0))
		expect(quiesced).toBe(false)

		inFlight.resolve()
		await quiescing
		await saving
		expect(settledBeforeQuiesceReturned).toBe(true)
	})

	it('keeps a refused payload recoverable, until the resolution that replaces it says otherwise', async () => {
		const q = { workspace: 'w', itemKind: 'variable' as const, path: 'u/me/quiesce_b' }
		// Refused, so the payload stays parked — the state a conflict resolution starts from.
		updateDraft.mockResolvedValueOnce({
			status: 'conflict',
			current_timestamp: '2020-01-02T00:00:00Z'
		})
		await UserDraftDbSyncer.save({ ...q, value: { v: 'refused' }, immediate: true })
		expect(UserDraftDbSyncer.getConflict(q).conflict).toBeTruthy()

		await UserDraftDbSyncer.quiesce(q)

		// A resolution abandoned here (the editor closed while quiescing) must leave the edit
		// somewhere it can still be sent. Flushing is how that payload is observed, since the
		// parked opts themselves are private to the syncer.
		updateDraft.mockClear()
		await UserDraftDbSyncer.flush(q)
		expect(updateDraft).toHaveBeenCalledTimes(1)
		expect(updateDraft.mock.calls[0][0]).toMatchObject({ requestBody: { value: { v: 'refused' } } })

		// Only the caller that has something to put in its place drops it.
		UserDraftDbSyncer.dropPending(q)
		updateDraft.mockClear()
		await UserDraftDbSyncer.flush(q)
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('cancels a debounced autosave so it cannot displace the write that follows', async () => {
		const q = { workspace: 'w', itemKind: 'variable' as const, path: 'u/me/quiesce_c' }
		// Debounced rather than immediate: this is the autosave a resolution has to call off, or
		// it fires mid-resolution and, being conditional, is refused.
		void UserDraftDbSyncer.save({ ...q, value: { v: 'queued' } })

		await UserDraftDbSyncer.quiesce(q)
		updateDraft.mockClear()

		// Long enough for the debounce to have fired had it survived.
		await new Promise((r) => setTimeout(r, 50))
		expect(updateDraft).not.toHaveBeenCalled()
	})
})
