import { ColumnIdentity, type ColumnDef } from './apps/components/display/dbtable/utils'
import type { IDbTableOps } from './dbOps'
import { matchesColumnFilter } from './dbTableFilters'
import type { DbType } from './dbTypes'

/** A query result has no column types, so they are read off the values: a column is a
 * number or a boolean only if every non-null value is. */
function inferDatatype(rows: Record<string, unknown>[], field: string): string {
	let kind: 'number' | 'boolean' | undefined
	for (const row of rows) {
		const v = row[field]
		if (v === null || v === undefined) continue
		const k = typeof v === 'number' ? 'number' : typeof v === 'boolean' ? 'boolean' : undefined
		if (!k || (kind && kind !== k)) return 'text'
		kind = k
	}
	return kind === 'number' ? 'numeric' : kind === 'boolean' ? 'boolean' : 'text'
}

function searchable(value: unknown): string {
	if (value === null || value === undefined) return ''
	return (typeof value === 'object' ? JSON.stringify(value) : String(value)).toLowerCase()
}

function compare(a: unknown, b: unknown): number {
	if (a === b) return 0
	if (a === null || a === undefined) return 1
	if (b === null || b === undefined) return -1
	if (typeof a === 'number' && typeof b === 'number') return a - b
	return searchable(a).localeCompare(searchable(b), undefined, { numeric: true })
}

/** Read-only table ops over rows already in memory, e.g. a SQL editor's result: searched,
 * filtered and sorted here rather than by a query. */
export function staticDbTableOps(
	rows: Record<string, unknown>[],
	dbType: DbType,
	tableKey: string
): IDbTableOps {
	const fields = [...new Set(rows.flatMap((r) => Object.keys(r)))]
	const colDefs: ColumnDef[] = fields.map((field) => ({
		field,
		datatype: inferDatatype(rows, field),
		defaultvalue: '',
		isprimarykey: false,
		isidentity: ColumnIdentity.No,
		isnullable: 'YES',
		isenum: false
	}))
	const datatypeOf = Object.fromEntries(colDefs.map((c) => [c.field, c.datatype]))

	function filtered(quicksearch: string, columnFilters: Record<string, unknown> = {}) {
		const q = quicksearch.toLowerCase()
		return rows.filter(
			(row) =>
				(!q || fields.some((f) => searchable(row[f]).includes(q))) &&
				Object.entries(columnFilters).every(([column, filter]) =>
					matchesColumnFilter(row[column], datatypeOf[column], filter)
				)
		)
	}

	return {
		dbType,
		tableKey,
		colDefs,
		getCount: async ({ quicksearch, columnFilters }) => filtered(quicksearch, columnFilters).length,
		getRows: async ({
			offset,
			limit,
			quicksearch,
			columnFilters,
			order_by,
			is_desc,
			explicitSort
		}) => {
			let result = filtered(quicksearch, columnFilters)
			// Unsorted, a result keeps the order its query gave it.
			if (explicitSort) {
				result = [...result].sort((a, b) => compare(a[order_by], b[order_by]))
				if (is_desc) result.reverse()
			}
			return result.slice(offset, offset + limit)
		}
	}
}
