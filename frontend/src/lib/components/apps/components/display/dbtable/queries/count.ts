import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import { Preview } from '$lib/gen'
import { buildParamters, type DbType } from '../utils'
import { getLanguageByResourceType, type ColumnDef, buildVisibleFieldList } from '../utils'

function makeCountQuery(
	dbType: DbType,
	table: string,
	whereClause: string | undefined = undefined,
	columnDefs: ColumnDef[]
): string {
	const wherePrefix = ' WHERE '
	const andCondition = ' AND '
	let quicksearchCondition = ''
	let query = buildParamters([{ field: 'quicksearch', datatype: 'text' }], dbType)

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
			query += `SELECT COUNT(*) as count FROM \`${table}\``
			break
		case 'postgresql':
			if (filteredColumns.length > 0) {
				quicksearchCondition += `($1 = '' OR CONCAT(${filteredColumns.join(
					', '
				)}) ILIKE '%' || $1 || '%')`
			} else {
				quicksearchCondition += `($1 = '' OR 1 = 1)`
			}
			query += `SELECT COUNT(*) as count FROM "${table}"`
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
		default:
			throw new Error('Unsupported database type')
	}

	if (whereClause) {
		query += `${wherePrefix}${quicksearchCondition}`
	} else {
		query += dbType === 'ms_sql_server' && !whereClause ? wherePrefix : andCondition
		query += quicksearchCondition
	}

	if (!whereClause && (dbType === Preview.language.MYSQL || dbType === 'postgresql')) {
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
