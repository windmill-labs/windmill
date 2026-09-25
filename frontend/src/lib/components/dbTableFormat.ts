import { matchesColumnFilter } from './dbTableFilters'

/** How a column's values are displayed. Display only: filters, sorting, edits and copies keep
 * the raw value. */
export type ColumnFormat = {
	unit?: { symbol: string; position: 'before' | 'after' }
	/** Significant digits; undefined shows the value as is. */
	digits?: number
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
	{ bg: '#dc2626', text: '#ffffff' },
	{ bg: '#ea580c', text: '#ffffff' },
	{ bg: '#f59e0b', text: '#1f2937' },
	{ bg: '#16a34a', text: '#ffffff' },
	{ bg: '#0d9488', text: '#ffffff' },
	{ bg: '#2563eb', text: '#ffffff' },
	{ bg: '#7c3aed', text: '#ffffff' },
	{ bg: '#db2777', text: '#ffffff' },
	{ bg: '#374151', text: '#ffffff' },
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
const COMPACT_UNITS: [number, string][] = [
	[1e3, 'k'],
	[1e6, 'm'],
	[1e9, 'b'],
	[1e12, 't']
]

/** Previews read most values as text, so a number may arrive as a string. */
export function asNumber(value: unknown): number | undefined {
	if (typeof value === 'number') return Number.isFinite(value) ? value : undefined
	if (typeof value === 'string' && NUMERIC.test(value.trim())) return Number(value.trim())
	return undefined
}

const trimmed = (n: number, digits: number) => String(Number(n.toPrecision(digits)))

/** `n` to `digits` significant digits. When its integer part alone has more digits than
 * that, it is compacted with a k/m/b/t suffix: 35 412 345 at 3 digits is `35.4m`. */
export function formatSignificant(n: number, digits: number): string {
	const abs = Math.abs(n)
	if (abs < 10 ** digits) return trimmed(n, digits)
	for (let i = 0; i < COMPACT_UNITS.length; i++) {
		const [div, suffix] = COMPACT_UNITS[i]
		const next = COMPACT_UNITS[i + 1]
		const scaled = Number((n / div).toPrecision(digits))
		// Rounding can carry into the next unit: 999 950 at 3 digits is 1.00m, not 1000k.
		if (next && (Math.abs(scaled) >= 1000 || abs >= next[0])) continue
		return `${trimmed(n / div, digits)}${suffix}`
	}
	return trimmed(n, digits)
}

/** The text a cell shows for `value`, or undefined when the format leaves it as is. */
export function formatValue(value: unknown, format: ColumnFormat | undefined): string | undefined {
	if (!format || value === null || value === undefined) return undefined
	const n = asNumber(value)
	// A unit on text would read as a quantity: `N/A` must not become `$N/A`.
	if (n === undefined) return undefined
	const text = format.digits ? formatSignificant(n, format.digits) : String(value).trim()
	if (!format.unit?.symbol) return format.digits ? text : undefined
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
	return !format.unit?.symbol && !format.digits && !format.rules?.length
}
