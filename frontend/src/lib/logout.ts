import { noteSessionEmail } from './onboardingProfile'
import { accountSetup } from './components/sidebar/accountSetup.svelte'
import { UserService } from '$lib/gen'
import { clearStores } from './storeUtils'

// Note: logout and logoutWithRedirect have been moved to logoutKit.ts
// as they depend on SvelteKit navigation

// The impersonation backup holds the impersonator's own auth token, so it must go on
// logout too, not only when impersonation is ended from the banner — otherwise a usable
// token stays readable for the lifetime of the tab.
function clearImpersonationFromStorage(): void {
	try {
		sessionStorage.removeItem('pre_impersonation_token')
		sessionStorage.removeItem('pre_impersonation_email')
	} catch (e) {
		console.error('error interacting with session storage', e)
	}
}

export async function clearUser() {
	try {
		noteSessionEmail(undefined)
		accountSetup.reset()
		clearStores()
		clearImpersonationFromStorage()
		await UserService.logout()
	} catch (error) {}
}
