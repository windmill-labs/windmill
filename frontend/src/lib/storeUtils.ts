import { resourceTypesStore } from "./components/resourceTypesStore"
import { workspaceStore, userStore, usersWorkspaceStore, superadmin } from "./stores"

export function switchWorkspace(workspace: string | undefined) {
	localStorage.removeItem('flow')
	localStorage.removeItem('app')
	resourceTypesStore.set(undefined)
	workspaceStore.set(workspace)
}

export function clearStores(): void {
	localStorage.removeItem('flow')
	localStorage.removeItem('app')
	localStorage.removeItem('workspace')
	resourceTypesStore.set(undefined)
	userStore.set(undefined)
	workspaceStore.set(undefined)
	usersWorkspaceStore.set(undefined)
	superadmin.set(undefined)
}
