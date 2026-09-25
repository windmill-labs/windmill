import { matchesColumnFilter } from './dbTableFilters'

/** How a column's values are displayed. Display only: filters, sorting, edits and copies keep
 * the raw value. */
export type ColumnFormat = {
	unit?: { symbol: string; position: 'before' | 'after' }
	/** Digits after the decimal point; undefined keeps those of the value. */
	decimals?: number
	/** Grouped by thousands (1,234,567) or shortened (1.2M); undefined writes 1234567. */
	notation?: 'thousands' | 'compact'
	/** Undefined aligns numbers right and anything else left. */
	align?: 'left' | 'right'
	/** The first rule whose condition matches styles the cell. */
	rules?: ColorRule[]
}

export type ColorRule = {
	/** In the search bar's filter syntax: `=paid`, `>= 4`, `pend`. Empty matches every cell. */
	condition: string
	bg?: string
	text?: string
	bold?: boolean
	italic?: boolean
}

/** Picked when adding a rule, so that one click sets a background and a text color that read
 * well together. */
export const RULE_PRESETS: { bg?: string; text: string }[] = [
	{ bg: '#fee2e2', text: '#991b1b' },
	{ bg: '#ffedd5', text: '#9a3412' },
	{ bg: '#fef3c7', text: '#92400e' },
	{ bg: '#dcfce7', text: '#166534' },
	{ bg: '#ccfbf1', text: '#115e59' },
	{ bg: '#dbeafe', text: '#1e40af' },
	{ bg: '#ede9fe', text: '#5b21b6' },
	{ bg: '#fce7f3', text: '#9d174d' },
	{ bg: '#f3f4f6', text: '#374151' },
	{ text: '#dc2626' },
	{ text: '#ea580c' },
	{ text: '#d97706' },
	{ text: '#16a34a' },
	{ text: '#0d9488' },
	{ text: '#2563eb' },
	{ text: '#7c3aed' },
	{ text: '#db2777' },
	{ text: '#9ca3af' }
]

// Each where it is usually written: 12.5%, $5, 5 €, £5, ¥5.
export const UNIT_PRESETS: { symbol: string; position: 'before' | 'after' }[] = [
	{ symbol: '%', position: 'after' },
	{ symbol: '$', position: 'before' },
	{ symbol: '\u00A0€', position: 'after' },
	{ symbol: '£', position: 'before' },
	{ symbol: '¥', position: 'before' }
]

const NUMERIC = /^-?(\d+\.?\d*|\.\d+)(e[+-]?\d+)?$/i
/** Previews read most values as text, so a number may arrive as a string. */
export function asNumber(value: unknown): number | undefined {
	if (typeof value === 'number') return Number.isFinite(value) ? value : undefined
	if (typeof value === 'string' && NUMERIC.test(value.trim())) return Number(value.trim())
	return undefined
}

/** Whether the numbers of the column are formatted at all. */
export function formatsNumbers(format: ColumnFormat | undefined): boolean {
	return !!format && (format.decimals !== undefined || !!format.notation || !!format.unit?.symbol)
}

/** The text a cell shows for `value`, or undefined when the format leaves it as is. */
export function formatValue(value: unknown, format: ColumnFormat | undefined): string | undefined {
	if (!format || value === null || value === undefined) return undefined
	const n = asNumber(value)
	// A unit on text would read as a quantity: `N/A` must not become `$N/A`.
	if (n === undefined) return undefined
	if (!formatsNumbers(format)) return undefined
	const text = new Intl.NumberFormat('en-US', {
		useGrouping: format.notation === 'thousands',
		notation: format.notation === 'compact' ? 'compact' : 'standard',
		minimumFractionDigits: format.decimals,
		// Left to the value, or to the compact notation's own rounding.
		maximumFractionDigits: format.decimals ?? (format.notation === 'compact' ? undefined : 100)
		// A string keeps the digits a float would lose: numeric columns are read as text.
	}).format(typeof value === 'string' ? (value.trim() as Intl.StringNumericLiteral) : n)
	if (!format.unit?.symbol) return text
	const { symbol, position } = format.unit
	if (position === 'after') return `${text}${symbol}`
	// A sign reads before a leading unit: -$5, not $-5.
	return text.startsWith('-') ? `-${symbol}${text.slice(1)}` : `${symbol}${text}`
}

export type CellStyle = Pick<ColorRule, 'bg' | 'text' | 'bold' | 'italic'>

/** Every rule `value` matches, layered in order: a later rule overrides only what it sets. */
export function cellStyle(
	value: unknown,
	datatype: string | undefined,
	format: ColumnFormat | undefined
): CellStyle | undefined {
	let style: CellStyle | undefined
	for (const rule of format?.rules ?? []) {
		const condition = rule.condition.trim()
		// No condition styles every cell.
		if (condition && !matchesColumnFilter(value, datatype, condition)) continue
		style = { ...style }
		if (rule.bg) style.bg = rule.bg
		if (rule.text) style.text = rule.text
		if (rule.bold) style.bold = true
		if (rule.italic) style.italic = true
	}
	return style
}

export function isEmptyFormat(format: ColumnFormat): boolean {
	return (
		!format.unit?.symbol &&
		format.decimals === undefined &&
		!format.notation &&
		!format.align &&
		!format.rules?.length
	)
}
