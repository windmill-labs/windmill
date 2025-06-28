import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import { buildParameters, type DbType } from '../utils'
import { getLanguageByResourceType, type ColumnDef, buildVisibleFieldList } from '../utils'

function makeSnowflakeSelectQuery(
	table: string,
	columnDefs: ColumnDef[],
	whereClause: string | undefined,
	options?: { limit?: number; offset?: number }
) {
	const limit = coerceToNumber(options?.limit) || 100
	const offset = coerceToNumber(options?.offset) || 0

	const headers: Array<{
		field: string
		datatype: string
	}> = [
		{
			field: 'quicksearch',
			datatype: 'text'
		}
	]

	let query = ''

	query += '\n'

	const filteredColumns = buildVisibleFieldList(columnDefs, 'snowflake')
	const selectClause = filteredColumns.join(', ')

	query += `SELECT ${selectClause} FROM ${table}`

	const quicksearchCondition = [
		'LENGTH(?) = 0',
		...filteredColumns.map((column) => {
			headers.push({
				field: 'quicksearch',
				datatype: 'text'
			})

			return `CONCAT(${column}) ILIKE CONCAT('%', ?, '%')`
		})
	].join(' OR ')

	if (whereClause) {
		query += ` WHERE ${whereClause} AND (${quicksearchCondition})`
	} else {
		query += ` WHERE ${quicksearchCondition}`
	}

	const orderBy = columnDefs.map((column) => {
		headers.push(
			{
				field: 'order_by',
				datatype: 'text'
			},
			{
				field: 'is_desc',
				datatype: 'boolean'
			},
			{
				field: 'order_by',
				datatype: 'text'
			},
			{
				field: 'is_desc',
				datatype: 'boolean'
			}
		)

		return `CASE WHEN ? = '${column.field}' AND ? = FALSE THEN "${column.field}" END ASC,
		CASE WHEN ? = '${column.field}' AND ? = TRUE THEN "${column.field}" END DESC`
	})

	query += ` ORDER BY ${orderBy.join(',\n')}`
	query += ` LIMIT ${limit} OFFSET ${offset}`
	query = buildParameters(headers, 'snowflake') + '\n' + query

	return query
}

