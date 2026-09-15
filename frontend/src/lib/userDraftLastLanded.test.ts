import { describe, it, expect, vi } from 'vitest'

let response: { status: 'saved' | 'conflict'; current_timestamp: string } = {
	status: 'saved',
	current_timestamp: '2020-01-01T00:00:00Z'
}
const updateDraft = vi.fn(async (..._args: any[]) => response)
vi.mock('./gen', () => ({
	DraftService: { updateDraft: (...a: unknown[]) => updateDraft(...(a as [])) }
}))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))
vi.mock('./localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))

import { UserDraftDbSyncer } from './userDraftDbSyncer.svelte'

const query = { workspace: 'ws', itemKind: 'variable' as const, path: 'u/me/v' }
const send = (value: unknown) => UserDraftDbSyncer.save({ ...query, value, immediate: true })

/**
 * `flushDraftDelete` leaves an editor on this fact, and it is deliberately
 * sticky — so it has to stop describing the server the moment the server stops
 * being ours to describe.
 */
describe('lastLandedWasDelete', () => {
	it('follows what the response handler saw land, and drops it on a conflict', async () => {
		await send({ path: 'u/me/v' })
		expect(UserDraftDbSyncer.lastLandedWasDelete(query)).toBe(false)

		await send(null)
		expect(UserDraftDbSyncer.lastLandedWasDelete(query)).toBe(true)

		// The server moved under us: the earlier delete no longer answers for this one.
		response = { status: 'conflict', current_timestamp: '2020-01-02T00:00:00Z' }
		await send(null)
		expect(UserDraftDbSyncer.lastLandedWasDelete(query)).toBe(false)
	})
})
