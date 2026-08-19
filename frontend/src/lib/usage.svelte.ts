import { resource } from 'runed'
import { UserService, WorkspaceService } from '$lib/gen'
import { isCloudHosted } from '$lib/cloud'
import {
	isPremiumStore,
	premiumFetchFailed,
	usageStore,
	workspaceUsageStore,
	type UserExt
} from '$lib/stores'

/**
 * The cloud execution counters and the workspace's plan tier, as `resource`s owned by
 * the root layout.
 *
 * These are per-workspace values published into app-wide stores, so every read site
 * would otherwise have to answer "does this still describe the workspace I'm rendering?"
 * on its own. `resource` answers the ordering half — it discards a superseded fetch and
 * exposes `loading`/`error` as states rather than in-band values — and each value is
 * tagged with the workspace it was fetched for, so the single publish site below can
 * assert the other half. Call once, at layout init.
 */
export function createUsageResources(args: {
	workspace: () => string | undefined
	user: () => UserExt | undefined
}) {
	// Both counters and the tier are cloud-only, and all three need an authenticated
	// membership: key on the user being loaded *for this workspace*, so a switch can't
	// fire them against the workspace we left.
	const readyWorkspace = () => {
		const workspace = args.workspace()
		if (!isCloudHosted() || !workspace) return undefined
		return args.user()?.workspace_id === workspace ? workspace : undefined
	}

	// `Number(...)`: both usage endpoints serve text/plain, so the client hands back a
	// string despite the generated `number` type. Interpolation and arithmetic coerce
	// it, but `toLocaleString` on a string returns it unchanged — the thousands
	// separator would silently go missing above 999.
	const workspaceExecutions = resource(readyWorkspace, async (workspace) =>
		workspace
			? {
					workspace,
					value: Number(await WorkspaceService.getWorkspaceUsage({ workspace }))
				}
			: undefined
	)

	// Account-wide rather than per-workspace, so it survives a switch; still keyed on
	// the workspace being ready, since that is when the session is usable.
	const userExecutions = resource(readyWorkspace, async (workspace) =>
		workspace ? Number(await UserService.getUsage()) : undefined
	)

	const premium = resource(readyWorkspace, async (workspace) =>
		workspace ? { workspace, value: await WorkspaceService.getIsPremium({ workspace }) } : undefined
	)

	// The one place any of this reaches the stores. A value is published only when it is
	// still tagged with the active workspace, and `undefined` while loading or after a
	// failure — never a stand-in like `0` or `false`, both of which are legal values a
	// consumer would render as real.
	$effect(() => {
		const workspace = args.workspace()
		const current = workspaceExecutions.current
		const live = !workspaceExecutions.loading && current && current.workspace === workspace
		workspaceUsageStore.set(live ? current.value : undefined)
	})

	$effect(() => {
		usageStore.set(userExecutions.loading ? undefined : userExecutions.current)
	})

	$effect(() => {
		const workspace = args.workspace()
		const current = premium.current
		const live = !premium.loading && !premium.error && current && current.workspace === workspace
		isPremiumStore.set(live ? current.value : undefined)
		// Distinguishable from "still loading", which is what lets affordances hold
		// through the pending window and still fail closed once the fetch has failed.
		premiumFetchFailed.set(!!premium.error)
	})

	return {
		/** Re-reads the counters. Executions accrue continuously, so anything displaying
		 * them needs this — the workspace-change refetch alone leaves an open tab stale. */
		refreshExecutions() {
			void workspaceExecutions.refetch()
			void userExecutions.refetch()
		}
	}
}

// Registered by the layout so components can ask for a re-read without owning the
// resources or reaching back into the layout.
let handle: ReturnType<typeof createUsageResources> | undefined = undefined

export function registerUsageResources(h: ReturnType<typeof createUsageResources>): void {
	handle = h
}

export function refreshExecutions(): void {
	handle?.refreshExecutions()
}
