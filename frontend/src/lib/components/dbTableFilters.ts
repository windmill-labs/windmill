import { Hash, ToggleLeft, Type } from 'lucide-svelte'
import type { FilterSchema, FilterSchemaRec } from './FilterSearchbar.svelte'
import {
	renderDbContainsFilter,
	renderDbEqualityFilter,
	renderDbLiteral,
	renderDbQuotedIdentifier,
	type ColumnDef
} from './apps/components/display/dbtable/utils'
import type { DbType } from './dbTypes'

export type DbColumnKind = 'number' | 'boolean' | 'text'

/** Search bar key holding the free text, which stays the table's quicksearch. */
export const FREE_TEXT_KEY = '_default_'

/** The search bar is a contenteditable, so a typed space can reach its values
 * as a non-breaking space, which a database compares as a different character. */
function normalizeSpaces(text: string): string {
	return text.replace(/\u00A0/g, ' ')
}

/** The search bar escapes a typed space as `\ ` (a double space separates
 * terms) and hands the free text over still escaped, unlike column values. */
export function unescapeFreeText(text: string): string {
	return normalizeSpaces(text.replace(/\\(.)/g, '$1').replace(/\\$/, '')).trim()
}

const NUMERIC_TYPE =
	/^(u?(tiny|small|medium|big|huge)?int(eger)?\d*|(small|big)?serial\d*|numeric|decimal|dec|float\d*|double|real|number|money|smallmoney|bignumeric)\b/
const BOOLEAN_TYPE = /^(bool|boolean|bit)\b/

export function dbColumnKind(datatype: string | undefined): DbColumnKind {
	const t = (datatype ?? '').trim().toLowerCase()
	if (NUMERIC_TYPE.test(t)) return 'number'
	if (BOOLEAN_TYPE.test(t)) return 'boolean'
	return 'text'
}

const NUMERIC_FILTER = /^(>=|<=|!=|<>|>|<|=)?\s*(-?\d+(\.\d+)?([eE][+-]?\d+)?)$/

export type DbTableFilterSchema = {
	schema: FilterSchemaRec
	/** Search bar key → column name. The bar only recognises `\w+` keys, so
	 * other column names get a sanitized key. */
	columnOfKey: Record<string, string>
	keyOfColumn: Record<string, string>
}

export function buildDbTableFilterSchema(columns: ColumnDef[]): DbTableFilterSchema {
	const schema: FilterSchemaRec = { [FREE_TEXT_KEY]: { type: 'string', hidden: true } }
	const columnOfKey: Record<string, string> = {}
	const keyOfColumn: Record<string, string> = {}
	for (const col of columns) {
		if (!col.field) continue
		let base = col.field.replace(/\W+/g, '_') || 'column'
		if (base === FREE_TEXT_KEY) base = `${base}col`
		let key = base
		for (let i = 2; key in schema; i++) key = `${base}_${i}`
		columnOfKey[key] = col.field
		keyOfColumn[col.field] = key
		schema[key] = columnFilterSchema(col)
	}
	return { schema, columnOfKey, keyOfColumn }
}

function columnFilterSchema(col: ColumnDef): FilterSchema {
	const kind = dbColumnKind(col.datatype)
	const common = { label: col.field }
	switch (kind) {
		case 'boolean':
			return {
				...common,
				type: 'boolean',
				default: true,
				icon: ToggleLeft,
				description: col.datatype
			}
		case 'number':
			return {
				...common,
				type: 'string',
				icon: Hash,
				description: `${col.datatype} · equals, or compare with >, >=, <, <=, !=`
			}
		case 'text':
			return {
				...common,
				type: 'string',
				icon: Type,
				description: `${col.datatype} · contains (case-insensitive), prefix with = for an exact match`
			}
	}
}

/** SQL predicate for one column filter. A malformed number matches nothing
 * rather than being dropped, so the table never shows rows the filter excludes. */
