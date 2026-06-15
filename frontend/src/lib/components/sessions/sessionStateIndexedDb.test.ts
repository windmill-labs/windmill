import { describe, it, expect, beforeEach, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'

// sessionState persistence is BROWSER-gated; the vitest "server" env reports false.
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))

// sessionState imports WorkspaceService; these tests don't touch the network.
vi.mock('$lib/gen', async (orig) => {
	const actual = await orig<typeof import('$lib/gen')>()
	return {
		...actual,
		WorkspaceService: {
			...actual.WorkspaceService,
			listUserWorkspaces: vi.fn().mockResolvedValue([])
		}
	}
})

import { userStore, type UserExt } from '$lib/stores'
import { sessionState, putSession, deleteSessionRecord, type Session } from './sessionState.svelte'

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
		localStorage.setItem(
			'windmill_sessions',
			JSON.stringify([
				{ id: 'leg1', name: 'L1', createdAt: 100 },
				// transient legacy entries are not persisted
				{ id: 'leg2', name: 'L2', createdAt: 200, transient: true },
				// legacy '' workspace marker is normalised away
				{ id: 'leg3', name: 'L3', createdAt: 50, workspace_id: '' }
			])
		)
		localStorage.setItem('windmill_sessions_last_seen_counts', JSON.stringify({ leg1: 5 }))

		const user = freshUser()
		userStore.set(user)
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['leg1', 'leg3']))

		const leg1 = sessionState.sessions.find((s) => s.id === 'leg1')!
		expect(leg1.lastSeenCount).toBe(5)
		expect(sessionState.sessions.find((s) => s.id === 'leg3')!.workspace_id).toBeUndefined()

		// Bare keys are gone so a later different user does not re-inherit them.
		expect(localStorage.getItem('windmill_sessions')).toBeNull()
		expect(localStorage.getItem('windmill_sessions_last_seen_counts')).toBeNull()
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
