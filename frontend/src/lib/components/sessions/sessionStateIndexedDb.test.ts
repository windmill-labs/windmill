import { describe, it, expect, beforeEach, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import { openDB } from 'idb'

// sessionState persistence is BROWSER-gated; the vitest "server" env reports false.
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))

// Spy on the attached-file GC so we can assert lifecycle deletes clean it up.
const { deleteItemsForSessionMock } = vi.hoisted(() => ({ deleteItemsForSessionMock: vi.fn() }))
vi.mock('../copilot/chat/files/attachedFilesDB', async (orig) => ({
	...(await orig<typeof import('../copilot/chat/files/attachedFilesDB')>()),
	deleteItemsForSession: deleteItemsForSessionMock
}))

// sessionState imports WorkspaceService; these tests don't touch the network.
vi.mock('$lib/gen', async (orig) => {
	const actual = await orig<typeof import('$lib/gen')>()
	return {
		...actual,
		WorkspaceService: {
			...actual.WorkspaceService,
			listUserWorkspaces: vi.fn().mockResolvedValue([]),
			getSessionWorkspaceStatus: vi.fn().mockResolvedValue({})
		}
	}
})

import { userStore, usersWorkspaceStore, type UserExt } from '$lib/stores'
import { WorkspaceService } from '$lib/gen'
import {
	sessionState,
	putSession,
	deleteSessionRecord,
	archiveSessionsForWorkspace,
	deleteSessionsForWorkspace,
	reconcileSessionsLifecycle,
	setSessionArchived,
	type Session
} from './sessionState.svelte'

function asUser(email: string): UserExt {
	return { email, username: email.split('@')[0] } as unknown as UserExt
}
function session(over: Partial<Session> = {}): Session {
	return { id: 's1', name: 'sess', createdAt: 0, ...over }
}
const flush = () => new Promise<void>((r) => setTimeout(r, 0))

// Each test uses a distinct email: the module-level sessionsDb handle gates its
// legacy migration to once-per-scoped-name for the process, and fresh emails
// also dodge any cross-test cached connection.
let n = 0
function freshUser() {
	return asUser(`u${n++}@x.com`)
}

// Re-hydrate the in-memory list by toggling the user off and on (the only
// public path that triggers sessionState's onUserChange hydration).
async function rehydrate(user: UserExt) {
	userStore.set(undefined)
	await flush()
	userStore.set(user)
	await flush()
}

beforeEach(async () => {
	;(globalThis as any).indexedDB = new IDBFactory()
	localStorage.clear()
	userStore.set(undefined)
	usersWorkspaceStore.set(undefined)
	await flush()
	sessionState.sessions = []
	sessionState.currentSessionId = undefined
})

