import { describe, it, expect } from 'vitest'
import { workspaceMenuHref } from './workspaceMenuHref'

describe('workspaceMenuHref', () => {
	it('on a session route, keeps the workspace id (so new-tab lands in the right workspace)', () => {
		expect(
			workspaceMenuHref({
				routeId: '/(root)/(logged)/sessions',
				base: '',
				pathname: '/sessions',
				searchParams: new URLSearchParams('session_name=foo'),
				id: 'wm-fork-bar'
			})
		).toBe('/?workspace=wm-fork-bar')
	})

	it('respects the base prefix on a session route', () => {
		expect(
			workspaceMenuHref({
				routeId: '/(root)/(logged)/sessions',
				base: '/wm',
				pathname: '/wm/sessions',
				searchParams: new URLSearchParams(),
				id: 'ws2'
			})
		).toBe('/wm/?workspace=ws2')
	})

	it('off a session route, swaps the workspace param on the current path', () => {
		expect(
			workspaceMenuHref({
				routeId: '/(root)/(logged)/scripts/edit/[...path]',
				base: '',
				pathname: '/scripts/edit/u/me/x',
				searchParams: new URLSearchParams('workspace=old&foo=1'),
				id: 'new_ws'
			})
		).toBe('/scripts/edit/u/me/x?workspace=new_ws&foo=1')
	})

	it('adds the workspace param when none was present', () => {
		expect(
			workspaceMenuHref({
				routeId: '/(root)/(logged)/runs',
				base: '',
				pathname: '/runs',
				searchParams: new URLSearchParams(),
				id: 'w'
			})
		).toBe('/runs?workspace=w')
	})
})
