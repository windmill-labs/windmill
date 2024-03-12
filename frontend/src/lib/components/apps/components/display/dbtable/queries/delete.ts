import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import { getLanguageByResourceType, type ColumnDef, buildParameters, type DbType } from '../utils'

function deleteWithAllValues(table: string, columns: ColumnDef[], dbType: DbType) {
	let query = buildParameters(columns, dbType)

	switch (dbType) {
		case 'postgresql': {
			const conditions = columns
				.map((c, i) => `${c.field} = $${i + 1}::text::${c.datatype} `)
				.join(' AND ')

			query += `\nDELETE FROM ${table} WHERE ${conditions} RETURNING 1;`
			return query
		}
		case 'mysql': {
			const conditions = columns.map((c) => `${c.field} = :${c.field}`).join(' AND ')
			query += `\nDELETE FROM ${table} WHERE ${conditions}`
			return query
		}
		case 'ms_sql_server': {
			const conditions = columns.map((c, i) => `${c.field} = @p${i + 1} `).join(' AND ')
			query += `\nDELETE FROM ${table} WHERE ${conditions}`
			return query
		}
		case 'snowflake': {
			const conditions = columns.map((c, i) => `${c.field} = ? `).join(' AND ')
			query += `\nDELETE FROM ${table} WHERE ${conditions}`
			return query
		}
		case 'bigquery': {
			const conditions = columns.map((c, i) => `${c.field} = @${c.field}`).join(' AND ')
			query += `\nDELETE FROM ${table} WHERE ${conditions}`
			return query
		}
		default:
			throw new Error('Unsupported database type')
	}
}

export function getDeleteInput(
	resource: string,
	table: string,
	columns: ColumnDef[],
	dbType: DbType
): AppInput | undefined {
	if (!resource || !table) {
		return undefined
	}

	const deleteRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: deleteWithAllValues(table, columns, dbType),
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

	return deleteQuery
}
