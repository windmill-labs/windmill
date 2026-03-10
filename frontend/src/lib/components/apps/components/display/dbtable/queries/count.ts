import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import { wrapDucklakeQuery } from '../../../../../ducklake'
import type { DbType, DbInput } from '$lib/components/dbTypes'
import { buildParameters } from '../utils'
import { getLanguageByResourceType, type ColumnDef, buildVisibleFieldList } from '../utils'

export function makeCountQuery(
	dbType: DbType,
	table: string,
	whereClause: string | undefined = undefined,
	columnDefs: ColumnDef[]
): string {
	const wherePrefix = ' WHERE '
	const andCondition = ' AND '
	let quicksearchCondition = ''
	let query = buildParameters(
		[{ field: 'quicksearch', datatype: dbType === 'bigquery' ? 'string' : 'text' }],
		dbType
	)

	query += '\n'

	if (whereClause) {
		quicksearchCondition = ` ${whereClause} AND `
	}

	const filteredColumns = buildVisibleFieldList(columnDefs, dbType)

	switch (dbType) {
		case 'mysql':
			if (filteredColumns.length > 0) {
				quicksearchCondition += ` (:quicksearch = '' OR CONCAT_WS(' ', ${filteredColumns.join(
					', '
				)}) LIKE CONCAT('%', :quicksearch, '%'))`
			} else {
				quicksearchCondition += ` (:quicksearch = '' OR 1 = 1)`
			}
			query += `SELECT COUNT(*) as count FROM ${table}`
			break
		case 'postgresql':
			if (filteredColumns.length > 0) {
				quicksearchCondition += `($1 = '' OR CONCAT(${filteredColumns.join(
					', '
				)}) ILIKE '%' || $1 || '%')`
			} else {
				quicksearchCondition += `($1 = '' OR 1 = 1)`
			}
			query += `SELECT COUNT(*) as count FROM ${table}`
			break
		case 'ms_sql_server':
			if (filteredColumns.length > 0) {
				quicksearchCondition += `(@p1 = '' OR CONCAT(${filteredColumns.join(
					', +'
				)}) LIKE '%' + @p1 + '%')`
			} else {
				quicksearchCondition += `(@p1 = '' OR 1 = 1)`
			}
			query += `SELECT COUNT(*) as count FROM [${table}]`
			break
		case 'snowflake': {
			query = ''

			if (filteredColumns.length > 0) {
				query += buildParameters(
					[
						{ field: 'quicksearch', datatype: 'text' },
						{ field: 'quicksearch', datatype: 'text' }
					],
					dbType
				)

				query += '\n'

				quicksearchCondition += `(? = '' OR CONCAT(${filteredColumns.join(
					', '
				)}) ILIKE '%' || ? || '%')`
			} else {
				query += buildParameters([{ field: 'quicksearch', datatype: 'text' }], dbType)

				query += '\n'

				quicksearchCondition += `(? = '' OR 1 = 1)`
			}

			query += `SELECT COUNT(*) as count FROM ${table}`
			break
		}
		case 'bigquery': {
			if (filteredColumns.length > 0) {
				const searchClause = filteredColumns
					.map((col) => {
						const def = columnDefs.find((c) => c.field === col.slice(1, -1))
						if (
							def?.datatype === 'JSON' ||
							def?.datatype.startsWith('STRUCT') ||
							def?.datatype.startsWith('ARRAY')
						) {
							return `TO_JSON_STRING(${col})`
						}
						return `${col}`
					})
					.join(',')
				quicksearchCondition += `(@quicksearch = '' OR REGEXP_CONTAINS(CONCAT(${searchClause}), '(?i)' || @quicksearch))`
			} else {
				quicksearchCondition += `(@quicksearch = '' OR 1 = 1)`
			}
			query += `SELECT COUNT(*) as count FROM \`${table}\``
			break
		}
		case 'duckdb':
			if (filteredColumns.length > 0) {
				quicksearchCondition += ` ($quicksearch = '' OR CONCAT(' ', ${filteredColumns.join(
					', '
				)}) LIKE CONCAT('%', $quicksearch, '%'))`
			} else {
				quicksearchCondition += ` ($quicksearch = '' OR 1 = 1)`
			}
			query += `SELECT COUNT(*) as count FROM ${table}`
			break
		default:
			throw new Error('Unsupported database type:' + dbType)
	}

	if (whereClause) {
		query += `${wherePrefix}${quicksearchCondition}`
	} else {
		query += dbType === 'ms_sql_server' && !whereClause ? wherePrefix : andCondition
		query += quicksearchCondition
	}

	if (
		!whereClause &&
		(dbType === 'mysql' ||
			dbType === 'postgresql' ||
			dbType === 'snowflake' ||
			dbType === 'bigquery' ||
			dbType === 'duckdb')
	) {
		query = query.replace(`${andCondition}`, wherePrefix)
	}

	return query
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
	let query = makeCountQuery(dbType, table, whereClause, columnDefs)
	if (dbInput.type === 'ducklake') query = wrapDucklakeQuery(query, dbInput.ducklake)

	const updateRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'inline',
		inlineScript: {
			content: query,
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
