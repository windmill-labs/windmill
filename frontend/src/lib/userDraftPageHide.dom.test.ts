import { describe, it, expect, vi } from 'vitest'

// The page-hide flush is wired to a `pagehide` listener registered at import time, and only when
// a `document` exists — so this is a `.dom` test: under the node project the listener is never
// registered and the path cannot be reached at all.
const updateDraft = vi.fn(async () => ({
	status: 'saved' as const,
	current_timestamp: '2020-01-01T00:00:00Z'
}))

vi.mock('./gen', () => ({
	DraftService: { updateDraft: (...a: unknown[]) => updateDraft(...(a as [])) }
}))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))
vi.mock('./localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))

import { UserDraftDbSyncer } from './userDraftDbSyncer.svelte'

describe('UserDraftDbSyncer page-hide flush', () => {
	it('keeps a refused payload across pagehide and lets the rest go with the page', () => {
		globalThis.fetch = vi.fn(async () => new Response(null, { status: 200 })) as never
		const refused = { workspace: 'w', itemKind: 'variable' as const, path: 'u/me/refused' }
		const ordinary = { workspace: 'w', itemKind: 'variable' as const, path: 'u/me/ordinary' }

		// Both parked by the debouncer, neither sent.
		void UserDraftDbSyncer.save({ ...refused, value: { v: 'mine, refused' } })
		void UserDraftDbSyncer.save({ ...ordinary, value: { v: 'ordinary' } })
		UserDraftDbSyncer.markConflict(refused, '2020-01-02T00:00:00Z')

		window.dispatchEvent(new Event('pagehide'))

		// The ordinary one has gone out with the page. The refused one was skipped rather than
		// sent, so clearing it here would leave a bfcache restore — the same context coming back —
		// with no copy of that edit, and the next read would adopt the remote draft over it.
		expect(UserDraftDbSyncer.pendingValue(ordinary)).toBeUndefined()
		expect(UserDraftDbSyncer.pendingValue(refused)).toEqual({ v: 'mine, refused' })
	})
})
