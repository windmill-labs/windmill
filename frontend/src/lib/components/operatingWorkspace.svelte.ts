import { getContext, setContext } from 'svelte'
import { fromStore, toStore, type Readable } from 'svelte/store'
import { workspaceStore } from '$lib/stores'
import { withWorkspaceParam } from './sessions/sessionMode.svelte'
import { useActingUser } from '$lib/actingUser.svelte'

// The workspace a subtree acts on. An AI session edits its (possibly forked) workspace while
// `workspaceStore` stays on the navigation workspace, and a component that reads the navigation
// store directly writes to the parent from inside a fork's editor. So the hosts that embed an
// editor for another workspace set it once, and every component under them reads
// `useOperatingWorkspace()` instead of `$workspaceStore` — nothing has to thread it through props.
// Outside such a host it is the navigation store itself.

const KEY = Symbol('operatingWorkspace')
const navigation = fromStore(workspaceStore)

/** Declare the workspace this subtree acts on. A getter, read wherever the workspace is used;
 * resolving to nothing defers to the enclosing host. Reads context: call during component
 * initialisation. */
export function setOperatingWorkspace(resolve: () => string | undefined): void {
	const outer = getContext<(() => string | undefined) | undefined>(KEY)
	setContext(KEY, outer ? () => resolve() ?? outer() : resolve)
}

/** The workspace this component acts on, as a store. Reads context: call during component
 * initialisation. Falls back to the navigation workspace where no host set one, or where the
 * host's resolves to nothing. */
export function useOperatingWorkspace(): Readable<string | undefined> {
	const resolve = getContext<(() => string | undefined) | undefined>(KEY)
	return resolve ? toStore(() => resolve() ?? navigation.current) : workspaceStore
}

/** The user acting in this subtree's operating workspace: `$userStore` describes the navigation
 * workspace, so a permission check reading it inside a fork's editor answers about the parent —
 * enabling or disabling controls by the wrong roles. Unresolved reads as unknown, which
 * `canWrite`/`isOwner` refuse; ask `resolved` before rendering that as a denial. Reads context
 * and registers an effect: call during component initialisation. */
export function useOperatingUser(): ReturnType<typeof useActingUser> {
	const operating = fromStore(useOperatingWorkspace())
	return useActingUser(() => operating.current)
}

/** Stamps this component's operating workspace onto an in-app link, which otherwise opens in
 * the navigation workspace. Reads context: call during component initialisation. */
export function useOperatingWorkspaceHref(): (href: string) => string {
	const operating = fromStore(useOperatingWorkspace())
	return (href) => withWorkspaceParam(href, operating.current)
}
