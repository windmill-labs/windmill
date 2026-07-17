import { describe, it, expect, afterEach, vi } from 'vitest'

// Mocked so a test can hold a POST in flight — that window is what makes a
// queued save displaceable.
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
 * Deploying queues several saves for one draft key back-to-back (mirror write,
 * post-deploy delete, unmount flush), so the runner displaces one of them. A
 * displaced save must read as "superseded", never as a failure.
 */
describe('UserDraftDbSyncer immediate save displacement', () => {
	it('resolves a displaced immediate save once the superseding save lands', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/displaced_a' }
		const inFlight = deferred()
		updateDraft.mockImplementationOnce(async () => {
			await inFlight.promise
			return { status: 'saved', current_timestamp: '2020-01-01T00:00:00Z' }
		})

		const first = UserDraftDbSyncer.save({ ...q, value: { content: '1' }, immediate: true })
		// Queues behind `first`, then gets displaced by the delete below.
		const displaced = UserDraftDbSyncer.save({ ...q, value: { content: '2' }, immediate: true })
		const deleting = UserDraftDbSyncer.save({ ...q, value: null, immediate: true })

		inFlight.resolve()
		await expect(displaced).resolves.toBeUndefined()
		await Promise.all([first, deleting])

		// The displaced task never POSTed — the delete carries the later state.
		expect(updateDraft).toHaveBeenCalledTimes(2)
		expect(updateDraft.mock.calls.map((c: any[]) => c[0].requestBody.value)).toEqual([
			{ content: '1' },
			null
		])
	})

	it('does not resolve a displaced save before the superseding POST lands', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/displaced_b' }
		const inFlight = deferred()
		const deletePost = deferred()
		updateDraft
			.mockImplementationOnce(async () => {
				await inFlight.promise
				return { status: 'saved', current_timestamp: '2020-01-01T00:00:00Z' }
			})
			.mockImplementationOnce(async () => {
				await deletePost.promise
				return { status: 'saved', current_timestamp: '2020-01-01T00:00:01Z' }
			})

		const first = UserDraftDbSyncer.save({ ...q, value: { content: '1' }, immediate: true })
		const displaced = UserDraftDbSyncer.save({ ...q, value: { content: '2' }, immediate: true })
		const deleting = UserDraftDbSyncer.save({ ...q, value: null, immediate: true })

		let displacedSettled = false
		void displaced.then(() => (displacedSettled = true))

		inFlight.resolve()
		await vi.waitFor(() => expect(updateDraft).toHaveBeenCalledTimes(2))
		// Delete still in flight: callers that `await save()` before invalidating
		// must not read the server yet.
		expect(displacedSettled).toBe(false)

		deletePost.resolve()
		await Promise.all([first, displaced, deleting])
		expect(displacedSettled).toBe(true)
	})

	it('resolves a pending save dropped by lockSync without POSTing it', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/displaced_lock' }
		const inFlight = deferred()
		updateDraft.mockImplementationOnce(async () => {
			await inFlight.promise
			return { status: 'saved', current_timestamp: '2020-01-01T00:00:00Z' }
		})

		const first = UserDraftDbSyncer.save({ ...q, value: { content: '1' }, immediate: true })
		const dropped = UserDraftDbSyncer.save({ ...q, value: { content: '2' }, immediate: true })
		// Another user's draft was loaded: this value must never reach the server.
		UserDraftDbSyncer.lockSync(q)

		inFlight.resolve()
		// Resolves like every other save on a locked key — the lock's whole point
		// is that the write is dropped, so the caller has nothing to wait for.
		await expect(dropped).resolves.toBeUndefined()
		await first
		expect(updateDraft).toHaveBeenCalledTimes(1)
		expect(updateDraft.mock.calls[0][0].requestBody.value).toEqual({ content: '1' })
		UserDraftDbSyncer.unlockSync(q)
	})

	it('resolves a flush displaced by a later immediate save', async () => {
		const q = { workspace: 'w', itemKind: 'script' as const, path: 'u/me/displaced_c' }
		const inFlight = deferred()
		updateDraft.mockImplementationOnce(async () => {
			await inFlight.promise
			return { status: 'saved', current_timestamp: '2020-01-01T00:00:00Z' }
		})

		// Park opts (the reactive mirror's autosave) so `flush` has something to send.
		void UserDraftDbSyncer.save({ ...q, value: { content: 'typed' }, auto: true })
		const first = UserDraftDbSyncer.save({ ...q, value: { content: 'x' }, immediate: true })
		const flushed = UserDraftDbSyncer.flush(q) // pending behind `first`
		const deleting = UserDraftDbSyncer.save({ ...q, value: null, immediate: true }) // displaces it

		inFlight.resolve()
		await expect(flushed).resolves.toBeUndefined()
		await Promise.all([first, deleting])
	})
})
