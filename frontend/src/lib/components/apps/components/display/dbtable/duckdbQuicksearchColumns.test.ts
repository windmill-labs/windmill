import { describe, expect, it } from 'vitest'
import { buildVisibleFieldList, duckdbQuicksearchColumns, type ColumnDef } from './utils'

function col(field: string, datatype: string, extra: Partial<ColumnDef> = {}): ColumnDef {
	return { field, datatype, ...extra } as ColumnDef
}

describe('duckdbQuicksearchColumns', () => {
	it('casts list and array columns, and nothing else', () => {
		expect(
			duckdbQuicksearchColumns([
				col('id', 'VARCHAR'),
				col('tags', 'VARCHAR[]'),
				col('pos', 'INTEGER[3]'),
				col('meta', 'STRUCT(a INTEGER)')
			])
		).toBe('"id", CAST("tags" AS VARCHAR), CAST("pos" AS VARCHAR), "meta"')
	})

	// This byte-identity is what keeps the policy digest of an already-deployed
	// Database Studio app valid; a table with no list column must produce the
	// query it produced before quicksearch learned to cast anything.
	it('emits the plain column list when no column is a list', () => {
		const columnDefs = [col('id', 'VARCHAR'), col('n', 'INTEGER'), col('at', 'TIMESTAMP')]
		expect(duckdbQuicksearchColumns(columnDefs)).toBe(
			buildVisibleFieldList(columnDefs, 'duckdb').join(', ')
		)
	})

	it('skips ignored columns', () => {
		expect(
			duckdbQuicksearchColumns([col('id', 'VARCHAR'), col('tags', 'VARCHAR[]', { ignored: true })])
		).toBe('"id"')
	})
})
