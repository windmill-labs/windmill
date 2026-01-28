import { get } from 'svelte/store'
import { CancelablePromise, UserService, type GlobalUserInfo } from '$lib/gen'
import { superadmin, devopsRole } from './stores.js'

let promise: CancelablePromise<GlobalUserInfo> | null = null
async function _refreshSuperadmin(): Promise<void> {
	let shouldFetch = get(superadmin) == undefined || get(devopsRole) == undefined
	if (!shouldFetch) return undefined
	promise?.cancel()
	promise = UserService.globalWhoami()
	try {
		const me = await promise
		superadmin.set(me.super_admin ? me.email : false)
		devopsRole.set(me.devops || me.super_admin ? me.email : false)
	} catch (error) {
		superadmin.set(false)
		devopsRole.set(false)
		console.error('error refreshing superadmin/devops role', error)
	}
	promise = null
}

export const refreshSuperadmin = Object.assign(_refreshSuperadmin, {
	cancel: () => promise?.cancel() as void
})
