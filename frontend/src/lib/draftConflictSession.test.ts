import { describe, it, expect } from 'vitest'

import { createDraftConflictSession } from './draftConflictSession.svelte'

/**
 * A resolution the user has walked away from must neither speak for what replaced it nor hold its
 * buttons disabled until it settles.
 */
describe('createDraftConflictSession', () => {
	it('frees the next session from a resolution the last one left in flight', () => {
		const session = createDraftConflictSession()

		const stale = session.start()
		expect(session.busy).toBe(true)

		session.end()
		expect(session.busy).toBe(false)
		expect(session.holds(stale)).toBe(false)

		const fresh = session.start()
		expect(session.busy).toBe(true)
		expect(session.holds(fresh)).toBe(true)

		// The stale request settles last: it must not release the live session's claim.
		session.finish(stale)
		expect(session.busy).toBe(true)

		session.finish(fresh)
		expect(session.busy).toBe(false)
	})
})
