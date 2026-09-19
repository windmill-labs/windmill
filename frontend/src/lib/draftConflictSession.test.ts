import { describe, it, expect } from 'vitest'

import { useDraftConflictSession } from './draftConflictSession.svelte'

/**
 * A drawer editor is reused by the next thing it opens, so a resolution the closed session left
 * in flight must neither speak for the new one nor hold its buttons disabled until it settles.
 */
describe('useDraftConflictSession', () => {
	it('frees a reopened session from a resolution the closed one left in flight', () => {
		const session = useDraftConflictSession()

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
