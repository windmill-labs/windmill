import { goto } from '$app/navigation'
import { UserService } from '$lib/gen'
import { clearStores } from './stores.js'
import { sendUserToast } from './utils.js'

export function logoutWithRedirect(rd?: string): void {
	const error = encodeURIComponent('You have been logged out because your session has expired.')
	goto(`/user/login?error=${error}${rd ? '&rd=' + encodeURIComponent(rd) : ''}`)
}

export async function logout(logoutMessage?: string): Promise<void> {
	try {
		clearStores()
		await UserService.logout()
		goto(`/user/login${logoutMessage ? '?error=' + encodeURIComponent(logoutMessage) : ''}`)
		sendUserToast('you have been logged out')
	} catch (error) {
		goto(
			`/user/login?error=${encodeURIComponent(
				'There was a problem logging you out, check the logs'
			)}`
		)
		console.error(error)
	}
}
