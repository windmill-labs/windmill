import { resourceTypesStore } from './components/resourceTypesStore'
import { workspaceStore, userStore, usersWorkspaceStore, superadmin, devopsRole } from './stores'

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
		localStorage.removeItem('workspace')
	} catch (e) {
		console.error('error interacting with local storage', e)
	}

	resourceTypesStore.set(undefined)
	userStore.set(undefined)
	workspaceStore.set(undefined)
	usersWorkspaceStore.set(undefined)
	superadmin.set(undefined)
	devopsRole.set(undefined)
}
