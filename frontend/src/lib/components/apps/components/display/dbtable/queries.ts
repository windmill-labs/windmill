import type { TableMetadata } from './utils'

export function makeMySQLQuery(
	tableMetadata: TableMetadata,
	table: string,
	whereClause: string | undefined = undefined
) {
	const filteredColumns = tableMetadata
		.filter((x) => x !== undefined)
		.map((column) => `\`${column?.field}\``)

	let selectClause = filteredColumns.join(', ')

	let orderBy = tableMetadata
		.map((column) => {
			return `
CASE WHEN :order_by = '${column.field}' AND :is_desc IS false THEN \`${column.field}\` END,
CASE WHEN :order_by = '${column.field}' AND :is_desc IS true THEN \`${column.field}\` END DESC`
		})
		.join(',\n')

	let query = `
-- :limit (int)
-- :offset (int)
-- :quicksearch (text)
-- :order_by (text)
-- :is_desc (boolean)

SELECT ${selectClause} FROM \`${table}\` where `

	if (whereClause) {
		query += ` ${whereClause} AND `
	}
	query += ` (:quicksearch = '' OR  CONCAT_WS(' ', ${filteredColumns.join(
		', '
	)}) LIKE CONCAT('%', :quicksearch, '%'))`
	query += ` ORDER BY ${orderBy}`

	query += ` LIMIT :limit OFFSET :offset`

	return query
}
