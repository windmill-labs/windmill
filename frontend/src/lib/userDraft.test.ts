import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'

// DraftService is hit (debounced) by UserDraftDbService; stub it so tests never
// touch the network and we can assert the calls.
const createDraft = vi.fn(async () => 'ok')
const deleteDraft = vi.fn(async () => 'ok')
vi.mock('./gen', () => ({
	DraftService: {
		createDraft: (...args: any[]) => createDraft(...args),
		deleteDraft: (...args: any[]) => deleteDraft(...args)
	}
}))

import {
	UserDraft,
	normalizeForCompare,
	localDraftDiffers,
	__resetUserDraftForTesting
} from './userDraft.svelte'
import { UserDraftDbService } from './userDraftDbService'

const WS = 'test-ws'
const opts = { workspace: WS }

beforeEach(() => {
	__resetUserDraftForTesting()
	createDraft.mockClear()
	deleteDraft.mockClear()
})

describe('normalizeForCompare', () => {
	it('drops undefined-valued keys via JSON round-trip', () => {
		expect(normalizeForCompare({ a: 1, b: undefined })).toEqual({ a: 1 })
	})
	it('returns undefined for undefined', () => {
		expect(normalizeForCompare(undefined)).toBeUndefined()
	})
})

describe('localDraftDiffers', () => {
	it('is false when there is no local draft', () => {
		expect(localDraftDiffers(undefined, { a: 1 })).toBe(false)
		expect(localDraftDiffers(null, { a: 1 })).toBe(false)
	})
	it('treats {a: undefined} and {} as equal (normalized)', () => {
		expect(localDraftDiffers({ a: undefined } as any, {} as any)).toBe(false)
	})
	it('is true when the values meaningfully differ', () => {
		expect(localDraftDiffers({ a: 1 }, { a: 2 })).toBe(true)
	})
})

describe('UserDraft in-memory store (no live handle)', () => {
	it('save then get returns the value without a mounted handle', () => {
		UserDraft.save('resource', 'f/r/db', { host: 'x' }, opts)
		expect(UserDraft.get('resource', 'f/r/db', opts)).toEqual({ host: 'x' })
		expect(UserDraft.has('resource', 'f/r/db', opts)).toBe(true)
	})

	it('remove clears the value', () => {
		UserDraft.save('resource', 'f/r/db', { host: 'x' }, opts)
		UserDraft.remove('resource', 'f/r/db', opts)
		expect(UserDraft.get('resource', 'f/r/db', opts)).toBeUndefined()
		expect(UserDraft.has('resource', 'f/r/db', opts)).toBe(false)
	})

	it('discard resets the value to the fallback', () => {
		UserDraft.save('resource', 'f/r/db', { host: 'edited' }, opts)
		UserDraft.discard('resource', 'f/r/db', { host: 'deployed' }, opts)
		expect(UserDraft.get('resource', 'f/r/db', opts)).toEqual({ host: 'deployed' })
	})

	it('list reflects in-memory drafts', () => {
		UserDraft.save('resource', 'f/r/a', { v: 1 }, opts)
		UserDraft.save('variable', 'f/v/b', { v: 2 }, opts)
		const list = UserDraft.list(opts)
		expect(list.map((d) => d.path).sort()).toEqual(['f/r/a', 'f/v/b'])
	})
})

describe('UserDraftDbService (debounced persistence)', () => {
	beforeEach(() => {
		vi.useFakeTimers()
	})
	afterEach(() => {
		vi.runOnlyPendingTimers()
		vi.useRealTimers()
	})

	it('persists DB-backed kinds via createDraft after debounce', async () => {
		UserDraftDbService.save({ workspace: WS, itemKind: 'script', path: 'f/s/x', content: { a: 1 } })
		expect(createDraft).not.toHaveBeenCalled() // debounced
		await vi.advanceTimersByTimeAsync(700)
		expect(createDraft).toHaveBeenCalledTimes(1)
		expect(createDraft.mock.calls[0][0]).toMatchObject({
			workspace: WS,
			requestBody: { path: 'f/s/x', typ: 'script', value: { a: 1 } }
		})
	})

	it('maps raw_app to the app draft typ', async () => {
		UserDraftDbService.save({
			workspace: WS,
			itemKind: 'raw_app',
			path: 'f/a/x',
			content: { a: 1 }
		})
		await vi.advanceTimersByTimeAsync(700)
		expect(createDraft.mock.calls[0][0].requestBody.typ).toBe('app')
	})

	it('deletes the draft when content is undefined', async () => {
		UserDraftDbService.save({ workspace: WS, itemKind: 'flow', path: 'f/f/x', content: undefined })
		await vi.advanceTimersByTimeAsync(700)
		expect(deleteDraft).toHaveBeenCalledTimes(1)
		expect(deleteDraft.mock.calls[0][0]).toMatchObject({
			workspace: WS,
			kind: 'flow',
			path: 'f/f/x'
		})
	})

	it('is a no-op for kinds without a DB draft', async () => {
		UserDraftDbService.save({
			workspace: WS,
			itemKind: 'resource',
			path: 'f/r/x',
			content: { a: 1 }
		})
		UserDraftDbService.save({
			workspace: WS,
			itemKind: 'trigger_http',
			path: 'f/t/x',
			content: { a: 1 }
		})
		await vi.advanceTimersByTimeAsync(700)
		expect(createDraft).not.toHaveBeenCalled()
		expect(deleteDraft).not.toHaveBeenCalled()
	})

	it('skips brand-new items at the empty path', async () => {
		UserDraftDbService.save({ workspace: WS, itemKind: 'script', path: '', content: { a: 1 } })
		await vi.advanceTimersByTimeAsync(700)
		expect(createDraft).not.toHaveBeenCalled()
	})

	it('coalesces a burst of edits into the latest write', async () => {
		UserDraftDbService.save({ workspace: WS, itemKind: 'script', path: 'f/s/y', content: { v: 1 } })
		UserDraftDbService.save({ workspace: WS, itemKind: 'script', path: 'f/s/y', content: { v: 2 } })
		UserDraftDbService.save({ workspace: WS, itemKind: 'script', path: 'f/s/y', content: { v: 3 } })
		await vi.advanceTimersByTimeAsync(700)
		expect(createDraft).toHaveBeenCalledTimes(1)
		expect(createDraft.mock.calls[0][0].requestBody.value).toEqual({ v: 3 })
	})
})
