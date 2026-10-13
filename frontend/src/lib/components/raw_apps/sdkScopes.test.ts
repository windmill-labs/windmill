import { beforeEach, describe, expect, it } from 'vitest'

import { hasStoredSdkConsent, movePreviewSdkConsent, storeSdkConsent } from './sdkScopes'

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

	it('keeps the editor preview and the deployed app apart, and moves preview consent', () => {
		storeSdkConsent('a@w.dev', 'ws', 'u/a/app', ['users:read'])
		expect(hasStoredSdkConsent('a@w.dev', 'ws', 'u/a/app', ['users:read'], true)).toBe(false)
		storeSdkConsent('a@w.dev', 'ws', 'u/a/draft_1', ['users:read'], true)
		expect(hasStoredSdkConsent('a@w.dev', 'ws', 'u/a/draft_1', ['users:read'])).toBe(false)

		movePreviewSdkConsent('a@w.dev', 'ws', 'u/a/draft_1', 'u/a/real')
		expect(hasStoredSdkConsent('a@w.dev', 'ws', 'u/a/real', ['users:read'], true)).toBe(true)
		expect(hasStoredSdkConsent('a@w.dev', 'ws', 'u/a/draft_1', ['users:read'], true)).toBe(false)
	})

	it('treats unreadable storage as no consent', () => {
		localStorage.setItem('wm_sdk_consent:a@w.dev:ws:u/a/app', 'not json')
		expect(hasStoredSdkConsent('a@w.dev', 'ws', 'u/a/app', ['users:read'])).toBe(false)
	})
})
