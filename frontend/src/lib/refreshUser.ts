import { get } from 'svelte/store'
import { UserService } from '$lib/gen'
import { superadmin } from './stores.js'
import { goto } from '$app/navigation'

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
			goto('/user/logout')
		}
	}
}
