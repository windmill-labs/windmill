import { describe, expect, it } from 'vitest'
import { cellColors, formatSignificant, formatValue } from './dbTableFormat'

describe('formatSignificant', () => {
	it('rounds to significant digits, compacting only past them', () => {
		expect(formatSignificant(35_412_345, 3)).toBe('35.4m')
		expect(formatSignificant(1234, 4)).toBe('1234')
		expect(formatSignificant(1234, 3)).toBe('1.23k')
		expect(formatSignificant(3.14159, 3)).toBe('3.14')
		expect(formatSignificant(0.012345, 2)).toBe('0.012')
		expect(formatSignificant(-2_500_000_000, 2)).toBe('-2.5b')
	})

	it('carries a rounded value into the next unit', () => {
		expect(formatSignificant(999_950, 3)).toBe('1m')
	})
})

describe('formatValue', () => {
	it('places the unit on its side, the sign before a leading one', () => {
		expect(formatValue('12.5', { unit: { symbol: '%', position: 'after' } })).toBe('12.5%')
		expect(formatValue(-5, { unit: { symbol: '$', position: 'before' } })).toBe('-$5')
		expect(formatValue('35412345', { digits: 3, unit: { symbol: '€', position: 'before' } })).toBe(
			'€35.4m'
		)
	})

	it('leaves a value untouched when nothing applies to it', () => {
		expect(formatValue('abc', { digits: 3 })).toBeUndefined()
		expect(formatValue(null, { unit: { symbol: '$', position: 'before' } })).toBeUndefined()
	})
})

describe('cellColors', () => {
	it('uses the first matching rule, in the filter syntax', () => {
		const format = {
			rules: [
				{ condition: '>= 100', bg: '#fee2e2' },
				{ condition: '>= 10', bg: '#fef9c3' }
			]
		}
		expect(cellColors('150', 'int4', format)?.bg).toBe('#fee2e2')
		expect(cellColors('50', 'int4', format)?.bg).toBe('#fef9c3')
		expect(cellColors('5', 'int4', format)).toBeUndefined()
		expect(
			cellColors('refunded', 'text', { rules: [{ condition: '=refunded', text: '#b91c1c' }] })
		).toEqual({ bg: undefined, text: '#b91c1c' })
	})
})
