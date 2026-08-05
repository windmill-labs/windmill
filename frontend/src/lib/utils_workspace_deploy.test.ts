import { describe, it, expect } from 'vitest'
import {
	checkPathWritePermission,
	diffActionableInDirection,
	diffCreatesInTarget,
	diffRemovesInTarget
} from './utils_workspace_deploy'

/** The row shape the fork comparison returns for an item the parent has and the
 * fork does not: one write on the fork side, whatever that write was. */
const parentOnly = { ahead: 1, behind: 0, exists_in_source: true, exists_in_fork: false }
const forkDeleted = {
	...parentOnly,
	fork_last_event_kind: 'delete',
	fork_last_event_origin: 'authored'
} as const
const forkRenamedAway = {
	...parentOnly,
	fork_last_event_kind: 'rename_from',
	fork_last_event_origin: 'authored'
} as const
const syncReverted = {
	...parentOnly,
	fork_last_event_kind: 'delete',
	fork_last_event_origin: 'sync'
} as const
const forkOnly = { ahead: 1, behind: 0, exists_in_source: false, exists_in_fork: true }
const bothSides = { ahead: 1, behind: 1, exists_in_source: true, exists_in_fork: true }

describe('deploy direction of a one-sided diff row', () => {
	it('offers a parent-only item to the fork even with no behind count', () => {
		expect(diffActionableInDirection(parentOnly, false)).toBe(true)
		expect(diffCreatesInTarget(parentOnly, false)).toBe(true)
		expect(diffRemovesInTarget(parentOnly, false)).toBe(false)
	})

	it('keeps a parent-only row with no recorded event out of a merge into the parent', () => {
		expect(diffActionableInDirection(parentOnly, true)).toBe(false)
		// An arbitrary target has no tally behind it and does propagate the removal.
		expect(diffActionableInDirection(parentOnly, true, true)).toBe(true)
		expect(diffRemovesInTarget(parentOnly, true)).toBe(true)
	})

	it('merges a removal the fork can show it made, and never one a sync made', () => {
		expect(diffActionableInDirection(forkDeleted, true)).toBe(true)
		expect(diffActionableInDirection(forkRenamedAway, true)).toBe(true)
		expect(diffActionableInDirection(syncReverted, true)).toBe(false)
		// Still a removal, so still opt-in rather than bulk-selected.
		expect(diffRemovesInTarget(forkDeleted, true)).toBe(true)
		// The update direction keeps offering it back whatever the fork recorded.
		expect(diffActionableInDirection(syncReverted, false)).toBe(true)
	})

	it('surfaces a fork deletion the parent also edited in both directions', () => {
		const conflict = { ...forkDeleted, behind: 1 }
		expect(diffActionableInDirection(conflict, true)).toBe(true)
		expect(diffActionableInDirection(conflict, false)).toBe(true)
	})

	it('does not resurrect a fork-only item into an update of the fork', () => {
		expect(diffActionableInDirection(forkOnly, false)).toBe(false)
		expect(diffCreatesInTarget(forkOnly, true)).toBe(true)
	})

	it('keeps a two-sided row on its counters', () => {
		expect(diffActionableInDirection(bothSides, true)).toBe(true)
		expect(diffActionableInDirection({ ...bothSides, behind: 0 }, false)).toBe(false)
		expect(diffCreatesInTarget(bothSides, true)).toBe(false)
		expect(diffRemovesInTarget(bothSides, false)).toBe(false)
	})
})

describe('per-item write permission in the deploy target', () => {
	const member = { is_admin: false, username: 'alice', folders: ['shared'] }
	const never = async () => {
		throw new Error('folder probe should not run')
	}

	it('lets a workspace admin write anywhere', async () => {
		const admin = { is_admin: true, username: 'root', folders: [] }
		expect(await checkPathWritePermission('dev', 'u/someone/x', admin, never)).toEqual({ ok: true })
		expect(await checkPathWritePermission('dev', 'f/locked/x', admin, never)).toEqual({ ok: true })
	})

	it('allows a user their own path and refuses someone else’s', async () => {
		expect(await checkPathWritePermission('dev', 'u/alice/x', member, never)).toEqual({ ok: true })
		const refused = await checkPathWritePermission('dev', 'u/bob/x', member, never)
		expect(refused.ok).toBe(false)
		expect(refused.reason).toContain('u/bob')
	})

	it('allows a folder in the write set without probing for it', async () => {
		expect(await checkPathWritePermission('dev', 'f/shared/x', member, never)).toEqual({ ok: true })
	})

	it('refuses a folder that exists in the target but is not writable', async () => {
		const refused = await checkPathWritePermission('dev', 'f/locked/x', member, async () => true)
		expect(refused.ok).toBe(false)
		expect(refused.reason).toContain('locked')
	})

	// The two fail-open paths. Turning either into a refusal would block a deploy the server
	// would have accepted, so they are asserted rather than left to the `catch` reading as dead.
	it('allows a folder the target does not have yet, since the deploy creates it', async () => {
		expect(
			await checkPathWritePermission('dev', 'f/brand_new/x', member, async () => false)
		).toEqual({ ok: true })
	})

	it('allows when the folder probe itself fails', async () => {
		const probeFailed = async () => {
			throw new Error('network')
		}
		expect(await checkPathWritePermission('dev', 'f/locked/x', member, probeFailed)).toEqual({
			ok: true
		})
	})
})
