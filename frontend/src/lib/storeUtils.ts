import { resourceTypesStore } from './components/resourceTypesStore'
import { refreshSuperadmin } from './refreshUser'
import { clearWorkspaceRoleCache } from './user'
import {
	workspaceStore,
	userStore,
	usersWorkspaceStore,
	nonMemberWorkspaces,
	superadmin,
	devopsRole,
	clearWorkspaceFromStorage
} from './stores'
import { resetProtectionRules, loadProtectionRules } from './workspaceProtectionRules.svelte'

export function switchWorkspace(workspace: string | undefined) {
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
		clearWorkspaceFromStorage()
	} catch (e) {
		console.error('error interacting with local storage', e)
	}

	resourceTypesStore.set(undefined)
	resetProtectionRules()
	clearWorkspaceRoleCache()
	userStore.set(undefined)
	workspaceStore.set(undefined)
	usersWorkspaceStore.set(undefined)
	nonMemberWorkspaces.set(undefined)
	refreshSuperadmin.cancel()
	superadmin.set(undefined)
	devopsRole.set(undefined)
}
