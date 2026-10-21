import { FolderService } from '$lib/gen'
import { resource } from 'runed'
import { untrack } from 'svelte'
import { workspaceStore, userStore } from '$lib/stores'

/** The folder rule a create at `path` takes on the server, for a caller it applies to. */
export async function resolveFolderDefaultPermissionedAs(
	workspace: string,
	path: string
): Promise<string | undefined> {
	if (!path.startsWith('f/')) return undefined
	try {
		return (
			(await FolderService.getFolderDefaultPermissionedAs({ workspace, path }))?.permissioned_as ??
			undefined
		)
	} catch {
		return undefined
	}
}

/**
 * Resolves the folder's `default_permissioned_as` rule that matches a given item path.
 * Returns `undefined` when the user is not admin/wm_deployer, the path is not under a
 * folder, the folder has no matching rules, or the fetch fails.
 *
 * Tracks workspace/user store changes reactively so the value refreshes on workspace
 * switch or permission change. Uses `runed`'s `resource()` which handles race conditions
 * and loading state automatically.
 */
export function useFolderDefaultPermissionedAs(
	pathGetter: () => string | undefined,
	// The acting workspace; defaults to the nav `$workspaceStore` when omitted so
	// a forked session resolves the folder default from its own workspace.
	wsGetter?: () => string | undefined
) {
	// Subscribe to the stores so the effect re-runs on changes.
	let navWorkspace = $state<string | undefined>(undefined)
	let user = $state<any>(undefined)

	$effect(() => {
		const unsubWs = workspaceStore.subscribe((w) => (navWorkspace = w))
		const unsubUser = userStore.subscribe((u) => (user = u))
		return () => {
			unsubWs()
			unsubUser()
		}
	})

	const folderResource = resource(
		() => ({ path: pathGetter(), workspace: wsGetter?.() ?? navWorkspace, user }),
		async ({ path, workspace, user }) => {
			const canPreserve =
				user?.is_admin || user?.is_super_admin || (user?.groups ?? []).includes('wm_deployers')
			if (!canPreserve || !workspace || !path) return undefined
			return resolveFolderDefaultPermissionedAs(workspace, path)
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

/**
 * Seeds a new script's or flow's run-as with the folder rule its path matches, so the editor
 * shows, and the deploy sends, the identity the rule would give it. The principal is sent
 * alone; the server derives the address. An identity the user or a seed chose is left alone,
 * and one seeded here is cleared again once the path stops matching.
 */
export function useFolderDefaultRunAs(
	newPathGetter: () => string | undefined,
	wsGetter: () => string | undefined,
	runAs: {
		get: () => { on_behalf_of_email?: string; on_behalf_of?: string }
		set: (permissionedAs: string | undefined) => void
	}
) {
	const folderDefault = useFolderDefaultPermissionedAs(newPathGetter, wsGetter)
	let seeded: string | undefined = undefined
	$effect(() => {
		// Creation ending (the first deploy) is not the path leaving the rule: keep the identity.
		if (newPathGetter() === undefined || folderDefault.loading) return
		const rule = folderDefault.value
		untrack(() => {
			if (rule === seeded) return
			const current = runAs.get()
			const ours =
				seeded === undefined
					? !current.on_behalf_of_email && !current.on_behalf_of
					: current.on_behalf_of === seeded
			if (!ours) return
			seeded = rule
			runAs.set(rule)
		})
	})
	return folderDefault
}

/**
 * Folder rules a deploy into another workspace meets, looked up once per item on first read.
 * Only an item new in that workspace takes one, so callers ask only for those.
 */
export function createFolderDefaultLookup() {
	const values = $state<Record<string, string | undefined>>({})
	const requested = new Set<string>()
	return (workspace: string, path: string): string | undefined => {
		const key = `${workspace}:${path}`
		if (!requested.has(key)) {
			requested.add(key)
			resolveFolderDefaultPermissionedAs(workspace, path).then((v) => (values[key] = v))
		}
		return values[key]
	}
}
