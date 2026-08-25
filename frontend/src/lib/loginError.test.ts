import { describe, it, expect } from 'vitest'
import { loginErrorMessage } from './loginError'

describe('loginErrorMessage', () => {
	it('maps the backend rejection to one message for a wrong email and a wrong password', () => {
		expect(loginErrorMessage({ status: 400, body: 'Bad request: Invalid login' })).toBe(
			'Invalid email or password.'
		)
	})

	it('surfaces the messages the endpoint is known to produce', () => {
		expect(
			loginErrorMessage({
				status: 400,
				body: 'Bad request: Password login is disabled on this instance'
			})
		).toBe('Password login is disabled on this instance')
		// The rate limiter's own wording is not echoed back: a 429 always gets this sentence.
		expect(loginErrorMessage({ status: 429, body: 'Bad request: slow down' })).toBe(
			'Too many login attempts. Please try again later.'
		)
	})

	it('never surfaces server text it does not recognise, to an unauthenticated visitor', () => {
		const sqlError = {
			status: 400,
			body: 'Bad request: SqlErr: error returned from database: relation "password" does not exist @backend/windmill-api-users/src/users.rs:123'
		}
		expect(loginErrorMessage(sqlError)).toBe('Could not sign you in. Please try again.')
		expect(
			loginErrorMessage({ status: 502, body: '<html><body>502 Bad Gateway</body></html>' })
		).toBe('Could not sign you in. Please try again.')
		expect(loginErrorMessage(new TypeError('Failed to fetch'))).toBe(
			'Could not sign you in. Please try again.'
		)
		expect(loginErrorMessage({ status: 400, body: { error: { message: { nested: true } } } })).toBe(
			'Could not sign you in. Please try again.'
		)
	})
})
