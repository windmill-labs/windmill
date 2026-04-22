import { FolderService } from '$lib/gen'
import { Minimatch } from 'minimatch'
import { resource } from 'runed'
import { workspaceStore, userStore } from '$lib/stores'

/**
 * Resolves the folder's `default_permissioned_as` rule that matches a given item path.
 * Returns `undefined` when the user is not admin/wm_deployer, the path is not under a
 * folder, the folder has no matching rules, or the fetch fails.
 *
 * Tracks workspace/user store changes reactively so the value refreshes on workspace
 * switch or permission change. Uses `runed`'s `resource()` which handles race conditions
 * and loading state automatically.
 */
export function useFolderDefaultPermissionedAs(pathGetter: () => string | undefined) {
	// Subscribe to the stores so the effect re-runs on changes.
	let workspace = $state<string | undefined>(undefined)
	let user = $state<any>(undefined)

	$effect(() => {
		const unsubWs = workspaceStore.subscribe((w) => (workspace = w))
		const unsubUser = userStore.subscribe((u) => (user = u))
		return () => {
			unsubWs()
			unsubUser()
		}
	})

	const folderResource = resource(
		() => ({ path: pathGetter(), workspace, user }),
		async ({ path, workspace, user }) => {
			const canPreserve =
				user?.is_admin || user?.is_super_admin || (user?.groups ?? []).includes('wm_deployers')
			if (!canPreserve || !workspace || !path?.startsWith('f/')) {
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
			return folderResource.loading
		}
	}
}
