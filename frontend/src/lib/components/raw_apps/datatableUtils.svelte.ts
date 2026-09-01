import { resource } from 'runed'
import { workspaceStore, dbSchemas } from '$lib/stores'
import { WorkspaceService } from '$lib/gen'
import { getDbSchemas } from '$lib/components/apps/components/display/dbtable/metadata'
import { ADMIN_DATATABLE_ROLE } from '$lib/components/dbTypes'
import { get } from 'svelte/store'

/**
 * Creates a resource that loads available datatables from the workspace.
 * Pass a getter function that returns the workspace to create a reactive dependency.
 */
export function createDatatablesResource(getWorkspace: () => string | undefined) {
	return resource.pre<string[]>([() => getWorkspace() ?? ''], async () => {
		const workspace = getWorkspace()
		if (!workspace) return []
		try {
			return (await WorkspaceService.listDataTables({ workspace })).map((d) => d.name)
		} catch (e) {
			console.error('Failed to load datatables:', e)
			return []
		}
	})
}

/**
 * Creates a resource that loads schemas for a given datatable.
 * The getDatatable getter is used as a reactive dependency - when it changes, schemas are refetched.
 */
export function createSchemasResource(
	getDatatable: () => string | undefined,
	getWorkspace: () => string | undefined = () => get(workspaceStore)
) {
	return resource<string[]>([() => getDatatable() ?? '', () => getWorkspace() ?? ''], async () => {
		const datatable = getDatatable()
		const workspace = getWorkspace()
		if (!datatable || !workspace) return []

		const resourcePath = `datatable://${datatable}`
		// Key the schema cache by workspace too: a datatable of the same name can
		// exist in both the nav and the acting workspace, so `datatable://<name>`
		// alone would let one workspace's schema be reused for the other.
		const cacheKey = `${workspace}:${resourcePath}`
		const schemas = get(dbSchemas)
		let dbSchema = schemas[cacheKey]

		if (!dbSchema) {
			try {
				schemas[cacheKey] = await getDbSchemas('postgresql', resourcePath, workspace, (msg) =>
					console.error('Schema error:', msg)
				)
				dbSchema = get(dbSchemas)[cacheKey]
			} catch (e) {
				console.error(`Failed to load schema for ${datatable}:`, e)
				return []
			}
		}

		if (!dbSchema?.schema) return []
		return Object.keys(dbSchema.schema)
	})
}

/**
 * Creates a resource that loads the roles the caller may use on a datatable,
 * and the one it defaults to.
 */
export function createRolesResource(
	getDatatable: () => string | undefined,
	getWorkspace: () => string | undefined = () => get(workspaceStore)
) {
	return resource(
		() => [getDatatable() ?? '', getWorkspace() ?? ''] as const,
		async ([datatableName, workspace]): Promise<{ roles: string[]; defaultRole: string }> => {
			if (!datatableName || !workspace) return { roles: [], defaultRole: ADMIN_DATATABLE_ROLE }
			try {
				const res = await WorkspaceService.listUsableDatatableRoles({ workspace, datatableName })
				return {
					roles: res.enabled ? res.roles : [],
					defaultRole: res.default_role
				}
			} catch (e) {
				console.error('Failed to load datatable roles:', e)
				return { roles: [], defaultRole: ADMIN_DATATABLE_ROLE }
			}
		},
		{ initialValue: { roles: [], defaultRole: ADMIN_DATATABLE_ROLE } }
	)
}

/**
 * Whether naming a role says anything here: a data table without permissions has
 * none to pick, and one whose single role is the implicit `admin` has no choice
 * to offer.
 */
export function rolesWorthPicking(roles: string[]): boolean {
	return roles.length > 1 || (roles.length === 1 && roles[0] !== ADMIN_DATATABLE_ROLE)
}

/**
 * Converts datatables array to Select items format
 */
export function toDatatableItems(datatables: string[]) {
	return (
		datatables?.map((dt) => ({
			value: dt,
			label: dt
		})) ?? []
	)
}

/**
 * Converts schemas array to Select items format
 */
export function toSchemaItems(schemas: string[]) {
	return (
		schemas?.map((s) => ({
			value: s,
			label: s
		})) ?? []
	)
}
