import { goto } from '$app/navigation'
import { UserService } from '$lib/gen'
import { clearStores } from './stores.js'
import { sendUserToast } from './utils.js'

function clearCookies() {
	document.cookie.split(";").forEach(function (c) { document.cookie = c.replace(/^ +/, "").replace(/=.*/, "=;expires=" + new Date().toUTCString() + ";path=/") })
}
export function logoutWithRedirect(rd?: string): void {
	const error = document.cookie.includes('token')
		? `error=${encodeURIComponent('You have been logged out because your session has expired.')}&`
		: ''
	clearCookies()
	goto(`/user/login?${error}${rd ? 'rd=' + encodeURIComponent(rd) : ''}`)
}

export async function logout(logoutMessage?: string): Promise<void> {
	try {
		clearStores()
		await UserService.logout()
		clearCookies()
		goto(`/user/login${logoutMessage ? '?error=' + encodeURIComponent(logoutMessage) : ''}`)
		sendUserToast('you have been logged out')
	} catch (error) {
		goto('/user/login')
		console.error(error)
	}
}
