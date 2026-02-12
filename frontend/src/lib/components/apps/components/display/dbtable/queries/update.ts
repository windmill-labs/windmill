import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import { wrapDucklakeQuery } from '../../../../../ducklake'
import type { DbInput, DbType } from '$lib/components/dbTypes'
import { getLanguageByResourceType, type ColumnDef, buildParameters } from '../utils'

export function makeUpdateQuery(
	table: string,
	column: { datatype: string; field: string },
	columns: { datatype: string; field: string }[],
	dbType: DbType
) {
	let query = buildParameters(
		[
			{ field: 'value_to_update', datatype: column.datatype },
			...(dbType === 'snowflake' ? columns.flatMap((c) => [c, c]) : columns)
		],
		dbType
	)

	query += `\n`

	switch (dbType) {
		case 'postgresql': {
			const conditions = columns
				.map(
					(c, i) =>
						`($${i + 2}::text::${c.datatype} IS NULL AND ${c.field} IS NULL OR ${c.field} = $${
							i + 2
						}::text::${c.datatype})`
				)
				.join('\n    AND ')

			query += `\nUPDATE ${table} SET ${column.field} = $1::text::${column.datatype} \nWHERE ${conditions}	RETURNING 1`
			return query
		}
		case 'mysql': {
			const conditions = columns
				.map((c) => `(:${c.field} IS NULL AND ${c.field} IS NULL OR ${c.field} = :${c.field})`)
				.join('\n    AND ')
			query += `\nUPDATE ${table} SET ${column.field} = :value_to_update \nWHERE ${conditions}`
			return query
		}
		case 'ms_sql_server': {
			const conditions = columns
				.map((c, i) => `(@p${i + 2} IS NULL AND ${c.field} IS NULL OR ${c.field} = @p${i + 2})`)
				.join('\n    AND ')
			query += `\nUPDATE ${table} SET ${column.field} = @p1 \nWHERE ${conditions}`
			return query
		}
		case 'snowflake': {
			const conditions = columns
				.map((c, i) => `(? = 'null' AND ${c.field} IS NULL OR ${c.field} = ?)`)
				.join('\n    AND ')
			query += `\nUPDATE ${table} SET ${column.field} = ? \nWHERE ${conditions}`
			return query
		}
		case 'bigquery': {
			const conditions = columns
				.map(
					(c, i) =>
						`(CAST(@${c.field} AS STRING) = 'null' AND ${c.field} IS NULL OR ${c.field} = @${c.field})`
				)
				.join('\n    AND ')
			query += `\nUPDATE ${table} SET ${column.field} = @value_to_update \nWHERE ${conditions}`
			return query
		}
		case 'duckdb': {
			const conditions = columns
				.map((c) => `($${c.field} IS NULL AND ${c.field} IS NULL OR ${c.field} = $${c.field})`)
				.join('\n    AND ')
			query += `\nUPDATE ${table} SET ${column.field} = $value_to_update \nWHERE ${conditions}`
			return query
		}
		default:
			throw new Error('Unsupported database type')
	}
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
	let query = makeUpdateQuery(table, column, columns, dbType)
	if (dbInput.type === 'ducklake') query = wrapDucklakeQuery(query, dbInput.ducklake)

	const updateRunnable: RunnableByName = {
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
