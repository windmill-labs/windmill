import { resource } from 'runed'
import { workspaceStore } from '$lib/stores'
import { WorkspaceService } from '$lib/gen'
import { ADMIN_DATATABLE_ROLE } from '$lib/components/dbTypes'
import { get } from 'svelte/store'

/**
 * The role an app keeps when its default data table changes.
 *
 * A data table role is defined on one data table, so it does not follow the
 * default to another: kept, the app's queries would name a role that data table
 * has never heard of, and the one it names here may not be the one it gets.
 */
export function roleAfterDatatableChange(
	previous: string | undefined,
	next: string | undefined,
	role: string | undefined
): string | undefined {
	return next === previous ? role : undefined
}

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
 * Creates a resource that loads the roles the caller may use on a datatable,
 * and the one it defaults to.
 */
export function createRolesResource(
	getDatatable: () => string | undefined,
	getWorkspace: () => string | undefined = () => get(workspaceStore)
) {
	return resource(
		() => [getDatatable() ?? '', getWorkspace() ?? ''] as const,
		async ([datatableName, workspace]): Promise<{
			/** The data table this answers for: while a switch is in flight the
			 * previous one's roles are still what `current` holds, and they say
			 * nothing about the one now selected. */
			datatable: string | undefined
			/** Whether the data table has permissions on. With none, every caller
			 * reaches it through its own connection and `roles` is empty because
			 * there is nothing to pick — which is not the same as a permissioned one
			 * this caller may run as nothing on. */
			permissioned: boolean
			/** The lookup itself failed, so neither of the above is an answer: an
			 * empty `roles` here means nothing was learned, not that there is
			 * nothing to pick. */
			failed: boolean
			roles: string[]
			defaultRole: string
		}> => {
			const empty = {
				datatable: datatableName || undefined,
				permissioned: false,
				failed: false,
				roles: [],
				defaultRole: ADMIN_DATATABLE_ROLE
			}
			if (!datatableName || !workspace) return empty
			try {
				const res = await WorkspaceService.listUsableDatatableRoles({ workspace, datatableName })
				return {
					...empty,
					permissioned: res.enabled,
					roles: res.enabled ? res.roles : [],
					defaultRole: res.default_role
				}
			} catch (e) {
				console.error('Failed to load datatable roles:', e)
				return { ...empty, failed: true }
			}
		},
		{
			initialValue: {
				datatable: undefined,
				permissioned: false,
				failed: false,
				roles: [],
				defaultRole: ADMIN_DATATABLE_ROLE
			}
		}
	)
}

/**
 * Creates a resource that loads, for one data table read as one role, the
 * schemas that role can reach and whether it may create more.
 *
 * Both answers are the connected role's, and one call carries them: asking the
 * schema list of a role that cannot see a schema is the same question as asking
 * what it may create in.
 */
export function createDatatableAccessResource(
	getDatatable: () => string | undefined,
	getRole: () => string | undefined,
	getWorkspace: () => string | undefined = () => get(workspaceStore)
) {
	return resource(
		() => [getDatatable() ?? '', getRole() ?? '', getWorkspace() ?? ''] as const,
		async ([datatable, role, workspace]): Promise<{
			/** What this answers for — both halves of it. Until they match the
			 * selection, the schemas and the right to create one belong to another
			 * data table or to another role, and at mount they are the initial value
			 * rather than an answer at all. What a role may create in is exactly the
			 * question, so an answer computed as a different one settles nothing. */
			datatable: string | undefined
			role: string | undefined
			schemas: string[]
			canCreateSchema: boolean
		}> => {
			const asked = { datatable: datatable || undefined, role: role || undefined }
			if (!datatable || !workspace) return { ...asked, schemas: [], canCreateSchema: false }
			try {
				const tables = await WorkspaceService.listDataTableTables({
					workspace,
					roleFor: datatable,
					role: role || undefined
				})
				const entry = tables.find((t) => t.datatable_name === datatable)
				return {
					...asked,
					schemas: Object.keys(entry?.schemas ?? {}).sort(),
					canCreateSchema: !!entry?.can_create_schema
				}
			} catch (e) {
				console.error('Failed to load datatable access:', e)
				return { ...asked, schemas: [], canCreateSchema: false }
			}
		},
		{ initialValue: { datatable: undefined, role: undefined, schemas: [], canCreateSchema: false } }
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
