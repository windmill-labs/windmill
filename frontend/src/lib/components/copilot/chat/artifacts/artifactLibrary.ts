import { get } from 'svelte/store'
import { userWorkspaces } from '$lib/stores'
import { shareWorkspaceId } from './artifactSharing'

/** The workspace an artifact made while working in `workspaceId` belongs to. */
export function homeWorkspace(workspaceId: string): string {
	return shareWorkspaceId(workspaceId, get(userWorkspaces))
}
