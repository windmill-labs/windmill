import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import type { DbType, DbInput } from '$lib/components/dbTypes'
import { getLanguageByResourceType, type ColumnDef } from '../utils'

export function makeCountQuery(
	dbType: DbType,
	table: string,
	whereClause: string | undefined = undefined,
	columnDefs: ColumnDef[]
): string {
	const payload: Record<string, unknown> = {
		table,
		columnDefs
	}

	if (whereClause !== undefined) {
		payload.whereClause = whereClause
	}

	return `-- WM_INTERNAL_DB_COUNT ${JSON.stringify(payload)}`
}

export function getCountInput(
	dbInput: DbInput,
	table: string,
	columnDefs: ColumnDef[],
	whereClause: string | undefined
): AppInput | undefined {
	if (
		(dbInput.type == 'ducklake' && !dbInput.ducklake) ||
		(dbInput.type == 'database' && !dbInput.resourcePath) ||
		!table ||
		!columnDefs?.length
	) {
		return undefined
	}
	const dbType = dbInput.type === 'ducklake' ? 'duckdb' : dbInput.resourceType

	const payload: Record<string, unknown> = {
		table,
		columnDefs
	}

	if (whereClause !== undefined) {
		payload.whereClause = whereClause
	}

	if (dbInput.type === 'ducklake') {
		payload.ducklake = dbInput.ducklake
	}

	const content = `-- WM_INTERNAL_DB_COUNT ${JSON.stringify(payload)}`

	const updateRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'inline',
		inlineScript: {
			content,
			language: getLanguageByResourceType(dbType),
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties:
					dbInput.type === 'database'
						? {
								database: {
									description: 'Database name',
									type: 'object',
									format: `resource-${dbType}`
								}
							}
						: {},
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
