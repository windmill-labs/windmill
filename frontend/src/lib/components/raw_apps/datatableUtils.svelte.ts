import { resource } from 'runed'
import { workspaceStore, dbSchemas } from '$lib/stores'
import { WorkspaceService } from '$lib/gen'
import { getDbSchemas } from '$lib/components/apps/components/display/dbtable/metadata'
import { get } from 'svelte/store'

/**
 * Creates a resource that loads available datatables from the workspace
 */
export function createDatatablesResource() {
	return resource<string[]>([], async () => {
		const workspace = get(workspaceStore)
		if (!workspace) return []
		try {
			return await WorkspaceService.listDataTables({ workspace })
		} catch (e) {
			console.error('Failed to load datatables:', e)
			return []
		}
	})
}

/**
 * Creates a resource that loads schemas for a given datatable
 */
export function createSchemasResource(getDatatable: () => string | undefined) {
	return resource<string[]>([], async () => {
		const datatable = getDatatable()
		const workspace = get(workspaceStore)
		if (!datatable || !workspace) return []

		const resourcePath = `datatable://${datatable}`
		const schemas = get(dbSchemas)
		let dbSchema = schemas[resourcePath]

		if (!dbSchema) {
			try {
				await getDbSchemas('postgresql', resourcePath, workspace, schemas, (msg) =>
					console.error('Schema error:', msg)
				)
				dbSchema = get(dbSchemas)[resourcePath]
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
	return schemas.map((s) => ({
		value: s,
		label: s
	}))
}
