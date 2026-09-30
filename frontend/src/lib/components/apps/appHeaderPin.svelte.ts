import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'

// A deployed app is the page, not a page inside Windmill's chrome, so it opens with the header out
// of the way. Pinning brings the header back for good — one flag for every app rather than one per
// app: someone who wants the breadcrumb over a dashboard wants it over the next one too.
const pref = useLocalStorageValue<boolean>('app_header_pinned', false, 'boolean')

/** Whether the page header stays in flow on a deployed app's page. */
export const appHeaderPinned = {
	get val(): boolean {
		return pref.val === true
	},
	set val(pinned: boolean) {
		pref.val = pinned
	}
}
