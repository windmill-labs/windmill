import { describe, it, expect } from 'vitest'
import {
	INSTANCE_BANNER_MESSAGE_MAX_LEN,
	isInstanceBannerVisible,
	resolveInstanceBanner
} from './instanceBanner'

const BANNER = {
	enabled: true,
	message: 'Scheduled maintenance on Saturday.',
	severity: 'warning',
	dismissible: true
}

describe('resolveInstanceBanner', () => {
	it('drops a link that is not an absolute http(s) URL', () => {
		// Declarative instance config writes global_settings rows directly, so the API's
		// scheme check is not the only thing standing between a stored value and an href.
		for (const link of ['javascript:alert(1)', 'data:text/html,x', 'status.example.com', 123]) {
			expect(resolveInstanceBanner({ ...BANNER, link })?.link).toBeUndefined()
		}
		expect(resolveInstanceBanner({ ...BANNER, link: 'https://status.example.com' })?.link).toBe(
			'https://status.example.com'
		)
	})

	it('keeps a message the backend accepted whole', () => {
		// The backend caps at INSTANCE_BANNER_MESSAGE_MAX_LEN code points (`chars().count()`).
		// Truncating with `slice` here would count UTF-16 units and halve an all-emoji message
		// that passed validation, so the two sides must measure the same way.
		const emoji = '\u{1F6A7}'.repeat(INSTANCE_BANNER_MESSAGE_MAX_LEN)
		expect([...resolveInstanceBanner({ ...BANNER, message: emoji })!.message]).toHaveLength(
			INSTANCE_BANNER_MESSAGE_MAX_LEN
		)
	})

	it('shows nothing when disabled or without a message', () => {
		// An enabled banner with no message is a writable state (the backend accepts it so a
		// half-typed announcement cannot fail an admin's whole settings save), so this is the
		// only thing keeping it off everyone's screen.
		expect(resolveInstanceBanner({ ...BANNER, enabled: false })).toBeUndefined()
		expect(resolveInstanceBanner({ ...BANNER, message: '   ' })).toBeUndefined()
		expect(resolveInstanceBanner({ ...BANNER, message: 42 })).toBeUndefined()
	})
})

describe('isInstanceBannerVisible', () => {
	it('honours a dismissal only while the announcement is dismissible', () => {
		const dismissible = resolveInstanceBanner(BANNER)!
		expect(isInstanceBannerVisible(dismissible, dismissible.fingerprint)).toBe(false)
		expect(isInstanceBannerVisible(dismissible, 'some other announcement')).toBe(true)

		// Escalating the same announcement to mandatory must reach the people who already
		// dismissed it — the fingerprint does not change, so nothing else would bring it back.
		const mandatory = resolveInstanceBanner({ ...BANNER, dismissible: false })!
		expect(mandatory.fingerprint).toBe(dismissible.fingerprint)
		expect(isInstanceBannerVisible(mandatory, mandatory.fingerprint)).toBe(true)
	})
})
