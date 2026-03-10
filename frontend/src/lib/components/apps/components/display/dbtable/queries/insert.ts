import type { AppInput } from '$lib/components/apps/inputType'
import type { DbType, DbInput } from '$lib/components/dbTypes'
import { getLanguageByResourceType, type ColumnDef } from '../utils'

export function makeInsertQuery(table: string, columns: ColumnDef[], dbType: DbType) {
	if (!table) throw new Error('Table name is required')

	const payload = { table, columns }
	return `-- WM_INTERNAL_DB_INSERT ${JSON.stringify(payload)}`
}

export function getInsertInput(dbInput: DbInput, table: string, columns: ColumnDef[]): AppInput {
	const dbType = dbInput.type === 'ducklake' ? 'duckdb' : dbInput.resourceType

	const payload: Record<string, unknown> = { table, columns }
	if (dbInput.type === 'ducklake') {
		payload.ducklake = dbInput.ducklake
	}

	const content = `-- WM_INTERNAL_DB_INSERT ${JSON.stringify(payload)}`

	return {
		runnable: {
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
		},
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
}
