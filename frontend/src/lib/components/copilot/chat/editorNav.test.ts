import { describe, it, expect } from 'vitest'
import { navStaysInEditor } from './editorNav'

describe('navStaysInEditor', () => {
	const FLOW = '/flows/edit/'

	it('preserves on the add → edit promotion (no current path yet)', () => {
		// On /flows/add the entity has no path; the first save navigates to
		// /flows/edit/{path}. Both undefined and '' must count as "promotion".
		expect(navStaysInEditor('/flows/edit/u/admin/foo', FLOW, undefined)).toBe(true)
		expect(navStaysInEditor('/flows/edit/u/admin/foo', FLOW, '')).toBe(true)
	})

	it('preserves when staying on the same entity', () => {
		expect(navStaysInEditor('/flows/edit/u/admin/foo', FLOW, 'u/admin/foo')).toBe(true)
	})

	it('clears when navigating to a different entity', () => {
		expect(navStaysInEditor('/flows/edit/u/admin/bar', FLOW, 'u/admin/foo')).toBe(false)
	})

	it('clears when leaving the editor entirely', () => {
		expect(navStaysInEditor('/', FLOW, 'u/admin/foo')).toBe(false)
		expect(navStaysInEditor('/scripts/edit/u/admin/foo', FLOW, 'u/admin/foo')).toBe(false)
		expect(navStaysInEditor('/flows/get/u/admin/foo', FLOW, 'u/admin/foo')).toBe(false)
	})

	it('does not treat a prefixed-but-different route as the editor', () => {
		// '/flows/edit-history/...' must not be mistaken for '/flows/edit/...'
		expect(navStaysInEditor('/flows/edit-history/foo', FLOW, undefined)).toBe(false)
	})

	it('works for the raw-app and script prefixes too', () => {
		expect(navStaysInEditor('/apps_raw/edit/u/admin/a', '/apps_raw/edit/', '')).toBe(true)
		expect(navStaysInEditor('/apps_raw/edit/u/admin/b', '/apps_raw/edit/', 'u/admin/a')).toBe(false)
		expect(navStaysInEditor('/scripts/edit/u/admin/s', '/scripts/edit/', 'u/admin/s')).toBe(true)
	})
})
