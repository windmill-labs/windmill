import { FolderService } from '$lib/gen'
import { Minimatch } from 'minimatch'
import { get } from 'svelte/store'
import { workspaceStore, userStore } from '$lib/stores'

export function useFolderDefaultPermissionedAs(pathGetter: () => string | undefined) {
	let value = $state<string | undefined>(undefined)
	let loading = $state(false)
	let fetchVersion = 0

	$effect(() => {
		const path = pathGetter()
		const user = get(userStore)
		const canPreserve =
			user?.is_admin || user?.is_super_admin || (user?.groups ?? []).includes('wm_deployers')

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

		const thisVersion = ++fetchVersion
		loading = true
		FolderService.getFolder({ workspace, name: folderName })
			.then((folder) => {
				if (thisVersion !== fetchVersion) return
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
				if (thisVersion !== fetchVersion) return
				value = undefined
			})
			.finally(() => {
				if (thisVersion !== fetchVersion) return
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
