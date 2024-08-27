import { enterpriseLicense, workspaceStore } from '$lib/stores'
import { WorkspaceService, type WorkspaceDeployUISettings } from '$lib/gen'
import { ALL_DEPLOYABLE } from '$lib/utils_deployable'
import { get, writable } from 'svelte/store'

const deployUiStore = writable<Record<string, WorkspaceDeployUISettings>>({})
export async function getDeployUiSettings(): Promise<WorkspaceDeployUISettings> {
	let deployUiSettings = get(deployUiStore)
	let workspace = get(workspaceStore)
	if (!deployUiSettings[workspace!]) {
		deployUiSettings[workspace!] = await getDeployUiSettingsInner()
		deployUiStore.set(deployUiSettings)
	}
	return deployUiSettings[workspace!]
}

async function getDeployUiSettingsInner(): Promise<WorkspaceDeployUISettings> {
	if (!get(enterpriseLicense)) {
		return ALL_DEPLOYABLE
	}
	let settings = await WorkspaceService.getSettings({ workspace: get(workspaceStore)! })
	return settings.deploy_ui ?? ALL_DEPLOYABLE
}
