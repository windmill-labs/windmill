import { get } from 'svelte/store'
import { UserService } from '$lib/gen'
import { superadmin, devopsRole } from './stores.js'

export async function refreshSuperadmin(): Promise<void> {
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
			// goto('/user/logout')
		}
	}

	if (get(devopsRole) == undefined) {
		try {
			const me = await UserService.globalWhoami()
			if (me.devops || me.super_admin) {
				devopsRole.set(me.email)
			} else {
				devopsRole.set(false)
			}
		} catch {
			devopsRole.set(false)
			// goto('/user/logout')
		}
	}
}
