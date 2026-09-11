import { describe, it, expect, beforeEach } from 'vitest'
import {
	clearPendingLoginMethod,
	confirmPendingLoginMethod,
	getLastLoginMethod,
	markLoginMethodPending,
	rememberLoginMethod
} from './lastLoginMethod'

describe('lastLoginMethod', () => {
	beforeEach(() => localStorage.clear())

	it('only promotes a pending method when told a session exists', () => {
		markLoginMethodPending({ kind: 'oauth', provider: 'gitlab' })
		expect(getLastLoginMethod()).toBeUndefined()

		confirmPendingLoginMethod()
		expect(getLastLoginMethod()).toEqual({ kind: 'oauth', provider: 'gitlab' })

		// the pending slot is spent, so a later confirm cannot re-promote it
		localStorage.removeItem('lastLoginMethod')
		confirmPendingLoginMethod()
		expect(getLastLoginMethod()).toBeUndefined()
	})

	it('forgets a pending method that never became a login', () => {
		rememberLoginMethod({ kind: 'password' })
		markLoginMethodPending({ kind: 'oauth', provider: 'github' })
		clearPendingLoginMethod()

		confirmPendingLoginMethod()
		expect(getLastLoginMethod()).toEqual({ kind: 'password' })
	})

	it('ignores stored values it does not recognise', () => {
		for (const stored of [
			'not json',
			'{}',
			'"password"',
			'{"kind":"oauth"}',
			'{"kind":"carrier"}'
		]) {
			localStorage.setItem('lastLoginMethod', stored)
			expect(getLastLoginMethod()).toBeUndefined()
		}
	})
})
