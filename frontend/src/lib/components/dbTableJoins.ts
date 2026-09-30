import { ColumnIdentity, type ColumnDef } from './apps/components/display/dbtable/utils'

/** A column of a table the previewed one references by foreign key, shown beside its own
 * columns. The read LEFT JOINs the target on the key (see `select_source` in the backend's
 * query builders). */
export type DbTableJoin = {
	/** The previewed table's foreign key column. */
	sourceColumn: string
	/** The referenced table, as the query names it. */
	targetTable: string
	/** The column the key references. */
	targetColumn: string
	/** The target's column to show. */
	column: string
	datatype: string
}

/** A table the previewed one can join: one of its single-column foreign keys, and the columns
 * of the table that key references. */
export type DbJoinTarget = {
	sourceColumn: string
	targetTable: string
	targetColumn: string
	columns: { field: string; datatype: string }[]
}

/** The joined column's name in the grid and the query: its path from the previewed table. */
export function joinAlias(join: Pick<DbTableJoin, 'sourceColumn' | 'column'>): string {
	return `${join.sourceColumn}.${join.column}`
}

export function joinedColumnDef(join: DbTableJoin): ColumnDef {
	return {
		field: joinAlias(join),
		datatype: join.datatype,
		defaultvalue: '',
		isprimarykey: false,
		isidentity: ColumnIdentity.No,
		isnullable: 'YES',
		isenum: false
	}
}

/** The joins as the SELECT and COUNT markers take them. */
export function joinsPayload(joins: DbTableJoin[]) {
	return joins.map(({ sourceColumn, targetTable, targetColumn, column }) => ({
		sourceColumn,
		targetTable,
		targetColumn,
		column,
		alias: joinAlias({ sourceColumn, column })
	}))
}
