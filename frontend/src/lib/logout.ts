import { goto } from '$app/navigation'
import { UserService } from '$lib/gen'
import { clearStores } from './stores.js'
import { sendUserToast } from './utils.js'

function clearCookies() {
	document.cookie.split(";").forEach(function (c) { document.cookie = c.replace(/^ +/, "").replace(/=.*/, "=;expires=" + new Date().toUTCString() + ";path=/") })
}
export function logoutWithRedirect(rd?: string): void {
	if (rd?.split('?')[0] != '/user/login') {
		const error = document.cookie.includes('token')
			? `error=${encodeURIComponent('You have been logged out because your session has expired.')}&`
			: ''
		clearCookies()
		goto(`/user/login?${error}${rd ? 'rd=' + encodeURIComponent(rd) : ''}`)
	}
}

export async function logout(): Promise<void> {
	try {
		clearStores()
		await UserService.logout()
		clearCookies()
	} catch (error) {
		console.error(error)
		clearCookies()
	}
	goto(`/user/login`)
	sendUserToast('you have been logged out')
}
