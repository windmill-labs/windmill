import { get } from 'svelte/store'
import { goto } from '$lib/navigation'
import { WorkspaceService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { clearStores } from '$lib/storeUtils'
import { workspaceStore } from '$lib/stores'

// Leave the active workspace and land on the workspace list. Shared by the
// settings dropdown and the sidebar menu.
export async function leaveCurrentWorkspace(): Promise<void> {
	await WorkspaceService.leaveWorkspace({ workspace: get(workspaceStore) ?? '' })
	sendUserToast('You left the workspace')
	clearStores()
	await goto('/user/workspaces')
}