describe('sessionState IndexedDB persistence', () => {
	it('persists a session and hydrates it back, newest-first by createdAt', async () => {
		const user = freshUser()
		userStore.set(user)
		await flush()

		await putSession(session({ id: 's1', createdAt: 100 }))
		await putSession(session({ id: 's2', createdAt: 200 }))

		await rehydrate(user)
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['s2', 's1']))
	})

	it('does not persist transient sessions', async () => {
		const user = freshUser()
		userStore.set(user)
		await flush()

		await putSession(session({ id: 't1', transient: true }))
		await rehydrate(user)
		await flush()
		expect(sessionState.sessions).toEqual([])
	})

	it('removes a session record', async () => {
		const user = freshUser()
		userStore.set(user)
		await flush()

		await putSession(session({ id: 'keep', createdAt: 1 }))
		await putSession(session({ id: 'drop', createdAt: 2 }))
		await deleteSessionRecord('drop')

		await rehydrate(user)
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['keep']))
	})

	it('isolates sessions between users', async () => {
		const a = freshUser()
		const b = freshUser()

		userStore.set(a)
		await flush()
		await putSession(session({ id: 'a1', createdAt: 1 }))

		// Switch to B: empty list, A's record not visible.
		userStore.set(b)
		await vi.waitFor(() => expect(sessionState.sessions).toEqual([]))
		await putSession(session({ id: 'b1', createdAt: 1 }))

		// Back to A: only A's record.
		userStore.set(a)
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['a1']))
	})

	it("drops the previous user's transient draft on a user change", async () => {
		const a = freshUser()
		const b = freshUser()

		userStore.set(a)
		await flush()
		// A starts an unsent draft — transient, in-memory only, never persisted.
		sessionState.sessions = [session({ id: 'a-draft', transient: true }), ...sessionState.sessions]

		// Switch to B: A's transient must not bleed into B's list (it would
		// otherwise be reused by createSession and inherit A's pending state).
		userStore.set(b)
		await vi.waitFor(() => {
			expect(sessionState.sessions.some((s) => s.id === 'a-draft')).toBe(false)
		})
	})

	it('resets the active session pointer on user switch', async () => {
		const a = freshUser()
		const b = freshUser()
		userStore.set(a)
		await flush()
		sessionState.currentSessionId = 'a1'
		userStore.set(b)
		await vi.waitFor(() => expect(sessionState.currentSessionId).toBeUndefined())
	})

	it('claims legacy localStorage sessions for the first connector, folding watermarks, then deletes the bare keys', async () => {
		usersWorkspaceStore.set({
			email: 'u@x.com',
			workspaces: [
				{ id: 'root', name: 'root', disabled: false },
				{ id: 'fork', name: 'fork', parent_workspace_id: 'root', disabled: false }
			] as never
		})
		localStorage.setItem(
			'windmill_sessions',
			JSON.stringify([
				{ id: 'leg1', name: 'L1', createdAt: 100 },
				// transient legacy entries are not persisted
				{ id: 'leg2', name: 'L2', createdAt: 200, transient: true },
				// legacy '' workspace marker is normalised away
				{ id: 'leg3', name: 'L3', createdAt: 50, workspace_id: '' },
				{ id: 'leg4', name: 'L4', createdAt: 25, workspace_id: 'fork' }
			])
		)
		localStorage.setItem('windmill_sessions_last_seen_counts', JSON.stringify({ leg1: 5 }))

		const user = freshUser()
		userStore.set(user)
		await vi.waitFor(() =>
			expect(sessionState.sessions.map((s) => s.id)).toEqual(['leg1', 'leg3', 'leg4'])
		)

		const leg1 = sessionState.sessions.find((s) => s.id === 'leg1')!
		expect(leg1.lastSeenCount).toBe(5)
		expect(sessionState.sessions.find((s) => s.id === 'leg3')!.workspace_id).toBeUndefined()
		expect(sessionState.sessions.find((s) => s.id === 'leg4')!.workspace_root_id).toBe('root')

		// Bare keys are gone so a later different user does not re-inherit them.
		expect(localStorage.getItem('windmill_sessions')).toBeNull()
		expect(localStorage.getItem('windmill_sessions_last_seen_counts')).toBeNull()
	})

	it('deletes lingering bare keys even when the user DB is already populated', async () => {
		const user = freshUser()
		const dbName = `windmill-sessions::${user.email}`
		// Returning user: their DB already holds a session...
		const seed = await openDB(dbName, 1, {
			upgrade(d) {
				d.createObjectStore('sessions', { keyPath: 'id' })
			}
		})
		await seed.put('sessions' as never, { id: 'existing', name: 'kept', createdAt: 1 } as never)
		seed.close()
		// ...and a partially-failed prior migration left bare keys behind.
		localStorage.setItem(
			'windmill_sessions',
			JSON.stringify([{ id: 'stale', name: 'x', createdAt: 2 }])
		)
		localStorage.setItem('windmill_sessions_last_seen_counts', JSON.stringify({ stale: 3 }))

		userStore.set(user)
		// The existing record is kept; the stale legacy session is NOT claimed
		// (DB already populated) — but the bare keys are still cleaned up.
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['existing']))
		expect(localStorage.getItem('windmill_sessions')).toBeNull()
		expect(localStorage.getItem('windmill_sessions_last_seen_counts')).toBeNull()
	})

	it('archiveSessionsForWorkspace tags only the sessions it archives, preserving user-archived ones', async () => {
		const user = freshUser()
		userStore.set(user)
		await flush()

		// One clean session and one the user already archived by hand, same workspace.
		await putSession(session({ id: 'clean', createdAt: 100, workspace_id: 'wsA' }))
		await putSession(
			session({ id: 'userarch', createdAt: 50, workspace_id: 'wsA', archived: true })
		)
		await rehydrate(user)
		await vi.waitFor(() => expect(sessionState.sessions.length).toBe(2))

		await archiveSessionsForWorkspace('wsA')
		await rehydrate(user)

		// waitFor: rehydrate's re-population settles asynchronously, so re-find
		// inside the retry rather than reading the list once immediately after.
		await vi.waitFor(() => {
			const clean = sessionState.sessions.find((s) => s.id === 'clean')!
			const userarch = sessionState.sessions.find((s) => s.id === 'userarch')!
			// Workspace-archived → tagged, so a later unarchive auto-restores it.
			expect(clean.archived).toBe(true)
			expect(clean.archivedByWorkspace).toBe(true)
			// User-archived → left untouched (no tag), so unarchive won't resurrect it.
			expect(userarch.archived).toBe(true)
			expect(userarch.archivedByWorkspace).toBeUndefined()
		})
	})

	it("GCs each session's attached files when its workspace is torn down", async () => {
		const user = freshUser()
		userStore.set(user)
		await flush()
		await putSession(session({ id: 'f1', createdAt: 1, workspace_id: 'wsX' }))
		await putSession(session({ id: 'f2', createdAt: 2, workspace_id: 'wsX' }))
		await rehydrate(user)
		await vi.waitFor(() => expect(sessionState.sessions.length).toBe(2))

		deleteItemsForSessionMock.mockClear()
		await deleteSessionsForWorkspace('wsX')

		// Both deleted sessions' linked files must be GC'd, not just their records.
		const cleaned = deleteItemsForSessionMock.mock.calls.map((c) => c[0]).sort()
		expect(cleaned).toEqual(['f1', 'f2'])
	})

	it('does not persist a per-session unarchive when the workspace is gone (resurrection guard)', async () => {
		const user = freshUser()
		// Seed the archived session while its workspace is still in the list so the
		// write lands.
		usersWorkspaceStore.set({
			email: user.email,
			workspaces: [{ id: 'gone-ws', name: 'gone', disabled: false }] as never
		})
		userStore.set(user)
		await flush()
		await putSession(session({ id: 'arch', createdAt: 1, workspace_id: 'gone-ws', archived: true }))
		await rehydrate(user)
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['arch']))

		// The workspace disappears from the user's list (archived/deleted elsewhere).
		// A non-empty list that omits it triggers the putSession resurrection guard.
		usersWorkspaceStore.set({
			email: user.email,
			workspaces: [{ id: 'live-ws', name: 'live', disabled: false }] as never
		})
		await flush()

		// Attempting to unarchive in place must NOT persist — this is exactly why the
		// per-session Unarchive control is hidden when the workspace is unavailable.
		setSessionArchived('arch', false)
		await flush()

		// Read straight from IndexedDB: the in-memory list is reassigned by the
		// async reconcile that rehydrate kicks off, so asserting against it races;
		// the DB is the source of truth for whether the unarchive persisted.
		const db = await openDB(`windmill-sessions::${user.email}`, 1)
		const rec = (await db.get('sessions' as never, 'arch')) as Session
		db.close()
		expect(rec.archived).toBe(true)
	})

	it('re-roots a sub-fork session on reconcile when an ancestor was deleted', async () => {
		const user = freshUser()
		// Family after a grandparent deletion: the FK's ON DELETE SET NULL nulled
		// fork-1's parent, so fork-1 is now the topmost member; fork_of_fork still
		// points at fork-1.
		usersWorkspaceStore.set({
			email: user.email,
			workspaces: [
				{ id: 'wm-fork-fork-1', name: 'fork-1', disabled: false },
				{
					id: 'wm-fork-fork_of_fork',
					name: 'fork_of_fork',
					parent_workspace_id: 'wm-fork-fork-1',
					disabled: false
				}
			] as never
		})
		vi.mocked(WorkspaceService.getSessionWorkspaceStatus).mockResolvedValueOnce({
			'wm-fork-fork_of_fork': 'active'
		} as never)

		userStore.set(user)
		await flush()
		// Seed with a STALE root pointing at the now-deleted grandparent.
		await putSession(
			session({
				id: 'sub',
				createdAt: 1,
				workspace_id: 'wm-fork-fork_of_fork',
				workspace_root_id: 'wm-grandparent-deleted'
			})
		)
		await rehydrate(user)
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['sub']))

		await reconcileSessionsLifecycle()
		await rehydrate(user)

		// Re-rooted to the new family topmost member, not left on the dead ancestor.
		// waitFor: rehydrate's re-population settles asynchronously.
		await vi.waitFor(() => {
			const sub = sessionState.sessions.find((s) => s.id === 'sub')!
			expect(sub.workspace_root_id).toBe('wm-fork-fork-1')
		})
	})

	it('clears the in-memory list on logout', async () => {
		const user = freshUser()
		userStore.set(user)
		await flush()
		await putSession(session({ id: 's1', createdAt: 1 }))
		await rehydrate(user)
		await vi.waitFor(() => expect(sessionState.sessions.length).toBe(1))

		userStore.set(undefined)
		await vi.waitFor(() => expect(sessionState.sessions).toEqual([]))
	})
})
