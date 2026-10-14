import { noteSessionEmail } from './onboardingProfile'
import { accountSetup } from './components/sidebar/accountSetup.svelte'
import { UserService } from '$lib/gen'
import { clearStores } from './storeUtils'

// Note: logout and logoutWithRedirect have been moved to logoutKit.ts
// as they depend on SvelteKit navigation

// Every browser-storage entry that can hold a credential must go on logout. The impersonation
// backup is the impersonator's own auth token; `oauth-callback` and `mcp-oauth-callback` carry a
// connected account's token response when the OAuth popup had no opener to post it to;
// `test_dev_token` is the session token of the /test_dev pages.
function clearCredentialsFromStorage(): void {
	try {
		sessionStorage.removeItem('pre_impersonation_token')
		sessionStorage.removeItem('pre_impersonation_email')
		localStorage.removeItem('oauth-callback')
		localStorage.removeItem('mcp-oauth-callback')
		localStorage.removeItem('test_dev_token')
	} catch (e) {
		console.error('error interacting with browser storage', e)
	}
}

export async function clearUser() {
	try {
		noteSessionEmail(undefined)
		accountSetup.reset()
		clearStores()
		clearCredentialsFromStorage()
		await UserService.logout()
	} catch (error) {}
}
