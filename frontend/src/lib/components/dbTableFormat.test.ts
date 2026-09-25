import { describe, expect, it } from 'vitest'
import { cellStyle, formatValue, UNIT_PRESETS } from './dbTableFormat'

describe('formatValue', () => {
	it('sets the decimals, groups thousands and compacts', () => {
		expect(formatValue('1234567.891', { decimals: 2 })).toBe('1234567.89')
		expect(formatValue('1234567.891', { decimals: 2, thousands: true })).toBe('1,234,567.89')
		expect(formatValue(3, { decimals: 2 })).toBe('3.00')
		expect(formatValue('35412345', { compact: true, decimals: 1 })).toBe('35.4M')
		// Past what a float holds: numeric columns arrive as text and keep every digit.
		expect(formatValue('12345678901234567890.5', { thousands: true })).toBe(
			'12,345,678,901,234,567,890.5'
		)
	})

	it('places the unit on its side, the sign before a leading one', () => {
		expect(formatValue('12.5', { unit: { symbol: '%', position: 'after' } })).toBe('12.5%')
		expect(formatValue(-5000, { unit: { symbol: '$', position: 'before' } })).toBe('-$5000')
		expect(formatValue('35412345', { compact: true, decimals: 1, unit: UNIT_PRESETS[2] })).toBe(
			'35.4M\u00A0€'
		)
	})

	it('leaves a value untouched when nothing applies to it', () => {
		expect(formatValue('1234', { align: 'left' })).toBeUndefined()
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
