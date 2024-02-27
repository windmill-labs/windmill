import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import { Preview } from '$lib/gen'
import { buildParamters } from '../utils'
import { getLanguageByResourceType, type ColumnDef, buildVisibleFieldList } from '../utils'

function makeSelectQuery(
	table: string,
	columnDefs: ColumnDef[],
	whereClause: string | undefined,
	dbType: Preview.language
) {
	if (!table) throw new Error('Table name is required')
	let quicksearchCondition = ''

	let query = buildParamters(
		[
			{ field: 'limit', datatype: 'int' },
			{ field: 'offset', datatype: 'int' },
			{ field: 'quicksearch', datatype: 'text' },
			{ field: 'order_by', datatype: 'text' },
			{ field: 'is_desc', datatype: 'boolean' }
		],
		dbType
	)

	query += '\n'

	const filteredColumns = buildVisibleFieldList(columnDefs, dbType)
	const selectClause = filteredColumns.join(', ')

	switch (dbType) {
		case Preview.language.MYSQL: {
			const orderBy = columnDefs
				.map((column) => {
					return `
CASE WHEN :order_by = '${column.field}' AND :is_desc IS false THEN \`${column.field}\` END,
CASE WHEN :order_by = '${column.field}' AND :is_desc IS true THEN \`${column.field}\` END DESC`
				})
				.join(',\n')

			quicksearchCondition = ` (:quicksearch = '' OR CONCAT_WS(' ', ${filteredColumns.join(
				', '
			)}) LIKE CONCAT('%', :quicksearch, '%'))`

			query += `SELECT ${selectClause} FROM \`${table}\``
			query += ` WHERE ${whereClause ? `${whereClause} AND` : ''} ${quicksearchCondition}`
			query += ` ORDER BY ${orderBy}`
			query += ` LIMIT :limit OFFSET :offset`

			break
		}

		case Preview.language.POSTGRESQL: {
			const orderBy = `
      ${columnDefs
				.map(
					(column) =>
						`
      (CASE WHEN $4 = '${column.field}' AND $5 IS false THEN "${column.field}"::text END),
      (CASE WHEN $4 = '${column.field}' AND $5 IS true THEN "${column.field}"::text END) DESC`
				)
				.join(',\n')}`

			quicksearchCondition = ` ($3 = '' OR "${table}"::text ILIKE '%' || $3 || '%')`
			query += `SELECT ${filteredColumns
				.map((column) => `${column}::text`)
				.join(', ')} FROM "${table}"`
			query += ` WHERE ${whereClause ? `${whereClause} AND` : ''} ${quicksearchCondition}`
			query += ` ORDER BY ${orderBy}`
			query += ` LIMIT $1::INT OFFSET $2::INT`

			break
		}

		case Preview.language.MSSQL:
			// MSSQL uses CONCAT for string concatenation and supports OFFSET FETCH for pagination
			// Note: MSSQL does not have a built-in ILIKE function, so we use LIKE with a case-insensitive collation if needed
			const orderBy = columnDefs
				.map((column) => {
					return `
(CASE WHEN @order_by = '${column.field}' AND @is_desc = 0 THEN ${column.field} END) ASC,
(CASE WHEN @order_by = '${column.field}' AND @is_desc = 1 THEN ${column.field} END) DESC`
				})
				.join(',\n')

			quicksearchCondition = ` (@quicksearch = '' OR CONCAT(${selectClause}) LIKE '%' + @quicksearch + '%')`

			query += `SELECT ${selectClause} FROM ${table}`
			query += ` WHERE ${whereClause ? `${whereClause} AND` : ''} ${quicksearchCondition}`
			query += ` ORDER BY ${orderBy}`
			query += ` OFFSET @offset ROWS FETCH NEXT @limit ROWS ONLY`
			break
		default:
			throw new Error('Unsupported database type')
	}

	return query
}

export function getSelectInput(
	resource: string,
	table: string | undefined,
	columnDefs: ColumnDef[],
	whereClause: string | undefined,
	dbType: Preview.language
): AppInput | undefined {
	if (!resource || !table || !columnDefs) {
		return undefined
	}

	if (columnDefs.length === 0) {
		return undefined
	}

	const getRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: makeSelectQuery(table, columnDefs, whereClause, dbType),
			language: getLanguageByResourceType(dbType)
		}
	}

	const getQuery: AppInput = {
		runnable: getRunnable,
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

	return getQuery
}
