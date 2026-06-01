import { describe, it, expect } from 'vitest'
import { navStaysInEditor } from './editorNav'

describe('navStaysInEditor', () => {
	const ADD = '/flows/add'
	const EDIT = '/flows/edit/'

	it('preserves on the add → edit promotion', () => {
		expect(navStaysInEditor('/flows/add', '/flows/edit/u/admin/foo', ADD, EDIT)).toBe(true)
	})

	it('preserves when staying on the same entity (e.g. a query change)', () => {
		expect(navStaysInEditor('/flows/edit/u/admin/foo', '/flows/edit/u/admin/foo', ADD, EDIT)).toBe(
			true
		)
	})

	it('clears when navigating to a different entity', () => {
		expect(navStaysInEditor('/flows/edit/u/admin/foo', '/flows/edit/u/admin/bar', ADD, EDIT)).toBe(
			false
		)
	})

	it('clears cross-entity even when editor state would be undefined (the bug)', () => {
		// The old signature used the open entity's path as the promotion signal,
		// which is undefined for a new/draft flow — making this wrongly preserve.
		// Deciding from `from` (the real current route) fixes it: a draft flow's
		// edit route is still a concrete `/flows/edit/{path}`.
		expect(
			navStaysInEditor('/flows/edit/u/admin/draftflow', '/flows/edit/u/admin/other', ADD, EDIT)
		).toBe(false)
	})

	it('clears when leaving the editor entirely', () => {
		expect(navStaysInEditor('/flows/edit/u/admin/foo', '/', ADD, EDIT)).toBe(false)
		expect(
			navStaysInEditor('/flows/edit/u/admin/foo', '/scripts/edit/u/admin/foo', ADD, EDIT)
		).toBe(false)
		expect(navStaysInEditor('/flows/edit/u/admin/foo', '/flows/get/u/admin/foo', ADD, EDIT)).toBe(
			false
		)
	})

	it('does not treat a prefixed-but-different route as the editor', () => {
		expect(navStaysInEditor('/flows/add', '/flows/edit-history/foo', ADD, EDIT)).toBe(false)
	})

	it('works for the raw-app and script add/edit routes too', () => {
		expect(
			navStaysInEditor(
				'/apps_raw/add',
				'/apps_raw/edit/u/admin/a',
				'/apps_raw/add',
				'/apps_raw/edit/'
			)
		).toBe(true)
		expect(
			navStaysInEditor(
				'/apps_raw/edit/u/admin/a',
				'/apps_raw/edit/u/admin/b',
				'/apps_raw/add',
				'/apps_raw/edit/'
			)
		).toBe(false)
		expect(
			navStaysInEditor(
				'/scripts/edit/u/admin/s',
				'/scripts/edit/u/admin/s',
				'/scripts/add',
				'/scripts/edit/'
			)
		).toBe(true)
	})
})
