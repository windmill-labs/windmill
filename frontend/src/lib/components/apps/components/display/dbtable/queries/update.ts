import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import type { Preview } from '$lib/gen'

import { getLanguageByResourceType, type ColumnDef, buildParamters } from '../utils'

function updateWithAllValues(
	table: string,
	column: ColumnDef,
	columns: ColumnDef[],
	dbType: Preview.language
) {
	let query = buildParamters(
		[
			{
				field: 'valueToUpdate',
				datatype: column.datatype
			},
			...columns
		],
		dbType
	)

	query += `\n`

	switch (dbType) {
		case 'postgresql': {
			const conditions = columns
				.map((c, i) => `${c.field} = $${i + 2}::text::${c.datatype} `)
				.join(' AND ')

			query += `\nUPDATE ${table} SET ${column.field} = $1::text::${column.datatype} WHERE ${conditions}	RETURNING 1`
			return query
		}
		case 'mysql': {
			const conditions = columns.map((c) => `${c.field} = :${c.field}`).join(' AND ')
			query += `\nUPDATE ${table} SET ${column.field} = :value_to_update WHERE ${conditions}`
			return query
		}
		case 'mssql': {
			const conditions = columns.map((c) => `${c.field} = @${c.field}`).join(' AND ')
			query += `\nUPDATE ${table} SET ${column.field} = @value_to_update WHERE ${conditions}`
			return query
		}
		default:
			throw new Error('Unsupported database type')
	}
}

export function getUpdateInput(
	resource: string,
	table: string,
	column: ColumnDef,
	columns: ColumnDef[],
	dbType: Preview.language
): AppInput | undefined {
	if (!resource || !table) {
		return undefined
	}

	const updateRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: updateWithAllValues(table, column, columns, dbType),
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
		fields: {
			database: {
				type: 'static',
				value: resource,
				fieldType: 'object',
				format: `resource-${dbType}`
			}
		},
		type: 'runnable',
		fieldType: 'object'
	}

	return updateQuery
}
