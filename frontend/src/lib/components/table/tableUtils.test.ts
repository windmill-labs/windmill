import { describe, expect, it } from 'vitest'

import { computeStructuredObjectsAndHeaders } from './tableUtils'

describe('computeStructuredObjectsAndHeaders', () => {
	const rows = [
		{ 'Pokemon name': 'Pikachu', Type: 'Electric', '1234': 'nil', 'Main strength': 'Speed' },
		{ 'Pokemon name': 'Charizard', Type: 'Fire/Flying', '1234': 'nil', 'Main strength': 'Attack' }
	]

	it('respects the explicit header order even with integer-like column names', () => {
		const [headers] = computeStructuredObjectsAndHeaders(rows, [
			'Pokemon name',
			'Type',
			'1234',
			'Main strength'
		])
		expect(headers).toEqual(['Pokemon name', 'Type', '1234', 'Main strength'])
	})

	it('keeps extra keys not present in the override (appended at the end)', () => {
		const [headers] = computeStructuredObjectsAndHeaders(
			[{ 'Pokemon name': 'Pikachu', Type: 'Electric', extra: 'x' }],
			['Pokemon name', 'Type']
		)
		expect(headers).toEqual(['Pokemon name', 'Type', 'extra'])
	})

	it('without an override, integer-like keys are reordered by Object.keys (documented baseline)', () => {
		const [headers] = computeStructuredObjectsAndHeaders(rows)
		expect(headers).toEqual(['1234', 'Pokemon name', 'Type', 'Main strength'])
	})

	it('returns empty arrays for non-array input', () => {
		expect(computeStructuredObjectsAndHeaders({} as any)).toEqual([[], []])
	})
})
