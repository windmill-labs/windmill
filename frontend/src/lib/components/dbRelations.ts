/** One row of the `ALL_FOREIGN_KEYS` marker: a single referencing column. */
export type RawAllForeignKeyRow = {
	fk_constraint_name: string
	source_schema: string
	source_table: string
	source_column: string
	target_schema: string
	target_table: string
	target_column: string
	ordinal: number | string
}

export type DbTableRef = { schema: string; table: string }

/** A foreign key, with its columns paired to the ones they point at by index. */
export type DbRelation = {
	constraint: string
	from: DbTableRef & { columns: string[] }
	to: DbTableRef & { columns: string[] }
}

export function tableKeyOf(ref: DbTableRef): string {
	return `${ref.schema}.${ref.table}`
}

export function columnKeyOf(ref: DbTableRef, column: string): string {
	return columnKeyIn(tableKeyOf(ref), column)
}

export function columnKeyIn(tableKey: string, column: string): string {
	return `${tableKey}.${column}`
}

/** Groups the flat rows into one relation per constraint, keeping the column
 * pairs in the constraint's declaration order (the query's `ordinal`). */
export function groupForeignKeyRows(rows: RawAllForeignKeyRow[]): DbRelation[] {
	const byConstraint = new Map<string, { relation: DbRelation; pairs: RawAllForeignKeyRow[] }>()

	for (const raw of rows) {
		const row = lowercaseKeys(raw)
		if (!row.source_table || !row.target_table) continue
		const key = `${row.source_schema}.${row.source_table}.${row.fk_constraint_name}`
		let entry = byConstraint.get(key)
		if (!entry) {
			entry = {
				relation: {
					constraint: row.fk_constraint_name,
					from: { schema: row.source_schema, table: row.source_table, columns: [] },
					to: { schema: row.target_schema, table: row.target_table, columns: [] }
				},
				pairs: []
			}
			byConstraint.set(key, entry)
		}
		entry.pairs.push(row)
	}

	return Array.from(byConstraint.values()).map(({ relation, pairs }) => {
		pairs.sort((a, b) => Number(a.ordinal) - Number(b.ordinal))
		relation.from.columns = pairs.map((p) => p.source_column)
		relation.to.columns = pairs.map((p) => p.target_column)
		return relation
	})
}

function lowercaseKeys(row: RawAllForeignKeyRow): RawAllForeignKeyRow {
	const out: any = {}
	for (const key of Object.keys(row)) out[key.toLowerCase()] = (row as any)[key]
	return out as RawAllForeignKeyRow
}

/** Everything the diagram needs to answer "what lights up when this is hovered".
 * Built once per fetched relation set, then read on every hover. */
export type RelationIndex = {
	/** Tables reachable from a table by one foreign key, either direction. */
	relatedTables: Map<string, Set<string>>
	/** Columns a column is paired with by a foreign key, either direction. */
	linkedColumns: Map<string, Set<string>>
	/** Every column taking part in a relation of a table, keyed by table. Both
	 * the table's own referencing/referenced columns and the far ends. */
	relatedColumns: Map<string, Set<string>>
	/** Columns that reference another table — rendered with a link marker. */
	referencingColumns: Set<string>
	/** Columns another table points at. */
	referencedColumns: Set<string>
	/** The table a column key belongs to. Both parts are dot-joined names, so
	 * splitting a column key back apart is not safe. */
	columnTable: Map<string, string>
}

export function buildRelationIndex(relations: DbRelation[]): RelationIndex {
	const index: RelationIndex = {
		relatedTables: new Map(),
		linkedColumns: new Map(),
		relatedColumns: new Map(),
		referencingColumns: new Set(),
		referencedColumns: new Set(),
		columnTable: new Map()
	}

	for (const relation of relations) {
		const fromTable = tableKeyOf(relation.from)
		const toTable = tableKeyOf(relation.to)

		add(index.relatedTables, fromTable, toTable)
		add(index.relatedTables, toTable, fromTable)

		const pairCount = Math.min(relation.from.columns.length, relation.to.columns.length)
		for (let i = 0; i < pairCount; i++) {
			const fromColumn = columnKeyOf(relation.from, relation.from.columns[i])
			const toColumn = columnKeyOf(relation.to, relation.to.columns[i])

			index.referencingColumns.add(fromColumn)
			index.referencedColumns.add(toColumn)
			index.columnTable.set(fromColumn, fromTable)
			index.columnTable.set(toColumn, toTable)

			add(index.linkedColumns, fromColumn, toColumn)
			add(index.linkedColumns, toColumn, fromColumn)

			// A self-referencing key would otherwise register only once.
			for (const table of [fromTable, toTable]) {
				add(index.relatedColumns, table, fromColumn)
				add(index.relatedColumns, table, toColumn)
			}
		}
	}

	return index
}

function add(map: Map<string, Set<string>>, key: string, value: string) {
	let set = map.get(key)
	if (!set) map.set(key, (set = new Set()))
	set.add(value)
}
