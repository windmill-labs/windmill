import { untrack } from 'svelte'
import { fromStore } from 'svelte/store'
import { userStore, workspaceStore, type UserExt } from '$lib/stores'
import { getUserExt } from '$lib/user'

/**
 * The user acting in a workspace that is not necessarily the one the top nav points at — an AI
 * session or a workspace-specific variant acts on a workspace the nav deliberately is not on.
 *
 * `$userStore` is loaded for the navigation workspace and answers only for that one, so it is
 * returned as-is there and costs no request. Any other workspace is asked once and cached under
 * its own id; keying the cache by workspace is what makes a superseded lookup harmless, since a
 * late answer can only ever land under the question it was asked.
 *
 * An unresolved user is `undefined`, and must never fall back to the navigation user, whose
 * rights belong to another workspace — `canWrite`/`isOwner` refuse for an unknown user, which is
 * the only safe answer. Callers that must not show that refusal as a denial ask `resolved` first.
 */
export function useActingUser(workspace: () => string | undefined) {
	const navWorkspace = fromStore(workspaceStore)
	const navUser = fromStore(userStore)
	// A failed lookup is cached as `undefined` under its key, so it refuses rather than
	// retrying on every read.
	let others: Record<string, UserExt | undefined> = $state({})

	$effect(() => {
		const ws = workspace()
		if (!ws || ws === navWorkspace.current) return
		if (ws in others) return
		untrack(() => {
			getUserExt(ws).then((u) => (others[ws] = u))
		})
	})

	function userIn(ws: string | undefined): UserExt | undefined {
		if (!ws) return undefined
		return ws === navWorkspace.current ? navUser.current : others[ws]
	}

	return {
		/** The acting user in `ws`, or `undefined` when it is not known. Only workspaces this
		 *  hook has been pointed at are looked up; the rest read as unknown. */
		in: userIn,
		/** Whether `ws` has an answer at all — a resolved user, or a lookup that failed. */
		resolved: (ws: string | undefined): boolean =>
			!!ws && (ws === navWorkspace.current || ws in others),
		get current(): UserExt | undefined {
			return userIn(workspace())
		}
	}
}
