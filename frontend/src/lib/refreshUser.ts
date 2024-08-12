import { get } from 'svelte/store'
import { UserService } from '$lib/gen'
import { superadmin } from './stores.js'

export async function refreshSuperadmin(
	gotoFn: ((path: string, opt?: Record<string, any> | undefined) => void) | undefined
): Promise<void> {
	if (get(superadmin) == undefined) {
		try {
			const me = await UserService.globalWhoami()
			if (me.super_admin) {
				superadmin.set(me.email)
			} else {
				superadmin.set(false)
			}
		} catch {
			superadmin.set(false)
			gotoFn?.('/user/logout')
		}
	}
}
