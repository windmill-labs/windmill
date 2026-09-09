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
	// A lookup that came back empty is remembered only for as long as its workspace is the one
	// being acted on, so the caller has a settled answer to show without the refusal outliving
	// the attempt. Leaving that workspace ends the attempt; so does `forgetFailures`.
	let shown: string | undefined

	$effect(() => {
		const ws = workspace()
		if (shown !== ws) {
			const left = shown
			shown = ws
			if (left && untrack(() => looked.get(left)?.kind) === 'lookup_failed') looked.delete(left)
		}
		if (!ws || ws === navWorkspace.current) return
		// Any settled answer stops the asking, a failure included — otherwise recording one
		// would re-enter this effect and loop.
		if (looked.has(ws)) return
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
		},
		/** Drop the lookups that came back empty so they are asked again — the other way an
		 *  attempt ends. A long-lived editor must call this when it starts a fresh session, or
		 *  a `whoami` that happened to fail pins its workspace to "unknown user" until the
		 *  acting workspace changes. */
		forgetFailures(): void {
			for (const [ws, lookup] of looked) {
				if (lookup.kind === 'lookup_failed') looked.delete(ws)
			}
		}
	}
}
