import { resource } from 'runed'
import { WorkspaceService } from '$lib/gen'
import type { UserWorkspace } from '$lib/stores'

/**
 * `$userWorkspaces` only lists workspaces the user is a *member* of. A superadmin can navigate into a
 * workspace they don't belong to, which leaves fork surfaces (the base picker, the family picker)
 * unable to resolve that workspace's family — so it appears empty. This composable fetches the
 * visited-but-unlisted workspace's metadata as superadmin and appends it, so the family root resolves
 * and can be forked. Its wider family (ancestors/siblings the caller can't see) stays unreachable
 * client-side, so only the visited workspace itself is added.
 */
export function useForkableWorkspaces(args: {
	workspaces: () => UserWorkspace[]
	currentWorkspaceId: () => string | undefined
	isSuperadmin: () => boolean
	// Skip the lookup entirely when the fork surface isn't in play (defaults to always enabled).
	enabled?: () => boolean
}) {
	const missing = resource(
		() => {
			const id = args.currentWorkspaceId()
			const enabled = args.enabled?.() ?? true
			return enabled && args.isSuperadmin() && id && !args.workspaces().some((w) => w.id === id)
				? id
				: undefined
		},
		async (ws) =>
			ws ? await WorkspaceService.getWorkspaceAsSuperAdmin({ workspace: ws }) : undefined
	)
	return {
		get current(): UserWorkspace[] {
			const base = args.workspaces()
			const w = missing.current
			// Drop the fetched entry once membership catches up, so the id is never duplicated.
			if (!w || base.some((b) => b.id === w.id)) return base
			return [
				...base,
				{
					id: w.id,
					name: w.name,
					username: '',
					color: w.color ?? undefined,
					parent_workspace_id: w.parent_workspace_id,
					disabled: false
				}
			]
		}
	}
}
