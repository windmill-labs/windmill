import { ResourceService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'

export interface Group {
	path: string
	value: {
		value?: object
		name: string
	}
}

export function createGroup(workspace: string, group: Group): Promise<string> {
	const createGroupRequest = {
		workspace,
		requestBody: {
			...group,
			resource_type: 'app_group',
			value: group.value || ''
		}
	}
	return ResourceService.createResource(createGroupRequest)
}

export async function getGroup(
	workspace: string,
	path: string
): Promise<{
	name: string
	value: any
}> {
	try {
		return ResourceService.getResourceValue({
			workspace,
			path
		}) as any
	} catch (e) {
		sendUserToast(`Group not found ${path}`)
		return {
			value: '',
			name: 'Not found'
		}
	}
}

export function updateGroup(workspace: string, path: string, updatedGroup: any): Promise<string> {
	const updateGroupRequest = {
		workspace,
		path,
		requestBody: updatedGroup
	}
	return ResourceService.updateResource(updateGroupRequest)
}

export function deleteGroup(workspace: string, path: string): Promise<string> {
	const deleteGroupRequest = {
		workspace,
		path: path
	}
	return ResourceService.deleteResource(deleteGroupRequest)
}

export async function listGroups(workspace: string): Promise<
	Array<{
		name: string
		path: string
	}>
> {
	const listGroupsRequest = {
		workspace,
		name: 'app_group'
	}
	return ResourceService.listResourceNames(listGroupsRequest)
}
