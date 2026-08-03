import { beforeEach, describe, expect, it } from 'vitest'

import { hasStoredSdkConsent, storeSdkConsent } from './sdkScopes'

describe('stored frontend SDK consent', () => {
	beforeEach(() => localStorage.clear())

	it('only covers scopes the viewer actually approved', () => {
		storeSdkConsent('a@w.dev', 'ws', 'u/a/app', ['users:read'])
		expect(hasStoredSdkConsent('a@w.dev', 'ws', 'u/a/app', ['users:read'])).toBe(true)
		// The app added a scope after the viewer consented: it must ask again
		// rather than silently minting a broader token.
		expect(hasStoredSdkConsent('a@w.dev', 'ws', 'u/a/app', ['users:read', 'jobs:run'])).toBe(false)
	})

	it('does not leak one viewer or app to another', () => {
		storeSdkConsent('a@w.dev', 'ws', 'u/a/app', ['users:read'])
		expect(hasStoredSdkConsent('b@w.dev', 'ws', 'u/a/app', ['users:read'])).toBe(false)
		expect(hasStoredSdkConsent('a@w.dev', 'ws', 'u/a/other', ['users:read'])).toBe(false)
		expect(hasStoredSdkConsent('a@w.dev', 'other', 'u/a/app', ['users:read'])).toBe(false)
	})

	it('treats unreadable storage as no consent', () => {
		localStorage.setItem('wm_sdk_consent:a@w.dev:ws:u/a/app', 'not json')
		expect(hasStoredSdkConsent('a@w.dev', 'ws', 'u/a/app', ['users:read'])).toBe(false)
	})
})