export function renderColumnFilter(
	column: string,
	datatype: string | undefined,
	value: unknown,
	dbType: DbType
): string | undefined {
	if (value === undefined || value === null || value === '') return undefined
	switch (dbColumnKind(datatype)) {
		case 'boolean':
			return typeof value === 'boolean' ? renderDbEqualityFilter(column, value, dbType) : undefined
		case 'number': {
			const m = String(value).trim().match(NUMERIC_FILTER)
			if (!m) return '1 = 0'
			const op = m[1] === '!=' ? '<>' : (m[1] ?? '=')
			return `${renderDbQuotedIdentifier(column, dbType)} ${op} ${m[2]}`
		}
		case 'text': {
			const text = String(value)
			if (!text.startsWith('=')) return renderDbContainsFilter(column, text, dbType)
			// PostgreSQL `json` has no `=`: compare as the text the grid shows.
			if (dbType === 'postgresql' && /^jsonb?$/i.test((datatype ?? '').trim())) {
				const literal = renderDbLiteral(text.slice(1), dbType)!
				return `CAST(${renderDbQuotedIdentifier(column, dbType)} AS TEXT) = ${literal}`
			}
			return renderDbEqualityFilter(column, text.slice(1), dbType)
		}
	}
}

/** AND of every column filter, or undefined when none applies. The free text
 * is not part of it: it goes to the reads as their quicksearch. */
export function renderDbTableFilters(
	filters: Record<string, unknown>,
	{ columnOfKey }: DbTableFilterSchema,
	columns: ColumnDef[],
	dbType: DbType
): string | undefined {
	const predicates: string[] = []
	for (const [key, value] of Object.entries(filters)) {
		const column = columnOfKey[key]
		if (!column) continue
		const datatype = columns.find((c) => c.field === column)?.datatype
		const predicate = renderColumnFilter(
			column,
			datatype,
			typeof value === 'string' ? normalizeSpaces(value) : value,
			dbType
		)
		if (predicate) predicates.push(`(${predicate})`)
	}
	return predicates.length ? predicates.join(' AND ') : undefined
}

/** Search bar value that selects exactly the rows whose `column` equals `value`. */
export function exactColumnFilterValue(
	datatype: string | undefined,
	value: unknown
): string | boolean {
	switch (dbColumnKind(datatype)) {
		case 'boolean':
			// Previews read values as text, so a false key arrives as "false" or "f".
			return typeof value === 'string'
				? ['true', 't', '1', 'yes', 'y'].includes(value.trim().toLowerCase())
				: Boolean(value)
		case 'number':
			return String(value)
		case 'text':
			return `=${String(value)}`
	}
}

/** Column → filter value, for a source that filters its rows itself rather than in SQL. */
export function columnFiltersOf(
	filters: Record<string, unknown>,
	{ columnOfKey }: DbTableFilterSchema
): Record<string, unknown> {
	const out: Record<string, unknown> = {}
	for (const [key, value] of Object.entries(filters)) {
		const column = columnOfKey[key]
		if (!column || value === undefined || value === null || value === '') continue
		out[column] = typeof value === 'string' ? normalizeSpaces(value) : value
	}
	return out
}

/** In-memory counterpart of `renderColumnFilter`, with the same semantics: a null never
 * matches, and a malformed number matches nothing. */
export function matchesColumnFilter(
	value: unknown,
	datatype: string | undefined,
	filter: unknown
): boolean {
	if (value === null || value === undefined) return false
	switch (dbColumnKind(datatype)) {
		case 'boolean':
			return typeof filter !== 'boolean' || value === filter
		case 'number': {
			const m = String(filter).trim().match(NUMERIC_FILTER)
			if (!m) return false
			const a = Number(value)
			const b = Number(m[2])
			switch (m[1]) {
				case '>':
					return a > b
				case '>=':
					return a >= b
				case '<':
					return a < b
				case '<=':
					return a <= b
				case '!=':
				case '<>':
					return a !== b
				default:
					return a === b
			}
		}
		case 'text': {
			const text = typeof value === 'object' ? JSON.stringify(value) : String(value)
			const f = String(filter)
			return f.startsWith('=') ? text === f.slice(1) : text.toLowerCase().includes(f.toLowerCase())
		}
	}
}
