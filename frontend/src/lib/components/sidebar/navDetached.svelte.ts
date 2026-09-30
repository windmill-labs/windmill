import { fromStore } from 'svelte/store'
import { userStore } from '$lib/stores'
import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'

// Empty until the user presses detach or attach once. Until then operators start detached:
// many of them navigate through a workspace's own app, and a docked sidebar would take its width.
const pref = useLocalStorageValue<string>('nav_detached', '', 'string')
const user = fromStore(userStore)

/** Whether the sidebar is detached: hidden behind a floating button that slides it in over the page. */
export const navDetached = {
	get val(): boolean {
		if (pref.val === 'true') return true
		if (pref.val === 'false') return false
		return !!user.current?.operator
	},
	set val(detached: boolean) {
		pref.val = String(detached)
	}
}
