import { describe, it, expect } from 'vitest'
import { workspaceMenuHref } from './workspaceMenuHref'

describe('workspaceMenuHref', () => {
	it('on a session route, keeps the session and adds the workspace id (new-tab stays in session mode)', () => {
		expect(
			workspaceMenuHref({
				pathname: '/sessions',
				searchParams: new URLSearchParams('session_name=foo'),
				id: 'wm-fork-bar'
			})
		).toBe('/sessions?session_name=foo&workspace=wm-fork-bar')
	})

	it('swaps the workspace param on the current path', () => {
		expect(
			workspaceMenuHref({
				pathname: '/scripts/edit/u/me/x',
				searchParams: new URLSearchParams('workspace=old&foo=1'),
				id: 'new_ws'
			})
		).toBe('/scripts/edit/u/me/x?workspace=new_ws&foo=1')
	})

	it('adds the workspace param when none was present', () => {
		expect(
			workspaceMenuHref({
				pathname: '/runs',
				searchParams: new URLSearchParams(),
				id: 'w'
			})
		).toBe('/runs?workspace=w')
	})
})
