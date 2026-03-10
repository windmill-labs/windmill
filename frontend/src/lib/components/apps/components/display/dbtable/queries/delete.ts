import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import type { DbType, DbInput } from '$lib/components/dbTypes'
import { getLanguageByResourceType, type ColumnDef } from '../utils'

export function makeDeleteQuery(table: string, columns: ColumnDef[], dbType: DbType) {
	const payload = { table, columns }
	return `-- WM_INTERNAL_DB_DELETE ${JSON.stringify(payload)}`
}

export function getDeleteInput(
	dbInput: DbInput,
	table: string,
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

	const payload: Record<string, unknown> = { table, columns }
	if (dbInput.type === 'ducklake') {
		payload.ducklake = dbInput.ducklake
	}

	const content = `-- WM_INTERNAL_DB_DELETE ${JSON.stringify(payload)}`

	const deleteRunnable: RunnableByName = {
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

	const deleteQuery: AppInput = {
		runnable: deleteRunnable,
		fields:
			dbInput.type == 'database'
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

	return deleteQuery
}
