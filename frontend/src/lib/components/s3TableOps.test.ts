import { describe, expect, it } from 'vitest'
import { datatypeOf, jsonTableRows, serverTableFormat } from './s3TableOps'
import { dbColumnKind } from './dbTableFilters'

describe('serverTableFormat', () => {
	it('pages delimited text and newline-delimited JSON through the csv reader', () => {
		expect(serverTableFormat('a/b.parquet')).toBe('parquet')
		for (const key of ['a.csv', 'a.TSV', 'a.jsonl', 'a.ndjson']) {
			expect(serverTableFormat(key)).toBe('csv')
		}
	})

	it('leaves plain JSON to the browser', () => {
		expect(serverTableFormat('a.json')).toBeUndefined()
		expect(serverTableFormat('a.txt')).toBeUndefined()
	})
})

describe('datatypeOf', () => {
	it('maps Arrow types to the kinds the grid filters by', () => {
		expect(dbColumnKind(datatypeOf('Int64'))).toBe('number')
		expect(dbColumnKind(datatypeOf('UInt8'))).toBe('number')
		expect(dbColumnKind(datatypeOf('Float64'))).toBe('number')
		expect(dbColumnKind(datatypeOf('Decimal128(21, 1)'))).toBe('number')
		expect(dbColumnKind(datatypeOf('Boolean'))).toBe('boolean')
		expect(dbColumnKind(datatypeOf('Utf8View'))).toBe('text')
	})

	it('names a nested type without its field list', () => {
		expect(datatypeOf('List(Field { name: "item", data_type: Utf8 })')).toBe('list')
		expect(datatypeOf('Timestamp(Microsecond, None)')).toBe('timestamp')
	})
})

describe('jsonTableRows', () => {
	it('keeps objects and wraps other elements in a value column', () => {
		expect(jsonTableRows([{ a: 1 }, 2, null])).toEqual([{ a: 1 }, { value: 2 }, { value: null }])
	})

	it('has no table for an empty array or a non-array', () => {
		expect(jsonTableRows([])).toBeUndefined()
		expect(jsonTableRows({ a: 1 })).toBeUndefined()
	})
})
