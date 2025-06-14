import { goto } from '$lib/navigation'
import { UserService } from '$lib/gen'
import { clearStores } from './storeUtils'
import { sendUserToast } from './toast'

export async function logoutWithRedirect(rd?: string): Promise<void> {
	console.log('logoutWithRedirect', rd)
	await clearUser()
	const splitted = rd?.split('?')[0]
	if (rd && rd != '/' && splitted != '/user/login' && splitted != '/user/logout') {
		const error = document.cookie.includes('token')
			? `error=${encodeURIComponent('You have been logged out because your session has expired.')}&`
			: ''
		console.log('login redirect with error', error, rd)
		goto(`/user/login?${error}${rd ? 'rd=' + encodeURIComponent(rd) : ''}`, { replaceState: true })
	} else {
		console.log('login redirect vanilla')
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
