import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import type { DbInput, DbType } from '$lib/components/dbTypes'
import { getLanguageByResourceType, type ColumnDef } from '../utils'

export function makeUpdateQuery(
	table: string,
	column: { datatype: string; field: string },
	columns: { datatype: string; field: string }[],
	dbType: DbType
) {
	const payload = { table, column, columns }
	return `-- WM_INTERNAL_DB_UPDATE ${JSON.stringify(payload)}`
}

export function getUpdateInput(
	dbInput: DbInput,
	table: string,
	column: ColumnDef,
	columns: ColumnDef[]
): AppInput | undefined {
	if (
		(dbInput.type == 'ducklake' && !dbInput.ducklake) ||
		(dbInput.type == 'database' && !dbInput.resourcePath) ||
		!table
	) {
		return undefined
	}
	const dbType = dbInput.type === 'ducklake' ? 'duckdb' : dbInput.resourceType

	const payload: Record<string, unknown> = { table, column, columns }
	if (dbInput.type === 'ducklake') {
		payload.ducklake = dbInput.ducklake
	}

	const content = `-- WM_INTERNAL_DB_UPDATE ${JSON.stringify(payload)}`

	const updateRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'inline',
		inlineScript: {
			content,
			language: getLanguageByResourceType(dbType),
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {},
				required: ['database'],
				type: 'object'
			}
		}
	}

	const updateQuery: AppInput = {
		runnable: updateRunnable,
		fields:
			dbInput.type === 'database'
				? {
						database: {
							type: 'static',
							value: dbInput.resourcePath.startsWith('datatable://')
								? dbInput.resourcePath
								: `$res:${dbInput.resourcePath}`,
							fieldType: 'object',
							format: `resource-${dbType}`
						}
					}
				: {},
		type: 'runnable',
		fieldType: 'object'
	}

	return updateQuery
}
