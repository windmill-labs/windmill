import { resourceTypesStore } from './components/resourceTypesStore'
import {
	workspaceStore,
	userStore,
	usersWorkspaceStore,
	superadmin,
	devopsRole,
	clearWorkspaceFromStorage
} from './stores'
import { protectionRulesStore, loadProtectionRules } from './workspaceProtectionRulesStore'

export function switchWorkspace(workspace: string | undefined) {
	try {
		localStorage.removeItem('flow')
		localStorage.removeItem('app')
	} catch (e) {
		console.error('error interacting with local storage', e)
	}
	resourceTypesStore.set(undefined)

	// Clear protection rules store
	protectionRulesStore.set({
		rulesets: undefined,
		loading: false,
		error: undefined
	})

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
	protectionRulesStore.set({
		rulesets: undefined,
		loading: false,
		error: undefined
	})
	userStore.set(undefined)
	workspaceStore.set(undefined)
	usersWorkspaceStore.set(undefined)
	superadmin.set(undefined)
	devopsRole.set(undefined)
}
