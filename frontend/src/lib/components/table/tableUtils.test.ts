import { describe, it, expect } from 'vitest'
import { computeStructuredObjectsAndHeaders } from './tableUtils'

describe('computeStructuredObjectsAndHeaders', () => {
	it('respects an explicit headerOrder when a column name is integer-like (#9183)', () => {
		// "1234" must keep its specified position instead of jumping to the front,
		// the way Object.keys() orders integer-like keys.
		const headerOrder = ['Pokemon name', 'Type', '1234', 'Main strength']
		const rows = [
			{ 'Pokemon name': 'Pikachu', Type: 'Electric', '1234': 'nil', 'Main strength': 'Speed' }
		]
		const [headers] = computeStructuredObjectsAndHeaders(rows, headerOrder)
		expect(headers).toEqual(['Pokemon name', 'Type', '1234', 'Main strength'])
	})

	it('appends keys not present in headerOrder, after the explicit ones', () => {
		const [headers] = computeStructuredObjectsAndHeaders([{ a: 1, b: 2, c: 3 }], ['a', 'b'])
		expect(headers).toEqual(['a', 'b', 'c'])
	})

	it('keeps the existing object-key behavior when no headerOrder is given', () => {
		const [headers] = computeStructuredObjectsAndHeaders([{ x: 1, y: 2 }])
		expect(headers).toEqual(['x', 'y'])
	})
})
