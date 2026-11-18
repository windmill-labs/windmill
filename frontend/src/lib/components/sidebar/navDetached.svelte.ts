import { fromStore } from 'svelte/store'
import { roleCapabilities } from '$lib/stores'
import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'

// One preference per role rather than one per account. The same person is both — an operator in
// one workspace, a developer in the next — and the two want different things from the sidebar, so
// a single key would carry whichever was pressed last into the other role and never give it back.
// Empty until that role presses detach or attach once.
const operatorPref = useLocalStorageValue<string>('nav_detached_operator', '', 'string')
const devPref = useLocalStorageValue<string>('nav_detached_dev', '', 'string')
const caps = fromStore(roleCapabilities)

/** Whether the sidebar is detached: hidden behind a floating button that slides it in over the page. */
export const navDetached = {
	get val(): boolean {
		// Unasked, operators start detached: many of them navigate through a workspace's own app,
		// and a docked sidebar would take its width.
		const operator = !caps.current.code
		const pref = operator ? operatorPref : devPref
		if (pref.val === 'true') return true
		if (pref.val === 'false') return false
		return operator
	},
	set val(detached: boolean) {
		const pref = caps.current.code ? devPref : operatorPref
		pref.val = String(detached)
	}
}
