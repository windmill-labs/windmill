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
 * just resolved — so the resolution waits for the key to go quiet first.
 */
describe('UserDraftDbSyncer.quiesce', () => {
	it('waits for a save already in flight, and drops what it parks', async () => {
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

		// Whatever that save left parked belonged to the version being replaced, so a later flush
		// has nothing to send — the observable form of "dropped", since the parked payload itself
		// is private to the syncer.
		updateDraft.mockClear()
		await UserDraftDbSyncer.flush(q)
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('leaves nothing queued for a later flush to send', async () => {
		const q = { workspace: 'w', itemKind: 'variable' as const, path: 'u/me/quiesce_b' }
		// Debounced rather than immediate: this is the autosave a resolution has to call off.
		void UserDraftDbSyncer.save({ ...q, value: { v: 'queued' } })

		await UserDraftDbSyncer.quiesce(q)
		updateDraft.mockClear()

		// A flush after quiescing has nothing to send: the queued payload is gone, not merely
		// deferred, so it cannot land on top of the version the user chose.
		await UserDraftDbSyncer.flush(q)
		expect(updateDraft).not.toHaveBeenCalled()
	})
})
