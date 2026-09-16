import { get } from 'svelte/store'
import { CancelablePromise, CancelError, UserService, type GlobalUserInfo } from '$lib/gen'
import { superadmin, devopsRole } from './stores.js'

let promise: CancelablePromise<GlobalUserInfo> | null = null
/**
 * `force` asks the server even when the stores already hold an answer. Worth it where a wrong
 * answer changes what the page offers rather than how it looks: a logged-out load sets both
 * stores to `false` — the request 401s — and without `force` nothing asks again for the rest
 * of the session, so the user who signs in next reads as neither superadmin nor devops.
 */
async function _refreshSuperadmin(opts?: { force?: boolean }): Promise<void> {
	let shouldFetch = opts?.force || get(superadmin) == undefined || get(devopsRole) == undefined
	if (!shouldFetch) return undefined
	promise?.cancel()
	// Held locally so the check at the end can tell this request from a later caller's, which
	// by then owns `promise`.
	const mine = UserService.globalWhoami()
	promise = mine
	try {
		const me = await mine
		superadmin.set(me.super_admin ? me.email : false)
		devopsRole.set(me.devops || me.super_admin ? me.email : false)
	} catch (error) {
		// A cancellation says nothing about this user, so it must not be written down as an
		// answer: `clearStores` cancels on logout, and a second caller cancels the first — and
		// `false` here is precisely the stale state `force` exists to get out of.
		if (!(error instanceof CancelError)) {
			superadmin.set(false)
			devopsRole.set(false)
			console.error('error refreshing superadmin/devops role', error)
		}
	}
	// Only if nobody has started another: clearing a live request's handle would put it beyond
	// the reach of `cancel()`, and it would then land on a session that had been cleared.
	if (promise === mine) promise = null
}

export const refreshSuperadmin = Object.assign(_refreshSuperadmin, {
	cancel: () => promise?.cancel() as void
})
