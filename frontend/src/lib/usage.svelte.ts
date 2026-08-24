import { resource } from 'runed'
import { UserService, WorkspaceService } from '$lib/gen'
import { isCloudHosted } from '$lib/cloud'
import { scopedValue, tagged } from '$lib/utils/scopedValue'
import {
	isPremiumStore,
	premiumFetchFailed,
	usageStore,
	workspaceUsageStore,
	type UserExt
} from '$lib/stores'

/**
 * The cloud execution counters and the workspace's plan tier. Call once, at layout init:
 * these are app-wide values, and the logged-in layout outlives every in-app navigation.
 */
export function createUsageResources(args: {
	workspace: () => string | undefined
	user: () => UserExt | undefined
}) {
	// All three need an authenticated membership, so they key on the user being loaded
	// *for this workspace* — a switch must not fire them against the workspace we left.
	const readyWorkspace = () => {
		const workspace = args.workspace()
		if (!isCloudHosted() || !workspace) return undefined
		return args.user()?.workspace_id === workspace ? workspace : undefined
	}
	// The user counter is account-wide, so its key is the account: a workspace switch
	// is not a change of key and must not re-fetch or clear it.
	const readyUser = () => (isCloudHosted() ? args.user()?.email : undefined)

	// `Number(...)`: both usage endpoints serve text/plain, so the client hands back a
	// string despite the generated `number` type. Interpolation and arithmetic coerce
	// it, but `toLocaleString` on a string returns it unchanged — the thousands
	// separator would silently go missing above 999.
	const fetchWorkspaceExecutions = tagged(async (workspace: string) =>
		Number(await WorkspaceService.getWorkspaceUsage({ workspace }))
	)
	const fetchUserExecutions = tagged(async (_email: string) => Number(await UserService.getUsage()))
	const fetchPremium = tagged((workspace: string) => WorkspaceService.getIsPremium({ workspace }))

	const workspaceExecutions = resource(readyWorkspace, async (workspace) =>
		workspace ? await fetchWorkspaceExecutions(workspace) : undefined
	)

	const userExecutions = resource(readyUser, async (email) =>
		email ? await fetchUserExecutions(email) : undefined
	)

	const premium = resource(readyWorkspace, async (workspace) =>
		workspace ? await fetchPremium(workspace) : undefined
	)

	const scopedWorkspaceExecutions = scopedValue<number>()
	const scopedUserExecutions = scopedValue<number>()
	const scopedPremium = scopedValue<boolean>()

	// The only place any of this reaches a store. `undefined` until a value for the
	// active scope has arrived — never a stand-in like `0` or `false`, both of which are
	// legal values a consumer would render as real.
	$effect(() => {
		workspaceUsageStore.set(
			scopedWorkspaceExecutions(args.workspace(), workspaceExecutions.current)
		)
	})

	$effect(() => {
		usageStore.set(scopedUserExecutions(args.user()?.email, userExecutions.current))
	})

	$effect(() => {
		const tier = scopedPremium(args.workspace(), premium.current)
		isPremiumStore.set(tier)
		// Only a failure that left us with no tier for this workspace counts: a late
		// rejection for a workspace we left must not retract affordances here.
		premiumFetchFailed.set(!!premium.error && tier === undefined)
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
