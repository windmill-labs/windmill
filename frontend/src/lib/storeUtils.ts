import { resourceTypesStore } from './components/resourceTypesStore'
import { refreshSuperadmin } from './refreshUser'
import {
	workspaceStore,
	userStore,
	usersWorkspaceStore,
	superadmin,
	devopsRole,
	clearWorkspaceFromStorage
} from './stores'
import { resetProtectionRules, loadProtectionRules } from './workspaceProtectionRules.svelte'

export function switchWorkspace(workspace: string | undefined) {
	try {
		localStorage.removeItem('flow')
		localStorage.removeItem('app')
	} catch (e) {
		console.error('error interacting with local storage', e)
	}
	resourceTypesStore.set(undefined)

	// Clear protection rules state
	resetProtectionRules()

	workspaceStore.set(workspace)

	// Eagerly load protection rules for new workspace
	if (workspace) {
		loadProtectionRules(workspace)
	}
}

export function clearStores(): void {
	try {
		localStorage.removeItem('flow')
		localStorage.removeItem('app')
		clearWorkspaceFromStorage()
	} catch (e) {
		console.error('error interacting with local storage', e)
	}

	resourceTypesStore.set(undefined)
	resetProtectionRules()
	userStore.set(undefined)
	workspaceStore.set(undefined)
	usersWorkspaceStore.set(undefined)
	refreshSuperadmin.cancel()
	superadmin.set(undefined)
	devopsRole.set(undefined)
}
