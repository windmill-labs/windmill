import { describe, it, expect } from 'vitest'
import { sidebarPageAllowed } from './operatorRoutes'
import type { UserWorkspace } from '$lib/stores'

const ws = (operator_settings: any): UserWorkspace =>
	({ id: 'w', name: 'w', username: 'u', operator_settings }) as UserWorkspace

describe('sidebarPageAllowed', () => {
	it('shows every page to anyone who is not an operator', () => {
		expect(sidebarPageAllowed(false, ws(null), 'runs')).toBe(true)
		expect(sidebarPageAllowed(undefined, undefined, 'runs')).toBe(true)
	})

	it('admits an operator only where the workspace turned the page on', () => {
		expect(sidebarPageAllowed(true, ws({ runs: true }), 'runs')).toBe(true)
		expect(sidebarPageAllowed(true, ws({ runs: false }), 'runs')).toBe(false)
	})

	it('withholds a page the settings do not mention', () => {
		// `=== true`, not truthiness: a right absent from the jsonb, or settings that were never
		// written, must not read as granted.
		expect(sidebarPageAllowed(true, ws({ variables: true }), 'runs')).toBe(false)
		expect(sidebarPageAllowed(true, ws(null), 'runs')).toBe(false)
		expect(sidebarPageAllowed(true, undefined, 'runs')).toBe(false)
	})
})
