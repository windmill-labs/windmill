import { untrack } from 'svelte'
import { fromStore } from 'svelte/store'
import { SvelteMap } from 'svelte/reactivity'
import { userStore, workspaceStore, type UserExt } from '$lib/stores'
import { getWorkspaceRole, type RoleLookup } from '$lib/user'

/**
 * The user acting in a workspace that is not necessarily the one the top nav points at — an AI
 * session or a workspace-specific variant acts on a workspace the nav deliberately is not on.
 *
 * `$userStore` answers for the navigation workspace at no cost; every other workspace is looked
 * up. An unresolved user is `undefined` and must never fall back to the navigation user, whose
 * rights belong to another workspace — `canWrite`/`isOwner` refuse for an unknown user, which is
 * the only safe answer. A caller that must not render that refusal as a denial asks `resolved`
 * first.
 */
export function useActingUser(workspace: () => string | undefined) {
	const navWorkspace = fromStore(workspaceStore)
	const navUserStore = fromStore(userStore)
	// `switchWorkspace` moves the navigation workspace before the layout refetches the user, so
	// the navigation user answers only once they are that workspace's.
	const navUser = $derived(
		navUserStore.current?.workspace_id === navWorkspace.current ? navUserStore.current : undefined
	)
	// A Map, not an object: a workspace may legitimately be named `constructor`, which a plain
	// object would answer for out of its prototype.
	const looked = new SvelteMap<string, RoleLookup>()
	// The workspace last asked about, so one selection asks once — a failed lookup included,
	// which `getWorkspaceRole` deliberately does not cache. Pointing back at it re-asks.
	let asked: string | undefined

	$effect(() => {
		const ws = workspace()
		if (!ws || ws === navWorkspace.current) {
			asked = undefined
			return
		}
		if (ws === asked || looked.get(ws)?.kind === 'resolved') return
		asked = ws
		untrack(() => {
			// Memoized process-wide, so two components pointed at the same workspace share one
			// request rather than each issuing their own.
			getWorkspaceRole(ws).then((lookup) => looked.set(ws, lookup))
		})
	})

	function userIn(ws: string | undefined): UserExt | undefined {
		if (!ws) return undefined
		if (ws === navWorkspace.current) return navUser
		const lookup = looked.get(ws)
		return lookup?.kind === 'resolved' ? lookup.user : undefined
	}

	return {
		/** The acting user in `ws`, or `undefined` when it is not known. Only workspaces this
		 *  hook has been pointed at are looked up; the rest read as unknown. */
		in: userIn,
		/** Whether `ws` has an answer at all — a user, or a lookup that came back without one. */
		resolved: (ws: string | undefined): boolean =>
			!!ws && (ws === navWorkspace.current ? navUser !== undefined : looked.has(ws)),
		get current(): UserExt | undefined {
			return userIn(workspace())
		}
	}
}
