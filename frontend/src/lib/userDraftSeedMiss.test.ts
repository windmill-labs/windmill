import { describe, it, expect, vi } from 'vitest'

vi.mock('./gen', () => ({ DraftService: { updateDraft: vi.fn() } }))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))
vi.mock('./localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))
vi.mock('./userDraftDbSyncer.svelte', () => ({
	UserDraftDbSyncer: { save: vi.fn(), flush: vi.fn(async () => {}) }
}))

import { UserDraft } from './userDraft.svelte'

const OPTS = { workspace: 'ws' }
let n = 0
const nextPath = () => `u/me/s${n++}`

/**
 * A chat write seeds the cell an editor is about to hold. Seeding before it does
 * records the miss, which the session reads back to know the write never reached
 * the form and the editor has to re-read it.
 */
describe('seed misses', () => {
	it('records a miss when no editor holds the cell', () => {
		const path = nextPath()
		UserDraft.seed('trigger_schedule', path, { a: 1 }, OPTS)
		expect(UserDraft.takeSeedMiss('trigger_schedule', path, OPTS)).toBe(true)
		// Read once: the next action must not spend the same marker.
		expect(UserDraft.takeSeedMiss('trigger_schedule', path, OPTS)).toBe(false)
	})

	// The editor adopts what it loaded as the cell's baseline once it mounts, and
	// that load can predate the write that missed. Such a seed stays out of the
	// bookkeeping entirely — it neither records a miss nor consumes one.
	it('leaves a standing miss alone when the editor bootstraps its baseline', () => {
		const path = nextPath()
		UserDraft.seed('trigger_schedule', path, { a: 1 }, OPTS)
		UserDraft.seed('trigger_schedule', path, { a: 0 }, { ...OPTS, baseline: true })
		expect(UserDraft.takeSeedMiss('trigger_schedule', path, OPTS)).toBe(true)
	})

	it('records nothing for a bootstrap seed of its own', () => {
		const path = nextPath()
		UserDraft.seed('trigger_schedule', path, { a: 0 }, { ...OPTS, baseline: true })
		expect(UserDraft.takeSeedMiss('trigger_schedule', path, OPTS)).toBe(false)
	})
})
