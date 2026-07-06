import { describe, it, expect, vi } from 'vitest'
import { get } from 'svelte/store'
import {
	commitSessionWorkspace,
	decideSessionLifecycle,
	isForkSession,
	renameSession,
	setGeneratedSessionSummary,
	sessionState,
	type Session
} from './sessionState.svelte'
import {
	enterpriseLicense,
	usersWorkspaceStore,
	workspaceStore,
	type UserWorkspace
} from '$lib/stores'
import { WorkspaceService } from '$lib/gen'

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

// Set the workspace list to a given number of non-'admins' workspaces so the
// commit-path's CE workspace-cap check (mirror of backend _check_nb_of_workspaces)
// can be exercised. Returns a restore fn.
function withNonAdminWorkspaces(count: number): () => void {
	const prev = get(usersWorkspaceStore)
	const workspaces = Array.from({ length: count }, (_, i) => ws(`ws_${i}`))
	usersWorkspaceStore.set({ email: 't@t', workspaces } as never)
	return () => usersWorkspaceStore.set(prev)
}

describe('commitSessionWorkspace — CE workspace-cap fork guard', () => {
	it('throws and never calls createWorkspaceFork when the CE workspace cap is reached', async () => {
		const id = 'test-commit-fork-capped'
		const prevLicense = get(enterpriseLicense)
		enterpriseLicense.set(undefined)
		// At/above the cap (2 non-'admins' workspaces) the backend would reject, so
		// the client guard blocks the commit before materializeFork.
		const restoreWs = withNonAdminWorkspaces(2)
		vi.mocked(WorkspaceService.createWorkspaceFork).mockClear()
		sessionState.sessions.push({
			id,
			name: 'fork-capped',
			createdAt: 0,
			pending_fork: { parent_workspace_id: 'parent_ws', id: 'wm-fork-capped', name: 'capped' }
		} as Session)
		try {
			await expect(commitSessionWorkspace(id, 'parent_ws')).rejects.toThrow(/limited to/)
			expect(WorkspaceService.createWorkspaceFork).not.toHaveBeenCalled()
			// pending_fork is preserved so the block persists until the user picks a
			// non-fork workspace (which clears it via setSessionPendingWorkspace).
			const s = sessionState.sessions.find((x) => x.id === id)
			expect(s?.pending_fork).toBeDefined()
			expect(s?.workspace_id).toBeUndefined()
		} finally {
			const i = sessionState.sessions.findIndex((x) => x.id === id)
			if (i >= 0) sessionState.sessions.splice(i, 1)
			restoreWs()
			enterpriseLicense.set(prevLicense)
		}
	})

	it('lets an unlicensed user under the cap proceed to createWorkspaceFork', async () => {
		const id = 'test-commit-fork-under-cap'
		const prevLicense = get(enterpriseLicense)
		enterpriseLicense.set(undefined)
		// Below the cap (1 non-'admins' workspace) CE still allows a fork — the
		// guard must NOT fire. createWorkspaceFork is mocked to reject, so the
		// commit falls through to the failure contract (undefined, pending dropped).
		const restoreWs = withNonAdminWorkspaces(1)
		vi.mocked(WorkspaceService.createWorkspaceFork).mockClear()
		sessionState.sessions.push({
			id,
			name: 'fork-under-cap',
			createdAt: 0,
			pending_fork: { parent_workspace_id: 'parent_ws', id: 'wm-fork-ok', name: 'ok' }
		} as Session)
		try {
			const committed = await commitSessionWorkspace(id, 'parent_ws')
			expect(committed).toBeUndefined()
			expect(WorkspaceService.createWorkspaceFork).toHaveBeenCalled()
			const s = sessionState.sessions.find((x) => x.id === id)
			expect(s?.pending_fork).toBeUndefined()
		} finally {
			const i = sessionState.sessions.findIndex((x) => x.id === id)
			if (i >= 0) sessionState.sessions.splice(i, 1)
			restoreWs()
			enterpriseLicense.set(prevLicense)
		}
	})
})

