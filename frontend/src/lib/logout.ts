import { goto } from '$lib/navigation'
import { UserService } from '$lib/gen'
import { clearStores } from './storeUtils'
import { sendUserToast } from './toast'

export async function logoutWithRedirect(rd?: string): Promise<void> {
	await clearUser()
	if (rd && rd != '/' && rd?.split('?')[0] != '/user/login') {
		const error = document.cookie.includes('token')
			? `error=${encodeURIComponent('You have been logged out because your session has expired.')}&`
			: ''
		goto(`/user/login?${error}${rd ? 'rd=' + encodeURIComponent(rd) : ''}`, { replaceState: true })
	} else {
		goto('/user/login', { replaceState: true })
	}
}

export async function logout(): Promise<void> {
	await clearUser()
	goto(`/user/login`)
	sendUserToast('you have been logged out')
}

export async function clearUser() {
	try {
		clearStores()
		await UserService.logout()
	} catch (error) {}
}
