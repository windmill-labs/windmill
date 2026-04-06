import { describe, expect, it } from 'vitest'
import { shouldHideAppEditButton } from './viewer'

describe('shouldHideAppEditButton', () => {
	it('hides the edit button when the query param is enabled', () => {
		expect(shouldHideAppEditButton(new URLSearchParams('hideEditBtn=true'), undefined)).toBe(true)
	})

	it('hides the edit button when the app config enables it', () => {
		expect(shouldHideAppEditButton(new URLSearchParams(), { hideEditButton: true })).toBe(true)
	})

	it('does not hide the edit button when both controls are disabled', () => {
		expect(shouldHideAppEditButton(new URLSearchParams(), { hideEditButton: false })).toBe(false)
	})

	it('keeps the app-level setting when the query param is explicitly false', () => {
		expect(
			shouldHideAppEditButton(new URLSearchParams('hideEditBtn=false'), { hideEditButton: true })
		).toBe(true)
	})
})
