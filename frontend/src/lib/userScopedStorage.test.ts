import { describe, it, expect, beforeEach, vi } from 'vitest'

// userScopedStorage's subscription and the sessionState persistence paths are
// gated on BROWSER; the vitest "server" env reports BROWSER=false.
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))

// sessionState pulls in WorkspaceService; we only exercise local persistence.
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
import {
	scopedKey,
	getCurrentUserEmail,
	onUserChange,
	migrateLegacyLocalStorage
} from './userScopedStorage'
import {
	sessionState,
	persistSessions,
	type Session
} from './components/sessions/sessionState.svelte'

function asUser(email: string): UserExt {
	return { email, username: email.split('@')[0] } as unknown as UserExt
}

function session(over: Partial<Session> = {}): Session {
	return { id: 's1', name: 'sess', createdAt: 0, ...over }
}

beforeEach(() => {
	localStorage.clear()
	userStore.set(undefined)
	sessionState.sessions = []
	sessionState.currentSessionId = undefined
})

describe('scopedKey / getCurrentUserEmail', () => {
	it('returns undefined while no user is logged in', () => {
		expect(getCurrentUserEmail()).toBeUndefined()
		expect(scopedKey('windmill_sessions')).toBeUndefined()
	})

	it('namespaces by email and isolates distinct users', () => {
		userStore.set(asUser('a@x.com'))
		expect(scopedKey('windmill_sessions')).toBe('windmill_sessions::a@x.com')
		userStore.set(asUser('b@y.com'))
		expect(scopedKey('windmill_sessions')).toBe('windmill_sessions::b@y.com')
	})
})

describe('onUserChange', () => {
	it('fires immediately with the current email, then on every change', () => {
		userStore.set(asUser('a@x.com'))
		const calls: Array<[string | undefined, string | undefined]> = []
		onUserChange((email, prev) => calls.push([email, prev]))
		// Immediate fire with current email, prev undefined.
		expect(calls).toEqual([['a@x.com', undefined]])
		userStore.set(asUser('b@y.com'))
		expect(calls).toEqual([
			['a@x.com', undefined],
			['b@y.com', 'a@x.com']
		])
		// No-op when the email is unchanged.
		userStore.set(asUser('b@y.com'))
		expect(calls).toHaveLength(2)
	})
})

describe('migrateLegacyLocalStorage', () => {
	it('claims a legacy key into the target and deletes the legacy copy', () => {
		localStorage.setItem('windmill_sessions', '[{"id":"x"}]')
		migrateLegacyLocalStorage('windmill_sessions', 'windmill_sessions::a@x.com')
		expect(localStorage.getItem('windmill_sessions::a@x.com')).toBe('[{"id":"x"}]')
		expect(localStorage.getItem('windmill_sessions')).toBeNull()
	})

	it('does not overwrite an existing target', () => {
		localStorage.setItem('windmill_sessions', 'legacy')
		localStorage.setItem('windmill_sessions::a@x.com', 'mine')
		migrateLegacyLocalStorage('windmill_sessions', 'windmill_sessions::a@x.com')
		expect(localStorage.getItem('windmill_sessions::a@x.com')).toBe('mine')
		// Legacy left untouched since the target was already populated.
		expect(localStorage.getItem('windmill_sessions')).toBe('legacy')
	})

	it('is a no-op when the target key is undefined (no user)', () => {
		localStorage.setItem('windmill_sessions', 'legacy')
		migrateLegacyLocalStorage('windmill_sessions', undefined)
		expect(localStorage.getItem('windmill_sessions')).toBe('legacy')
	})
})

describe('sessionState user scoping', () => {
	it('persists under the email-namespaced key and isolates users', () => {
		userStore.set(asUser('a@x.com'))
		sessionState.sessions = [session({ id: 'a1', name: 'A session' })]
		persistSessions()
		expect(localStorage.getItem('windmill_sessions::a@x.com')).toContain('a1')

		// Switching user hydrates B's (empty) list and resets the active session;
		// A's data stays under A's key, invisible to B.
		userStore.set(asUser('b@y.com'))
		expect(sessionState.sessions).toEqual([])
		expect(localStorage.getItem('windmill_sessions::b@y.com')).toBeNull()

		sessionState.sessions = [session({ id: 'b1', name: 'B session' })]
		persistSessions()
		expect(localStorage.getItem('windmill_sessions::b@y.com')).toContain('b1')
		expect(localStorage.getItem('windmill_sessions::a@x.com')).toContain('a1')

		// Back to A: their list is restored intact.
		userStore.set(asUser('a@x.com'))
		expect(sessionState.sessions.map((s) => s.id)).toEqual(['a1'])
	})

	it('claims legacy un-namespaced sessions for the first user to log in', () => {
		localStorage.setItem('windmill_sessions', JSON.stringify([session({ id: 'legacy1' })]))
		userStore.set(asUser('a@x.com'))
		expect(sessionState.sessions.map((s) => s.id)).toEqual(['legacy1'])
		expect(localStorage.getItem('windmill_sessions::a@x.com')).toContain('legacy1')
		// Legacy key is removed so a later different user does not re-claim it.
		expect(localStorage.getItem('windmill_sessions')).toBeNull()
	})

	it('does not persist while no user is known', () => {
		sessionState.sessions = [session({ id: 'orphan' })]
		persistSessions()
		expect(localStorage.getItem('windmill_sessions')).toBeNull()
		expect(localStorage.length).toBe(0)
	})
})
