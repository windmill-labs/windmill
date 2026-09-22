import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('./onboardingProfile', () => ({ noteSessionEmail: vi.fn() }))
vi.mock('./components/sidebar/accountSetup.svelte', () => ({
	accountSetup: { reset: vi.fn() }
}))
vi.mock('./storeUtils', () => ({ clearStores: vi.fn() }))
vi.mock('$lib/gen', () => ({ UserService: { logout: vi.fn(async () => {}) } }))

import { clearUser } from './logout'

describe('clearUser', () => {
	beforeEach(() => {
		sessionStorage.clear()
		vi.clearAllMocks()
	})

	// Logging out while impersonating must not leave the impersonator's own token behind.
	it('drops the impersonation token backup', async () => {
		sessionStorage.setItem('pre_impersonation_token', 'a-token')
		sessionStorage.setItem('pre_impersonation_email', 'admin@windmill.dev')

		await clearUser()

		expect(sessionStorage.getItem('pre_impersonation_token')).toBeNull()
		expect(sessionStorage.getItem('pre_impersonation_email')).toBeNull()
	})
})
