import { FolderService } from '$lib/gen'
import { Minimatch } from 'minimatch'
import { resource } from 'runed'
import { fromStore } from 'svelte/store'
import { workspaceStore } from '$lib/stores'
import { useActingUser } from '$lib/actingUser.svelte'
import { canPreserveOnBehalfOf } from './itemSettings/onBehalfOf'

/**
 * Resolves the folder's `default_permissioned_as` rule that matches a given item path.
 * Returns `undefined` when the user is not admin/wm_deployer, the path is not under a
 * folder, the folder has no matching rules, or the fetch fails.
 *
 * The rights are the acting user's in the resolved workspace, as the backend checks them there:
 * `$userStore` answers for the navigation workspace only. `loading` holds while that user is
 * still being looked up. Call during component initialisation.
 */
export function useFolderDefaultPermissionedAs(
	pathGetter: () => string | undefined,
	// The acting workspace; defaults to the nav `$workspaceStore` when omitted so
	// a forked session resolves the folder default from its own workspace.
	wsGetter?: () => string | undefined
) {
	const navWorkspace = fromStore(workspaceStore)
	const workspace = () => wsGetter?.() ?? navWorkspace.current
	const acting = useActingUser(workspace)

	const folderResource = resource(
		() => ({ path: pathGetter(), workspace: workspace(), user: acting.current }),
		async ({ path, workspace, user }) => {
			if (!canPreserveOnBehalfOf(user) || !workspace || !path?.startsWith('f/')) {
				return undefined
			}
			const parts = path.split('/')
			if (parts.length < 3) return undefined
			const folderName = parts[1]
			const relative = path.slice(`f/${folderName}/`.length)
			if (!relative) return undefined

			try {
				const folder = await FolderService.getFolder({ workspace, name: folderName })
				const rules = folder.default_permissioned_as ?? []
				const match = rules.find((rule) => {
					try {
						return new Minimatch(rule.path_glob).match(relative)
					} catch {
						return false
					}
				})
				return match?.permissioned_as
			} catch {
				return undefined
			}
		}
	)

	return {
		get value() {
			return folderResource.current
		},
		get loading() {
			const ws = workspace()
			return (
				folderResource.loading ||
				(!!ws && !!pathGetter()?.startsWith('f/') && !acting.resolved(ws))
			)
		}
	}
}
