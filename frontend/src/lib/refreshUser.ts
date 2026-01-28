import { get } from 'svelte/store'
import { CancelablePromise, UserService } from '$lib/gen'
import { superadmin, devopsRole } from './stores.js'
import { CancelablePromiseUtils } from './cancelable-promise-utils.js'

let promise: CancelablePromise<void> | null = null
function _refreshSuperadmin(): CancelablePromise<void> {
	let shouldFetch = get(superadmin) == undefined || get(devopsRole) == undefined
	if (!shouldFetch) return CancelablePromiseUtils.pure<void>(undefined)
	promise?.cancel()
	promise = CancelablePromiseUtils.then(UserService.globalWhoami(), (me) => {
		superadmin.set(me.super_admin ? me.email : false)
		devopsRole.set(me.devops || me.super_admin ? me.email : false)
		return CancelablePromiseUtils.pure<void>(undefined)
	})
	promise = CancelablePromiseUtils.catchErr(promise, (error) => {
		superadmin.set(false)
		devopsRole.set(false)
		console.error('error refreshing superadmin/devops role', error)
		return CancelablePromiseUtils.pure<void>(undefined)
	})
	return CancelablePromiseUtils.finallyDo(promise, () => (promise = null))
}

export const refreshSuperadmin = Object.assign(_refreshSuperadmin, {
	cancel: () => promise?.cancel() as void
})
