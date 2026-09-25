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
	/** In the search bar's filter syntax: `=paid`, `>= 4`, `pend`. */
	condition: string
	bg?: string
	text?: string
}

export const UNIT_PRESETS: { symbol: string; position: 'before' | 'after' }[] = [
	{ symbol: '%', position: 'after' },
	{ symbol: '$', position: 'before' },
	{ symbol: '€', position: 'before' },
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
	let text = n !== undefined && format.digits ? formatSignificant(n, format.digits) : undefined
	if (!format.unit?.symbol) return text
	text ??= typeof value === 'object' ? JSON.stringify(value) : String(value)
	const { symbol, position } = format.unit
	if (position === 'after') return `${text}${symbol}`
	// A sign reads before a leading unit: -$5, not $-5.
	return text.startsWith('-') ? `-${symbol}${text.slice(1)}` : `${symbol}${text}`
}

/** The colors of the first rule `value` matches, if any. */
export function cellColors(
	value: unknown,
	datatype: string | undefined,
	format: ColumnFormat | undefined
): { bg?: string; text?: string } | undefined {
	for (const rule of format?.rules ?? []) {
		if (!rule.condition.trim() || (!rule.bg && !rule.text)) continue
		if (matchesColumnFilter(value, datatype, rule.condition.trim()))
			return { bg: rule.bg, text: rule.text }
	}
	return undefined
}

export function isEmptyFormat(format: ColumnFormat): boolean {
	return !format.unit?.symbol && !format.digits && !format.rules?.length
}
