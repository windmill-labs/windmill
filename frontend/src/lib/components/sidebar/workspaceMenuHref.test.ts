import { describe, it, expect } from 'vitest'
import { workspaceMenuHref } from './workspaceMenuHref'

describe('workspaceMenuHref', () => {
	it('keeps the open session for a same-family target (new-tab stays on the chat)', () => {
		expect(
			workspaceMenuHref({
				pathname: '/sessions',
				searchParams: new URLSearchParams('session_name=foo'),
				id: 'wm-fork-bar',
				sameFamily: true
			})
		).toBe('/sessions?session_name=foo&workspace=wm-fork-bar')
	})

	it('drops the open session for a cross-family target', () => {
		expect(
			workspaceMenuHref({
				pathname: '/sessions',
				searchParams: new URLSearchParams('session_name=foo'),
				id: 'other-root',
				sameFamily: false
			})
		).toBe('/sessions?workspace=other-root')
	})

	it('swaps the workspace param on the current path', () => {
		expect(
			workspaceMenuHref({
				pathname: '/scripts/edit/u/me/x',
				searchParams: new URLSearchParams('workspace=old&foo=1'),
				id: 'new_ws',
				sameFamily: false
			})
		).toBe('/scripts/edit/u/me/x?workspace=new_ws&foo=1')
	})

	it('adds the workspace param when none was present', () => {
		expect(
			workspaceMenuHref({
				pathname: '/runs',
				searchParams: new URLSearchParams(),
				id: 'w',
				sameFamily: true
			})
		).toBe('/runs?workspace=w')
	})
})
