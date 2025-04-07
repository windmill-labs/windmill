import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import { buildParameters, type DbType } from '../utils'
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
			dbType === 'bigquery')
	) {
		query = query.replace(`${andCondition}`, wherePrefix)
	}

	return query
}

export function getCountInput(
	resource: string,
	table: string,
	resourceType: DbType,
	columnDefs: ColumnDef[],
	whereClause: string | undefined
): AppInput | undefined {
	if (!resource || !table || !columnDefs) {
		// Return undefined if resource or table is not defined

		return undefined
	}

	const query = makeCountQuery(resourceType, table, whereClause, columnDefs)

	const updateRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: query,
			language: getLanguageByResourceType(resourceType),
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {
					database: {
						description: 'Database name',
						type: 'object',
						format: `resource-${resourceType}`
					}
				},
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
				format: `resource-${resourceType}`
			}
		},
		type: 'runnable',
		fieldType: 'object'
	}

	return updateQuery
}
