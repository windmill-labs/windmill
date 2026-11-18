import { describe, it, expect, beforeEach } from 'vitest'
import type { UserExt } from '$lib/stores'
import { forgetLastSession, optimisticUser, readSessionHint, rememberUser } from './lastSession'

let cookie = ''
;(globalThis as any).document = {
	get cookie() {
		return cookie
	}
}
const hint = (v: string) => (cookie = `other=1; ${__WM_SESSION_HINT__}=${v}`)

const bob = { email: 'bob@x.dev', username: 'bob', workspace_id: 'demo' } as unknown as UserExt

beforeEach(() => {
	localStorage.clear()
	cookie = ''
})

describe('lastSession', () => {
	it('reads the hint by its own name only', () => {
		expect(readSessionHint('token=abc; x=1')).toBeUndefined()
		expect(readSessionHint(`a=1; ${__WM_SESSION_HINT__}=h1; b=2`)).toBe('h1')
		expect(readSessionHint(`${__WM_SESSION_HINT__}=`)).toBeUndefined()
	})

	it('renders as the remembered user only while the session hint still matches', () => {
		hint('h1')
		rememberUser(bob, 'h1')
		expect(optimisticUser('demo')).toEqual(bob)
		// Another login on the browser: another session, another hint.
		hint('h2')
		expect(optimisticUser('demo')).toBeUndefined()
		// The session ended: no hint at all.
		cookie = ''
		expect(optimisticUser('demo')).toBeUndefined()
	})

	it('only for the workspace it was confirmed in, and never after a logout', () => {
		hint('h1')
		rememberUser(bob, 'h1')
		expect(optimisticUser('other')).toBeUndefined()
		forgetLastSession()
		expect(optimisticUser('demo')).toBeUndefined()
	})

	it('remembers nothing without a session hint', () => {
		rememberUser(bob, 'h1')
		hint('h1')
		expect(optimisticUser('demo')).toBeUndefined()
	})

	it("never files a whoami answer under a session that changed while it was in flight", () => {
		hint('alice')
		const sentWith = readSessionHint()
		// Another tab logs in as bob before alice's whoami answers.
		hint('bob')
		rememberUser({ ...bob, email: 'alice@x.dev' } as UserExt, sentWith)
		expect(optimisticUser('demo')).toBeUndefined()
	})
})
