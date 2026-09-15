import { WorkspaceService, type ListUsableDatatableRolesResponse } from '$lib/gen'
import { ADMIN_DATATABLE_ROLE } from './dbTypes'

// `datatable_roles_unavailable` on the server.
const ROLES_UNAVAILABLE = 'Data table roles are a Windmill Enterprise Edition feature'

/**
 * The roles the caller may connect as on a data table. Without the Enterprise Edition every roles
 * route refuses, which reads as "not under roles": the data table is then used the way it was
 * before roles, and one that is under roles is refused when something connects to it.
 */
export async function listUsableDatatableRoles(
	workspace: string,
	datatableName: string
): Promise<ListUsableDatatableRolesResponse> {
	try {
		return await WorkspaceService.listUsableDatatableRoles({ workspace, datatableName })
	} catch (e) {
		const detail = `${(e as { body?: unknown })?.body ?? ''} ${(e as Error)?.message ?? e}`
		if (detail.includes(ROLES_UNAVAILABLE)) {
			return { permissioned: false, roles: [], default_role: ADMIN_DATATABLE_ROLE }
		}
		throw e
	}
}
