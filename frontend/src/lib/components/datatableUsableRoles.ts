import { WorkspaceService, type ListUsableDatatableRolesResponse } from '$lib/gen'
import { isCloudHosted } from '$lib/cloud'
import { ADMIN_DATATABLE_ROLE } from './dbTypes'

// `datatable_roles_unavailable` on the server.
const ROLES_UNAVAILABLE = 'Data table roles are a Windmill Enterprise Edition feature'

const NOT_UNDER_ROLES: ListUsableDatatableRolesResponse = {
	permissioned: false,
	roles: [],
	default_role: ADMIN_DATATABLE_ROLE
}

/**
 * The roles the caller may connect as on a data table. Cloud has no instance database, so no data
 * table there is under roles, and none of the role pickers show. Without the Enterprise Edition
 * every roles route refuses, which reads the same way: the data table is then used the way it was
 * before roles, and one that is under roles is refused when something connects to it.
 */
export async function listUsableDatatableRoles(
	workspace: string,
	datatableName: string
): Promise<ListUsableDatatableRolesResponse> {
	if (isCloudHosted()) return NOT_UNDER_ROLES
	try {
		return await WorkspaceService.listUsableDatatableRoles({ workspace, datatableName })
	} catch (e) {
		const detail = `${(e as { body?: unknown })?.body ?? ''} ${(e as Error)?.message ?? e}`
		if (detail.includes(ROLES_UNAVAILABLE)) return NOT_UNDER_ROLES
		throw e
	}
}
