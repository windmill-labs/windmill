import { FolderService } from '$lib/gen'
import { Minimatch } from 'minimatch'
import { get } from 'svelte/store'
import { workspaceStore, userStore } from '$lib/stores'

/**
 * Resolves the folder's `default_permissioned_as` rule that matches a given item
 * path. Used by the deploy-side UI to preselect the folder default in the
 * `OnBehalfOfSelector` for admins and `wm_deployers` members.
 *
 * Returns `undefined` when:
 * - the path is not under a folder,
 * - the user is not admin or `wm_deployers`,
 * - the folder has no rules,
 * - no rule matches.
 *
 * Matching is relative to the folder root — `jobs/**` matches `f/<folder>/jobs/...`.
 */
export function useFolderDefaultPermissionedAs(pathGetter: () => string | undefined) {
	let value = $state<string | undefined>(undefined)
	let loading = $state(false)

	$effect(() => {
		const path = pathGetter()
		const user = get(userStore)
		const canPreserve = user?.is_admin || (user?.groups ?? []).includes('wm_deployers')

		if (!canPreserve || !path?.startsWith('f/')) {
			value = undefined
			return
		}

		const parts = path.split('/')
		if (parts.length < 3) {
			value = undefined
			return
		}
		const folderName = parts[1]
		const relative = path.slice(`f/${folderName}/`.length)
		if (!relative) {
			value = undefined
			return
		}

		const workspace = get(workspaceStore)
		if (!workspace) {
			value = undefined
			return
		}

		loading = true
		FolderService.getFolder({ workspace, name: folderName })
			.then((folder) => {
				const rules = folder.default_permissioned_as ?? []
				const match = rules.find((rule) => {
					try {
						return new Minimatch(rule.path_glob).match(relative)
					} catch {
						return false
					}
				})
				value = match?.permissioned_as
			})
			.catch(() => {
				value = undefined
			})
			.finally(() => {
				loading = false
			})
	})

	return {
		get value() {
			return value
		},
		get loading() {
			return loading
		}
	}
}