describe('commitSessionWorkspace — workspaceStore sync (non-fork branch)', () => {
	it('syncs workspaceStore to the committed workspace when they differ', async () => {
		// Repro: user is sitting in a fork workspace (wm-fork-x) and creates a
		// new session whose pending_workspace_id defaults to the root workspace.
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
			expect(s?.workspace_root_id).toBe('root_ws')
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
			const s = sessionState.sessions.find((x) => x.id === id)
			expect(s?.workspace_root_id).toBe('root_ws')
			expect(get(workspaceStore)).toBe('root_ws')
		} finally {
			const i = sessionState.sessions.findIndex((x) => x.id === id)
			if (i >= 0) sessionState.sessions.splice(i, 1)
			workspaceStore.set(prev)
		}
	})
})

describe('session summary generation guards', () => {
	it('applies a generated summary only while the placeholder is still untouched', () => {
		const id = 'test-generated-summary'
		sessionState.sessions.push({
			id,
			name: 'generated-summary',
			createdAt: 0,
			chatId: 'chat-1',
			summary: 'Bright session',
			summarySource: 'placeholder'
		} as Session)
		try {
			expect(setGeneratedSessionSummary(id, 'Build invoice workflow', 'chat-1')).toBe(true)
			const s = sessionState.sessions.find((x) => x.id === id)
			expect(s?.summary).toBe('Build invoice workflow')
			expect(s?.summarySource).toBe('generated')
		} finally {
			const i = sessionState.sessions.findIndex((x) => x.id === id)
			if (i >= 0) sessionState.sessions.splice(i, 1)
		}
	})

	it('does not overwrite a manual rename or a different chat id', () => {
		const id = 'test-generated-summary-manual'
		sessionState.sessions.push({
			id,
			name: 'generated-summary-manual',
			createdAt: 0,
			chatId: 'chat-1',
			summary: 'Bright session',
			summarySource: 'placeholder'
		} as Session)
		try {
			expect(setGeneratedSessionSummary(id, 'Wrong chat title', 'chat-2')).toBe(false)
			renameSession(id, 'My chosen title')
			expect(setGeneratedSessionSummary(id, 'Generated title', 'chat-1')).toBe(false)
			const s = sessionState.sessions.find((x) => x.id === id)
			expect(s?.summary).toBe('My chosen title')
			expect(s?.summarySource).toBe('manual')
		} finally {
			const i = sessionState.sessions.findIndex((x) => x.id === id)
			if (i >= 0) sessionState.sessions.splice(i, 1)
		}
	})
})

describe('decideSessionLifecycle — the never-orphaned rule (pure)', () => {
	const mk = (over: Partial<Session> = {}): Session => ({
		id: 'x',
		name: 'session-1',
		workspace_id: 'ws',
		createdAt: 0,
		...over
	})

	it('deleted workspace → delete, regardless of archive state', () => {
		expect(decideSessionLifecycle(mk(), 'deleted')).toEqual({ action: 'delete' })
		expect(decideSessionLifecycle(mk({ archived: true }), 'deleted')).toEqual({ action: 'delete' })
	})

	it('archived workspace → archive (tagged) when not already archived; no-op otherwise', () => {
		expect(decideSessionLifecycle(mk(), 'archived')).toEqual({
			action: 'archive',
			patch: { archived: true, archivedByWorkspace: true }
		})
		expect(decideSessionLifecycle(mk({ archived: true }), 'archived')).toEqual({ action: 'noop' })
	})

	it('active workspace → unarchive only the ones WE archived (archivedByWorkspace)', () => {
		expect(
			decideSessionLifecycle(mk({ archived: true, archivedByWorkspace: true }), 'active')
		).toEqual({
			action: 'unarchive',
			patch: { archived: undefined, archivedByWorkspace: undefined }
		})
		// user-archived (no archivedByWorkspace flag) is left alone
		expect(decideSessionLifecycle(mk({ archived: true }), 'active')).toEqual({ action: 'noop' })
		expect(decideSessionLifecycle(mk(), 'active')).toEqual({ action: 'noop' })
	})

	it('unknown status (workspace absent from the queried set) → no-op, never a delete', () => {
		expect(decideSessionLifecycle(mk(), undefined)).toEqual({ action: 'noop' })
	})
})
