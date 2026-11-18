import { describe, expect, it } from 'vitest'
import { fieldsBlamedByInsertError } from './insertRowError'

const fields = [
	{ name: 'id', datatype: 'integer' },
	{ name: 'name', datatype: 'text' },
	{ name: 'born', datatype: 'date' },
	{ name: 'note', datatype: 'text' }
]

describe('fieldsBlamedByInsertError', () => {
	it('picks a column the error names', () => {
		expect(
			fieldsBlamedByInsertError(
				'null value in column "name" of relation "people" violates not-null constraint',
				fields,
				{}
			)
		).toEqual(['name'])
		expect(
			fieldsBlamedByInsertError("Incorrect date value: '' for column 'born' at row 1", fields, {})
		).toEqual(['born'])
	})

	it('falls back to the field holding the rejected value, narrowed by type', () => {
		const args = { id: 1, name: '', born: '', note: '' }
		expect(
			fieldsBlamedByInsertError('Cannot parse "" as date: premature end of input', fields, args)
		).toEqual(['born'])
		expect(
			fieldsBlamedByInsertError('invalid input syntax for type integer: "abc"', fields, {
				id: 'abc',
				name: 'abc'
			})
		).toEqual(['id'])
	})

	it('blames nothing it cannot tie to a field', () => {
		expect(fieldsBlamedByInsertError('permission denied for table people', fields, {})).toEqual([])
	})
})
