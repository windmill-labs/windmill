import { noteSessionEmail } from './onboardingProfile'
import { accountSetup } from './components/sidebar/accountSetup.svelte'
import { UserService } from '$lib/gen'
import { clearStores } from './storeUtils'

// Note: logout and logoutWithRedirect have been moved to logoutKit.ts
// as they depend on SvelteKit navigation

export async function clearUser() {
	try {
		noteSessionEmail(undefined)
		accountSetup.reset()
		clearStores()
		await UserService.logout()
	} catch (error) {}
}
