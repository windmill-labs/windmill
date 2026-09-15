import { describe, it, expect, beforeEach, vi } from 'vitest'

// Stands in for the syncer's per-key facts. `landed` is what the response
// handler recorded; `state` is what is still queued behind it.
let landedDelete = false
let state: 'none' | 'pending' | 'saving' | 'failed' = 'none'
vi.mock('./userDraftDbSyncer.svelte', () => ({
	UserDraftDbSyncer: {
		flush: vi.fn(async () => {}),
		lastLandedWasDelete: vi.fn(() => landedDelete),
		getState: vi.fn(() => ({ state })),
		save: vi.fn()
	}
}))
vi.mock('./gen', () => ({ DraftService: { updateDraft: vi.fn() } }))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))
vi.mock('./localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))

import { flushDraftDelete } from './userDraft.svelte'

const run = () => flushDraftDelete('variable', 'u/me/v', { workspace: 'ws' })

beforeEach(() => {
	landedDelete = false
	state = 'none'
})

/**
 * A caller leaves an editor on this verdict, so anything short of "the draft is
 * gone and staying gone" has to read false.
 */
describe('flushDraftDelete', () => {
	it('confirms a delete that landed with nothing behind it', async () => {
		landedDelete = true
		expect(await run()).toBe(true)
	})

	// The form stays editable after Discard and `flush` only submits what it read
	// when it started, so an edit made in between is still queued — and would
	// recreate the draft once the caller had already left.
	it('rejects a delete with an edit queued behind it', async () => {
		landedDelete = true
		state = 'pending'
		expect(await run()).toBe(false)
	})

	// A failed delete never reaches the response handler, so nothing records it as
	// landed — the display hint would have said otherwise, since the editor
	// publishes that one itself the moment the cell returns to its baseline.
	it('rejects a delete that failed', async () => {
		landedDelete = false
		state = 'failed'
		expect(await run()).toBe(false)
	})

	it('rejects a key where an upsert landed last', async () => {
		landedDelete = false
		expect(await run()).toBe(false)
	})
})
