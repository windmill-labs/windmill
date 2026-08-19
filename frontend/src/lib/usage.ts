import { get } from 'svelte/store'
import { UserService, WorkspaceService } from '$lib/gen'
import { isCloudHosted } from '$lib/cloud'
import { usageStore, workspaceStore, workspaceUsageStore } from '$lib/stores'

// Workspace the value in `workspaceUsageStore` describes.
let usageFetchedFor: string | undefined = undefined

/**
 * Refreshes the cloud execution counters. Lives here rather than in the root layout
 * because executions accrue continuously: anything that displays them needs to be able
 * to re-read them, not only the layout on a workspace change.
 */
export async function refreshUsage(): Promise<void> {
	const workspace = get(workspaceStore)
	if (!isCloudHosted() || !workspace) return
	// Workspace usage belongs to a workspace: clear it for a new one, and drop a
	// response that lost the race. User usage is account-wide, so it survives the
	// switch. Each is assigned on its own so one endpoint failing leaves the other's
	// number intact rather than unresolved.
	if (usageFetchedFor !== workspace) {
		usageFetchedFor = workspace
		workspaceUsageStore.set(undefined)
	}
	// `Number(...)`: both usage endpoints serve text/plain, so the client hands back a
	// string despite the generated `number` type. Interpolation and arithmetic coerce
	// it, but `toLocaleString` on a string returns it unchanged — the thousands
	// separator would silently go missing above 999.
	await Promise.all([
		UserService.getUsage()
			.then((usage) => usageStore.set(Number(usage)))
			.catch((e) => console.error('Could not fetch user usage', e)),
		WorkspaceService.getWorkspaceUsage({ workspace })
			.then((usage) => {
				if (get(workspaceStore) === workspace) {
					workspaceUsageStore.set(Number(usage))
				}
			})
			.catch((e) => console.error('Could not fetch workspace usage', e))
	])
}
