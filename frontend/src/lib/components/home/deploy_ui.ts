import { enterpriseLicense, workspaceStore } from '$lib/stores'
import { WorkspaceService, type WorkspaceDeployUISettings } from '$lib/gen'
import { ALL_DEPLOYABLE } from '$lib/utils_deployable'
import { get, writable } from 'svelte/store'

const deployUiStore = writable<Record<string, WorkspaceDeployUISettings>>({})
let pending: Record<string, Promise<WorkspaceDeployUISettings>> = {}
let generation = 0

// Until the license loads, a load reads as CE and resolves to ALL_DEPLOYABLE. Rows warm the cache
// on mount, often before then, so a license arriving drops whatever was loaded without it.
enterpriseLicense.subscribe((license) => {
	if (!license) return
	const hadSettings = Object.keys(get(deployUiStore)).length > 0 || Object.keys(pending).length > 0
	generation++
	pending = {}
	deployUiStore.set({})
	if (hadSettings) warmDeployUiSettings()
})

export async function getDeployUiSettings(): Promise<WorkspaceDeployUISettings> {
	const workspace = get(workspaceStore)!
	const cached = get(deployUiStore)[workspace]
	if (cached) return cached
	const gen = generation
	const load = (pending[workspace] ??= getDeployUiSettingsInner())
	let settings: WorkspaceDeployUISettings
	try {
		settings = await load
	} finally {
		if (pending[workspace] === load) delete pending[workspace]
	}
	if (gen !== generation) return getDeployUiSettings()
	deployUiStore.update((s) => ({ ...s, [workspace]: settings }))
	return settings
}

/**
 * Starts loading the settings without waiting for them. Every row on a page calls this, so a
 * failed fetch must not reject: the root layout toasts each unhandled rejection, once per row.
 */
export function warmDeployUiSettings(): void {
	getDeployUiSettings().catch(() => {})
}

/**
 * The current workspace's settings if a load has already resolved. Row menus read this so they
 * can build their items synchronously: a menu whose items arrive after it opens is positioned for
 * its loading spinner and then flips once the real list is taller. A miss starts a new load, so a
 * fetch that failed is retried on the next open.
 */
export function cachedDeployUiSettings(): WorkspaceDeployUISettings | undefined {
	const settings = get(deployUiStore)[get(workspaceStore)!]
	if (!settings) warmDeployUiSettings()
	return settings
}

async function getDeployUiSettingsInner(): Promise<WorkspaceDeployUISettings> {
	if (!get(enterpriseLicense)) {
		return ALL_DEPLOYABLE
	}
	let settings = await WorkspaceService.getPublicSettings({ workspace: get(workspaceStore)! })
	return settings.deploy_ui ?? ALL_DEPLOYABLE
}