export function makeSelectQuery(
	table: string,
	columnDefs: ColumnDef[],
	whereClause: string | undefined,
	dbType: DbType,
	options?: { limit?: number; offset?: number }
) {
	if (!table) throw new Error('Table name is required')
	let quicksearchCondition = ''

	let query = buildParameters(
		[
			{ field: 'limit', datatype: dbType === 'bigquery' ? 'integer' : 'int' },
			{ field: 'offset', datatype: dbType === 'bigquery' ? 'integer' : 'int' },
			{ field: 'quicksearch', datatype: dbType === 'bigquery' ? 'string' : 'text' },
			{ field: 'order_by', datatype: dbType === 'bigquery' ? 'string' : 'text' },
			{ field: 'is_desc', datatype: dbType === 'bigquery' ? 'bool' : 'boolean' }
		],
		dbType
	)

	query += '\n'

	const filteredColumns = buildVisibleFieldList(columnDefs, dbType)
	const selectClause = filteredColumns.join(', ')

	switch (dbType) {
		case 'mysql': {
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

			query += `SELECT ${selectClause} FROM ${table}`
			query += ` WHERE ${whereClause ? `${whereClause} AND` : ''} ${quicksearchCondition}`
			query += ` ORDER BY ${orderBy}`
			query += ` LIMIT :limit OFFSET :offset`

			break
		}

		case 'postgresql': {
			const orderBy = `
      ${columnDefs
				.map(
					(column) =>
						`
      (CASE WHEN $4 = '${column.field}' AND $5 IS false THEN "${column.field}"::text END),
      (CASE WHEN $4 = '${column.field}' AND $5 IS true THEN "${column.field}"::text END) DESC`
				)
				.join(',\n')}`

			quicksearchCondition = `($3 = '' OR CONCAT(${filteredColumns.join(
				', '
			)}) ILIKE '%' || $3 || '%')`

			query += `SELECT ${filteredColumns
				.map((column) => `${column}::text`)
				.join(', ')} FROM ${table}\n`
			query += ` WHERE ${whereClause ? `${whereClause} AND` : ''} ${quicksearchCondition}\n`
			query += ` ORDER BY ${orderBy}\n`
			query += ` LIMIT $1::INT OFFSET $2::INT`

			break
		}

		case 'ms_sql_server':
			// MSSQL uses CONCAT for string concatenation and supports OFFSET FETCH for pagination
			// Note: MSSQL does not have a built-in ILIKE function, so we use LIKE with a case-insensitive collation if needed
			//
			// Note 2: CONCAT in mssql requires 2 to 254 arguments. But we can't change this query without breaking
			// existing policies
			const orderBy = columnDefs
				.map((column) => {
					return `
(CASE WHEN @p4 = '${column.field}' AND @p5 = 0 THEN ${column.field} END) ASC,
(CASE WHEN @p4 = '${column.field}' AND @p5 = 1 THEN ${column.field} END) DESC`
				})
				.join(',\n')

			quicksearchCondition = ` (@p3 = '' OR CONCAT(${selectClause}) LIKE '%' + @p3 + '%')`

			query += `SELECT ${selectClause} FROM ${table}`
			query += ` WHERE ${whereClause ? `${whereClause} AND` : ''} ${quicksearchCondition}`
			query += ` ORDER BY ${orderBy}`
			query += ` OFFSET @p2 ROWS FETCH NEXT @p1 ROWS ONLY`
			break
		case 'snowflake': {
			return makeSnowflakeSelectQuery(table, columnDefs, whereClause, options)
		}
		case 'bigquery': {
			const orderBy = columnDefs
				.map((column) => {
					if (
						column.datatype === 'JSON' ||
						column.datatype.startsWith('STRUCT') ||
						column.datatype.startsWith('ARRAY') ||
						column.datatype === 'GEOGRAPHY'
					) {
						return `
(CASE WHEN @order_by = '${column.field}' AND @is_desc = false THEN TO_JSON_STRING(${column.field}) END) ASC,
(CASE WHEN @order_by = '${column.field}' AND @is_desc = true THEN TO_JSON_STRING(${column.field}) END) DESC`
					}
					return `
(CASE WHEN @order_by = '${column.field}' AND @is_desc = false THEN ${column.field} END) ASC,
(CASE WHEN @order_by = '${column.field}' AND @is_desc = true THEN ${column.field} END) DESC`
				})
				.join(',\n')

			const searchClause = filteredColumns
				.map((col) => {
					const def = columnDefs.find((c) => c.field === col.slice(1, -1))
					if (
						def?.datatype === 'JSON' ||
						def?.datatype.startsWith('STRUCT') ||
						def?.datatype.startsWith('ARRAY') ||
						def?.datatype === 'GEOGRAPHY'
					) {
						return `TO_JSON_STRING(${col})`
					}
					return `CAST(${col} AS STRING)`
				})
				.join(',')
			quicksearchCondition = ` (@quicksearch = '' OR REGEXP_CONTAINS(CONCAT(${searchClause}), '(?i)' || @quicksearch))`

			query += `SELECT ${selectClause} FROM ${table}`
			query += ` WHERE ${whereClause ? `${whereClause} AND` : ''} ${quicksearchCondition}`
			query += ` ORDER BY ${orderBy}`
			query += ` LIMIT @limit OFFSET @offset`
			break
		}

		default:
			throw new Error('Unsupported database type')
	}

	return query
}

function coerceToNumber(value: any): number {
	if (typeof value === 'number') {
		return value
	}
	if (typeof value === 'string') {
		return parseInt(value, 10)
	}
	return 0
}

export function getSelectInput(
	resource: string,
	table: string | undefined,
	columnDefs: ColumnDef[],
	whereClause: string | undefined,
	dbType: DbType,
	options?: { limit?: number; offset?: number }
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
			content: makeSelectQuery(table, columnDefs, whereClause, dbType, options),
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
		fieldType: 'object',
		hideRefreshButton: true
	}

	return getQuery
}
