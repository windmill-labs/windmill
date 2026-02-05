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

export function switchWorkspace(workspace: string | undefined) {
	try {
		localStorage.removeItem('flow')
		localStorage.removeItem('app')
	} catch (e) {
		console.error('error interacting with local storage', e)
	}
	resourceTypesStore.set(undefined)
	workspaceStore.set(workspace)
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
	userStore.set(undefined)
	workspaceStore.set(undefined)
	usersWorkspaceStore.set(undefined)
	refreshSuperadmin.cancel()
	superadmin.set(undefined)
	devopsRole.set(undefined)
}
