import { resource } from 'runed'
import { workspaceStore } from '$lib/stores'
import { WorkspaceService } from '$lib/gen'
import { ADMIN_DATATABLE_ROLE } from '$lib/components/dbTypes'
import { get } from 'svelte/store'

/**
 * `fetch` wrapped so that an answer for a request a newer one has replaced resolves to
 * `stale()` instead: a resource keeps whichever answer lands last, and a slow answer for the
 * previous data table or role would otherwise describe a selection that no longer exists.
 */
function latestOnly<A extends unknown[], T>(
	fetch: (...args: A) => Promise<T>,
	stale: () => T
): (...args: A) => Promise<T> {
	let run = 0
	return async (...args) => {
		const mine = ++run
		const result = await fetch(...args)
		return mine === run ? result : stale()
	}
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

export type DatatableRoles = {
	/** The data table this answers for: while a switch is in flight, `current` still holds the
	 * previous one's roles, which say nothing about the one now selected. */
	datatable: string | undefined
	/** Whether the data table is under roles. Without roles `roles` is empty because there is
	 * nothing to pick, which is not the same as a permissioned one this caller may use no role of. */
	permissioned: boolean
	/** The lookup failed, so an empty `roles` means nothing was learned. */
	failed: boolean
	roles: string[]
	defaultRole: string
}

/**
 * Creates a resource that loads the roles the caller may use on a datatable, and the one it
 * defaults to.
 */
export function createRolesResource(
	getDatatable: () => string | undefined,
	getWorkspace: () => string | undefined = () => get(workspaceStore)
) {
	const initialValue: DatatableRoles = {
		datatable: undefined,
		permissioned: false,
		failed: false,
		roles: [],
		defaultRole: ADMIN_DATATABLE_ROLE
	}
	const rolesResource = resource(
		() => [getDatatable() ?? '', getWorkspace() ?? ''] as const,
		latestOnly(
			async ([datatableName, workspace]: readonly [string, string]): Promise<DatatableRoles> => {
				const empty = { ...initialValue, datatable: datatableName || undefined }
				if (!datatableName || !workspace) return empty
				try {
					const res = await WorkspaceService.listUsableDatatableRoles({ workspace, datatableName })
					return {
						...empty,
						permissioned: res.permissioned,
						roles: res.roles,
						defaultRole: res.default_role
					}
				} catch (e) {
					console.error('Failed to load datatable roles:', e)
					return { ...empty, failed: true }
				}
			},
			() => rolesResource.current
		),
		{ initialValue }
	)
	return rolesResource
}

export type DatatableAccess = {
	/** What this answers for. Until both match the selection, the schemas and the right to
	 * create one belong to another data table or another role. */
	datatable: string | undefined
	role: string | undefined
	/** The request failed, or the server kept the entry with an error (a role this caller may
	 * not use, an unreachable database). `canCreateSchema: false` is then no answer at all. */
	failed: boolean
	error: string | undefined
	schemas: string[]
	canCreateSchema: boolean
}

/**
 * Creates a resource that loads, for one data table read as one role, the schemas that role can
 * reach and whether it may create more.
 */
export function createDatatableAccessResource(
	getDatatable: () => string | undefined,
	getRole: () => string | undefined,
	getWorkspace: () => string | undefined = () => get(workspaceStore),
	/** False while the role is still being settled: a listing sent before that is read as the
	 * data table's default role, which is not the one about to be asked for. */
	getReady: () => boolean = () => true
) {
	const initialValue: DatatableAccess = {
		datatable: undefined,
		role: undefined,
		failed: false,
		error: undefined,
		schemas: [],
		canCreateSchema: false
	}
	const accessResource = resource(
		() => [getDatatable() ?? '', getRole() ?? '', getWorkspace() ?? '', getReady()] as const,
		latestOnly(
			async ([datatable, role, workspace, ready]: readonly [
				string,
				string,
				string,
				boolean
			]): Promise<DatatableAccess> => {
				const asked = {
					...initialValue,
					datatable: datatable || undefined,
					role: role || undefined
				}
				if (!ready) return accessResource.current
				if (!datatable || !workspace) return asked
				try {
					const tables = await WorkspaceService.listDataTableTables({
						workspace,
						datatableName: datatable,
						roleFor: datatable,
						role: role || undefined
					})
					const entry = tables.find((t) => t.datatable_name === datatable)
					return {
						...asked,
						failed: entry === undefined || entry.error !== undefined,
						error: entry?.error,
						schemas: Object.keys(entry?.schemas ?? {}).sort(),
						canCreateSchema: !!entry?.can_create_schema
					}
				} catch (e) {
					console.error('Failed to load datatable access:', e)
					return { ...asked, failed: true, error: (e as Error)?.message }
				}
			},
			() => accessResource.current
		),
		{ initialValue }
	)
	return accessResource
}

/**
 * Whether naming a role says anything: a data table without roles has none to pick, and one
 * whose single usable role is `admin` offers no choice.
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
