import { describe, it, expect } from 'vitest'
import { deriveForkStatus, isForkSession, type Session } from './sessionState.svelte'
import type { UserWorkspace } from '$lib/stores'
import type { WorkspaceComparison } from '$lib/gen'

// Minimal fixtures — only the fields these pure helpers read matter; the rest
// is filled via cast so we don't track unrelated schema churn.
function session(over: Partial<Session> = {}): Session {
	return { id: 's1', name: 'sess', createdAt: 0, ...over }
}
function ws(id: string, parent?: string): UserWorkspace {
	return { id, name: id, parent_workspace_id: parent } as unknown as UserWorkspace
}
function comparison(total_ahead: number, total_behind: number): WorkspaceComparison {
	return { summary: { total_ahead, total_behind } } as unknown as WorkspaceComparison
}

describe('isForkSession', () => {
	it('is false for a draft with no workspace at all', () => {
		expect(isForkSession(session(), [])).toBe(false)
	})

	it('is false when the workspace is a (non-fork) root', () => {
		expect(isForkSession(session({ workspace_id: 'root' }), [ws('root')])).toBe(false)
	})

	it('is true when the workspace has a parent (is a fork)', () => {
		expect(isForkSession(session({ workspace_id: 'fork' }), [ws('fork', 'root')])).toBe(true)
	})

	it('treats a committed-but-missing workspace as a fork (terminal unavailable state)', () => {
		expect(isForkSession(session({ workspace_id: 'gone' }), [])).toBe(true)
	})

	it('is false for a draft whose pending workspace is missing', () => {
		expect(isForkSession(session({ pending_workspace_id: 'gone' }), [])).toBe(false)
	})

	it('resolves a draft via its pending workspace', () => {
		expect(isForkSession(session({ pending_workspace_id: 'fork' }), [ws('fork', 'root')])).toBe(
			true
		)
	})
})

describe('deriveForkStatus', () => {
	it('is undefined for a draft with no workspace', () => {
		expect(deriveForkStatus(session(), [], undefined)).toBeUndefined()
	})

	it('is unavailable when a committed workspace is no longer in the list', () => {
		expect(deriveForkStatus(session({ workspace_id: 'gone' }), [], comparison(0, 0))).toBe(
			'unavailable'
		)
	})

	it('is undefined when a draft pending workspace is missing (not yet committed)', () => {
		expect(
			deriveForkStatus(session({ pending_workspace_id: 'gone' }), [], undefined)
		).toBeUndefined()
	})

	it('is undefined for a non-fork (root) workspace', () => {
		expect(
			deriveForkStatus(session({ workspace_id: 'root' }), [ws('root')], comparison(3, 3))
		).toBeUndefined()
	})

	it('is undefined for a fork before the comparison has loaded', () => {
		expect(
			deriveForkStatus(session({ workspace_id: 'fork' }), [ws('fork', 'root')], undefined)
		).toBeUndefined()
	})

	it('is diverged when the fork is both ahead and behind', () => {
		expect(
			deriveForkStatus(session({ workspace_id: 'fork' }), [ws('fork', 'root')], comparison(2, 1))
		).toBe('diverged')
	})

	it('is ahead when the fork has unmerged changes and the parent has not moved', () => {
		expect(
			deriveForkStatus(session({ workspace_id: 'fork' }), [ws('fork', 'root')], comparison(2, 0))
		).toBe('ahead')
	})

	it('is in_sync when neither side is ahead', () => {
		expect(
			deriveForkStatus(session({ workspace_id: 'fork' }), [ws('fork', 'root')], comparison(0, 0))
		).toBe('in_sync')
	})

	it('is in_sync when only the parent moved (behind-only, fork has no local changes)', () => {
		expect(
			deriveForkStatus(session({ workspace_id: 'fork' }), [ws('fork', 'root')], comparison(0, 2))
		).toBe('in_sync')
	})
})
