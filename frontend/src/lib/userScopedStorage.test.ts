import { describe, it, expect, beforeEach, vi } from 'vitest'

// The userStore subscription is gated on BROWSER; the vitest "server" env
// reports BROWSER=false.
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))

import { userStore, type UserExt } from '$lib/stores'
import {
	scopedKey,
	getCurrentUserEmail,
	onUserChange,
	migrateLegacyLocalStorage
} from './userScopedStorage'

function asUser(email: string): UserExt {
	return { email, username: email.split('@')[0] } as unknown as UserExt
}

beforeEach(() => {
	localStorage.clear()
	userStore.set(undefined)
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
		localStorage.setItem('ai-chat-autonomy-mode', 'yolo')
		migrateLegacyLocalStorage('ai-chat-autonomy-mode', 'ai-chat-autonomy-mode::a@x.com')
		expect(localStorage.getItem('ai-chat-autonomy-mode::a@x.com')).toBe('yolo')
		expect(localStorage.getItem('ai-chat-autonomy-mode')).toBeNull()
	})

	it('does not overwrite an existing target', () => {
		localStorage.setItem('ai-chat-autonomy-mode', 'yolo')
		localStorage.setItem('ai-chat-autonomy-mode::a@x.com', 'acceptedit')
		migrateLegacyLocalStorage('ai-chat-autonomy-mode', 'ai-chat-autonomy-mode::a@x.com')
		expect(localStorage.getItem('ai-chat-autonomy-mode::a@x.com')).toBe('acceptedit')
		// Legacy left untouched since the target was already populated.
		expect(localStorage.getItem('ai-chat-autonomy-mode')).toBe('yolo')
	})

	it('is a no-op when the target key is undefined (no user)', () => {
		localStorage.setItem('ai-chat-autonomy-mode', 'yolo')
		migrateLegacyLocalStorage('ai-chat-autonomy-mode', undefined)
		expect(localStorage.getItem('ai-chat-autonomy-mode')).toBe('yolo')
	})
})
