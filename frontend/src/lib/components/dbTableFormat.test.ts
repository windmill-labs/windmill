import { describe, expect, it } from 'vitest'
import { cellStyle, formatSignificant, formatValue, UNIT_PRESETS } from './dbTableFormat'

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
		expect(formatValue('35412345', { digits: 3, unit: UNIT_PRESETS[2] })).toBe('35.4m\u00A0€')
	})

	it('leaves a value untouched when nothing applies to it', () => {
		expect(formatValue('abc', { digits: 3 })).toBeUndefined()
		expect(formatValue('N/A', { unit: { symbol: '$', position: 'before' } })).toBeUndefined()
		expect(formatValue(null, { unit: { symbol: '$', position: 'before' } })).toBeUndefined()
	})
})

describe('cellStyle', () => {
	it('layers every matching rule, a later one overriding what it sets', () => {
		const format = {
			rules: [
				{ condition: '', bold: true, bg: '#f3f4f6' },
				{ condition: '>= 10', bg: '#fef9c3' },
				{ condition: '>= 100', bg: '#fee2e2', text: '#991b1b' }
			]
		}
		expect(cellStyle('150', 'int4', format)).toEqual({
			bold: true,
			bg: '#fee2e2',
			text: '#991b1b'
		})
		expect(cellStyle('50', 'int4', format)).toEqual({ bold: true, bg: '#fef9c3' })
		expect(cellStyle('5', 'int4', format)).toEqual({ bold: true, bg: '#f3f4f6' })
		expect(cellStyle('refunded', 'text', { rules: [{ condition: '=paid', bold: true }] })).toBe(
			undefined
		)
	})
})
