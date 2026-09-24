import { describe, expect, it } from 'vitest'
import {
	buildDbTableFilterSchema,
	dbColumnKind,
	renderColumnFilter,
	renderDbTableFilters,
	unescapeFreeText
} from './dbTableFilters'
import { ColumnIdentity, type ColumnDef } from './apps/components/display/dbtable/utils'

const col = (field: string, datatype: string): ColumnDef => ({
	field,
	datatype,
	defaultvalue: '',
	isprimarykey: false,
	isidentity: ColumnIdentity.No,
	isnullable: 'YES',
	isenum: false
})

describe('dbColumnKind', () => {
	it('does not mistake interval for an integer', () => {
		expect(dbColumnKind('interval')).toBe('text')
		expect(dbColumnKind('int4')).toBe('number')
		expect(dbColumnKind('bigint unsigned')).toBe('number')
		expect(dbColumnKind('bit')).toBe('boolean')
	})
})

describe('renderColumnFilter', () => {
	it('matches text case-insensitively without LIKE wildcards', () => {
		expect(renderColumnFilter('name', 'text', "50%_O'k", 'postgresql')).toBe(
			`strpos(lower(CAST("name" AS TEXT)), '50%_o''k') > 0`
		)
	})

	it('treats a leading = as an exact match', () => {
		expect(renderColumnFilter('name', 'varchar', '=Bob', 'mysql')).toBe("`name` = 'Bob'")
	})

	it('compares numbers and never passes a malformed one through', () => {
		expect(renderColumnFilter('id', 'int4', '>= 10', 'postgresql')).toBe('"id" >= 10')
		expect(renderColumnFilter('id', 'int4', '!=3', 'postgresql')).toBe('"id" <> 3')
		expect(renderColumnFilter('id', 'int4', '1; DROP TABLE x', 'postgresql')).toBe('1 = 0')
	})
})

describe('unescapeFreeText', () => {
	it('drops the escapes the search bar puts on typed spaces', () => {
		expect(unescapeFreeText('number\\ 77\\')).toBe('number 77')
	})
})

describe('renderDbTableFilters', () => {
	it('maps sanitized keys back to their column and ignores the free text', () => {
		const columns = [col('first name', 'text'), col('id', 'int4')]
		const schema = buildDbTableFilterSchema(columns)
		expect(schema.keyOfColumn['first name']).toBe('first_name')
		expect(
			renderDbTableFilters(
				{ first_name: 'al\u00A0b', id: '2', _default_: 'free' },
				schema,
				columns,
				'duckdb'
			)
		).toBe(`(contains(lower(CAST("first name" AS VARCHAR)), 'al b')) AND ("id" = 2)`)
	})
})
