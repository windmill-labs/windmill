import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import type { DbType, DbInput } from '$lib/components/dbTypes'
import { getLanguageByResourceType, type ColumnDef, buildParameters } from '../utils'

export function makeDeleteQuery(table: string, columns: ColumnDef[], dbType: DbType) {
	let query = buildParameters(
		dbType === 'snowflake' ? columns.flatMap((c) => [c, c]) : columns,
		dbType
	)

	switch (dbType) {
		case 'postgresql': {
			const conditions = columns
				.map(
					(c, i) =>
						`($${i + 1}::text::${c.datatype} IS NULL AND ${c.field} IS NULL OR ${c.field} = $${
							i + 1
						}::text::${c.datatype})`
				)
				.join('\n    AND ')

			query += `\nDELETE FROM ${table} \nWHERE ${conditions} RETURNING 1;`
			return query
		}
		case 'mysql': {
			const conditions = columns
				.map((c) => `(:${c.field} IS NULL AND ${c.field} IS NULL OR ${c.field} = :${c.field})`)
				.join('\n    AND ')
			query += `\nDELETE FROM ${table} \nWHERE ${conditions}`
			return query
		}
		case 'ms_sql_server': {
			const conditions = columns
				.map((c, i) => `(@p${i + 1} IS NULL AND ${c.field} IS NULL OR ${c.field} = @p${i + 1})`)
				.join('\n    AND ')
			query += `\nDELETE FROM ${table} \nWHERE ${conditions}`
			return query
		}
		case 'snowflake': {
			const conditions = columns
				.map((c, i) => `(? = 'null' AND ${c.field} IS NULL OR ${c.field} = ?)`)
				.join('\n    AND ')
			query += `\nDELETE FROM ${table} \nWHERE ${conditions}`
			return query
		}
		case 'bigquery': {
			const conditions = columns
				.map(
					(c, i) =>
						`(CAST(@${c.field} AS STRING) = 'null' AND ${c.field} IS NULL OR ${c.field} = @${c.field})`
				)
				.join('\n    AND ')
			query += `\nDELETE FROM ${table} \nWHERE ${conditions}`
			return query
		}
		case 'duckdb': {
			const conditions = columns
				.map((c) => `($${c.field} IS NULL AND ${c.field} IS NULL OR ${c.field} = $${c.field})`)
				.join('\n    AND ')
			query += `\nDELETE FROM ${table} \nWHERE ${conditions}`
			return query
		}
		default:
			throw new Error('Unsupported database type')
	}
}

export function buildDeleteMarker(
	table: string,
	columns: ColumnDef[],
	dbType: DbType,
	ducklake?: string
): string {
	const params: Record<string, unknown> = {
		table,
		columns: columns.map((c) => ({ field: c.field, datatype: c.datatype })),
		db_type: dbType
	}
	if (ducklake) params.ducklake = ducklake
	return `-- WM_INTERNAL_DB_DELETE_SCRIPT\n${JSON.stringify(params)}`
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
	const ducklake = dbInput.type === 'ducklake' ? dbInput.ducklake : undefined
	const query = buildDeleteMarker(table, columns, dbType, ducklake)
	const deleteRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'inline',
		inlineScript: {
			content: query,
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
