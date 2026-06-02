import { describe, it, expect, vi } from 'vitest'
import { get } from 'svelte/store'
import {
	commitSessionWorkspace,
	deriveForkStatus,
	isForkSession,
	sessionState,
	type Session
} from './sessionState.svelte'
import { enterpriseLicense, workspaceStore, type UserWorkspace } from '$lib/stores'
import { WorkspaceService, type WorkspaceComparison } from '$lib/gen'

// Force createWorkspaceFork to fail so we can pin commitSessionWorkspace's
// failure contract (the invariant the beforeSend abort fix relies on).
vi.mock('$lib/gen', async (orig) => {
	const actual = await orig<typeof import('$lib/gen')>()
	return {
		...actual,
		WorkspaceService: {
			...actual.WorkspaceService,
			createWorkspaceFork: vi.fn().mockRejectedValue(new Error('fork creation failed')),
			listUserWorkspaces: vi.fn().mockResolvedValue([])
		}
	}
})

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

describe('commitSessionWorkspace — fork-creation failure', () => {
	it('returns undefined and drops pending_fork (so beforeSend aborts the send)', async () => {
		const id = 'test-commit-fork-fail'
		// Forking is licensed here so the commit reaches materializeFork (which is
		// mocked to fail); the unlicensed path is covered separately below.
		const prevLicense = get(enterpriseLicense)
		enterpriseLicense.set('test-license')
		sessionState.sessions.push({
			id,
			name: 'fork-fail',
			createdAt: 0,
			pending_fork: { parent_workspace_id: 'parent_ws', id: 'wm-fork-nope', name: 'nope' }
		} as Session)
		try {
			const committed = await commitSessionWorkspace(id, 'parent_ws')
			// Not committed → undefined. This is what makes beforeSend throw rather
			// than letting the first message ship to the parent workspace.
			expect(committed).toBeUndefined()
			const s = sessionState.sessions.find((x) => x.id === id)
			expect(s?.workspace_id).toBeUndefined()
			expect(s?.pending_fork).toBeUndefined()
		} finally {
			const i = sessionState.sessions.findIndex((x) => x.id === id)
			if (i >= 0) sessionState.sessions.splice(i, 1)
			enterpriseLicense.set(prevLicense)
		}
	})
})

describe('commitSessionWorkspace — unlicensed fork guard', () => {
	it('throws and never calls createWorkspaceFork without an enterprise license', async () => {
		const id = 'test-commit-fork-unlicensed'
		const prevLicense = get(enterpriseLicense)
		enterpriseLicense.set(undefined)
		vi.mocked(WorkspaceService.createWorkspaceFork).mockClear()
		sessionState.sessions.push({
			id,
			name: 'fork-unlicensed',
			createdAt: 0,
			pending_fork: { parent_workspace_id: 'parent_ws', id: 'wm-fork-ee', name: 'ee' }
		} as Session)
		try {
			await expect(commitSessionWorkspace(id, 'parent_ws')).rejects.toThrow(/enterprise license/)
			expect(WorkspaceService.createWorkspaceFork).not.toHaveBeenCalled()
			// pending_fork is preserved so the block persists until the user picks a
			// non-fork workspace (which clears it via setSessionPendingWorkspace).
			const s = sessionState.sessions.find((x) => x.id === id)
			expect(s?.pending_fork).toBeDefined()
			expect(s?.workspace_id).toBeUndefined()
		} finally {
			const i = sessionState.sessions.findIndex((x) => x.id === id)
			if (i >= 0) sessionState.sessions.splice(i, 1)
			enterpriseLicense.set(prevLicense)
		}
	})
})

describe('commitSessionWorkspace — workspaceStore sync (non-fork branch)', () => {
	it('syncs workspaceStore to the committed workspace when they differ', async () => {
		// Repro: user is sitting in a fork workspace (wm-fork-x) and creates a
		// new session whose pending_workspace_id defaults to the family root.
		// Without the syncWorkspaceTo call in commitSessionWorkspace's non-fork
		// branch, the session metadata says root while the active workspace
		// stays on the fork — so AIChatManager.chatRequest's logAiChat and tool
		// calls would target the wrong workspace.
		const id = 'test-commit-ws-sync'
		const prev = get(workspaceStore)
		workspaceStore.set('wm-fork-x')
		sessionState.sessions.push({
			id,
			name: 'ws-sync',
			createdAt: 0,
			pending_workspace_id: 'root_ws'
		} as Session)
		try {
			const committed = await commitSessionWorkspace(id, undefined)
			expect(committed).toBe('root_ws')
			const s = sessionState.sessions.find((x) => x.id === id)
			expect(s?.workspace_id).toBe('root_ws')
			expect(s?.pending_workspace_id).toBeUndefined()
			expect(get(workspaceStore)).toBe('root_ws')
		} finally {
			const i = sessionState.sessions.findIndex((x) => x.id === id)
			if (i >= 0) sessionState.sessions.splice(i, 1)
			workspaceStore.set(prev)
		}
	})

	it('is a no-op on workspaceStore when it already matches the committed workspace', async () => {
		const id = 'test-commit-ws-match'
		const prev = get(workspaceStore)
		workspaceStore.set('root_ws')
		sessionState.sessions.push({
			id,
			name: 'ws-match',
			createdAt: 0,
			pending_workspace_id: 'root_ws'
		} as Session)
		try {
			const committed = await commitSessionWorkspace(id, undefined)
			expect(committed).toBe('root_ws')
			expect(get(workspaceStore)).toBe('root_ws')
		} finally {
			const i = sessionState.sessions.findIndex((x) => x.id === id)
			if (i >= 0) sessionState.sessions.splice(i, 1)
			workspaceStore.set(prev)
		}
	})
})
