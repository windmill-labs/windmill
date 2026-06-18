/**
 * A deliberately small, best-effort SQL engine for the benchmark datatable mock.
 *
 * This is NOT a real SQL implementation — it exists only so that writes a model
 * issues during an eval (`CREATE TABLE`, `INSERT`, `UPDATE`, `DELETE`, `DROP`)
 * become visible to its later reads (`list_datatables`, `get_datatable_table_schema`,
 * `SELECT`). Without that, a model that re-queries to verify a write sees stale
 * seed data, concludes the write failed, and loops until it exhausts its turns.
 *
 * It parses only the common statement shapes models produce. Anything it cannot
 * parse is a no-op success (it never throws) — behavioral evals assert that the
 * right statement was issued, not its exact data effects. Notable limits:
 *  - `SELECT` returns all rows of the referenced (or first) table — no WHERE
 *    filtering, projection, joins, or aggregation.
 *  - `WHERE` supports `col = value` predicates joined by `AND` only; an
 *    unparseable WHERE on UPDATE/DELETE affects zero rows (never the whole table).
 */

/** One seeded datatable table: its columns (col -> compact_type) and optional rows. */
export interface BenchmarkDatatableTableSeed {
	columns: Record<string, string>
	rows?: Record<string, unknown>[]
}

/** A seeded datatable: `datatable_name` plus a `schema -> table -> seed` map. */
export interface BenchmarkDatatableSeed {
	datatable_name: string
	schemas: {
		[schema: string]: {
			[table: string]: BenchmarkDatatableTableSeed
		}
	}
}

export interface DatatableSqlResult {
	rows: Record<string, unknown>[]
}

const DEFAULT_SCHEMA = 'public'

type ParsedRef = { schema: string; table: string }
type Predicate = { column: string; value: unknown }

/**
 * Apply one SQL statement to `datatable` IN PLACE and return the result rows.
 * SELECT returns the referenced/first table's rows; a mutation returns its
 * affected rows when it has a RETURNING clause, otherwise `[]`.
 */
export function applyDatatableSql(
	datatable: BenchmarkDatatableSeed,
	sql: string
): DatatableSqlResult {
	const statement = stripTrailingSemicolon(sql.trim())
	if (/^\s*(with|select)\b/i.test(statement)) {
		return { rows: selectRows(datatable, statement) }
	}
	if (/^\s*create\s+table\b/i.test(statement)) {
		return { rows: applyCreateTable(datatable, statement) }
	}
	if (/^\s*drop\s+table\b/i.test(statement)) {
		return { rows: applyDropTable(datatable, statement) }
	}
	if (/^\s*insert\s+into\b/i.test(statement)) {
		return { rows: applyInsert(datatable, statement) }
	}
	if (/^\s*update\b/i.test(statement)) {
		return { rows: applyUpdate(datatable, statement) }
	}
	if (/^\s*delete\s+from\b/i.test(statement)) {
		return { rows: applyDelete(datatable, statement) }
	}
	return { rows: [] }
}

// ============= Reads =============

function selectRows(
	datatable: BenchmarkDatatableSeed,
	sql: string
): Record<string, unknown>[] {
	const fromRef = sql.match(/\bfrom\s+([a-zA-Z_"][\w."]*)/i)?.[1]
	if (fromRef) {
		const catalog = catalogRows(datatable, fromRef)
		if (catalog) {
			return catalog
		}
	}
	const table = fromRef ? resolveTable(datatable, fromRef) : undefined
	const seed = table ?? firstTable(datatable)
	return seed?.rows ?? []
}

/**
 * Synthesize rows for a system-catalog query so a model verifying a `CREATE`/`DROP`
 * via `information_schema.tables` / `.columns` (or `pg_tables`) sees the current
 * tables/columns instead of fallback data. WHERE is not applied, so the model gets
 * the full set and finds (or no longer finds) the table it just changed.
 * Returns `undefined` for non-catalog refs so normal table resolution proceeds.
 */
function catalogRows(
	datatable: BenchmarkDatatableSeed,
	ref: string
): Record<string, unknown>[] | undefined {
	const normalized = ref.toLowerCase().replace(/"/g, '')
	const name = normalized.split('.').pop()
	const isCatalog = normalized.includes('information_schema.') || normalized.startsWith('pg_')
	if (!isCatalog) {
		return undefined
	}
	const tables = allTables(datatable)
	if (name === 'tables' || name === 'pg_tables') {
		return tables.map(({ schema, table }) => ({
			table_schema: schema,
			table_name: table,
			schemaname: schema,
			tablename: table
		}))
	}
	if (name === 'columns') {
		return tables.flatMap(({ schema, table, seed }) =>
			Object.entries(seed.columns).map(([column, type]) => ({
				table_schema: schema,
				table_name: table,
				column_name: column,
				data_type: type
			}))
		)
	}
	return undefined
}

function allTables(
	datatable: BenchmarkDatatableSeed
): { schema: string; table: string; seed: BenchmarkDatatableTableSeed }[] {
	return Object.entries(datatable.schemas).flatMap(([schema, tables]) =>
		Object.entries(tables).map(([table, seed]) => ({ schema, table, seed }))
	)
}

// ============= DDL =============

function applyCreateTable(
	datatable: BenchmarkDatatableSeed,
	sql: string
): Record<string, unknown>[] {
	const head = sql.match(
		/^\s*create\s+table\s+(?:if\s+not\s+exists\s+)?([a-zA-Z_"][\w."]*)/i
	)
	// The first top-level paren group is the column-definition list; using it (rather
	// than a greedy `(...)` capture) ignores any trailing `;`-separated statement.
	const columnText = extractParenGroups(sql)[0]
	if (!head || columnText === undefined) {
		return []
	}
	const { schema, table } = parseRef(head[1])
	const existing = datatable.schemas[schema]?.[table]
	if (existing) {
		return []
	}
	const columns: Record<string, string> = {}
	for (const rawDef of splitTopLevel(columnText)) {
		const def = rawDef.trim()
		if (!def || isTableConstraint(def)) {
			continue
		}
		const tokens = def.split(/\s+/)
		const column = unquoteIdentifier(tokens[0])
		if (!column) {
			continue
		}
		columns[column] = tokens[1] ?? 'text'
	}
	if (!datatable.schemas[schema]) {
		datatable.schemas[schema] = {}
	}
	datatable.schemas[schema][table] = { columns, rows: [] }
	return []
}

function applyDropTable(
	datatable: BenchmarkDatatableSeed,
	sql: string
): Record<string, unknown>[] {
	const match = sql.match(
		/^\s*drop\s+table\s+(?:if\s+exists\s+)?([a-zA-Z_"][\w."]*)/i
	)
	if (!match) {
		return []
	}
	const { schema, table } = parseRef(match[1])
	if (datatable.schemas[schema]?.[table]) {
		delete datatable.schemas[schema][table]
	}
	return []
}

// ============= DML =============

function applyInsert(
	datatable: BenchmarkDatatableSeed,
	sql: string
): Record<string, unknown>[] {
	const { body, returning } = splitOffReturning(sql)
	const match = body.match(
		/^\s*insert\s+into\s+([a-zA-Z_"][\w."]*)\s*(?:\(([^)]*)\))?\s*values\s*([\s\S]+)$/i
	)
	if (!match) {
		return []
	}
	const table = resolveTable(datatable, match[1])
	if (!table) {
		return []
	}
	const columns = match[2]
		? splitTopLevel(match[2]).map((entry) => unquoteIdentifier(entry.trim()))
		: Object.keys(table.columns)
	const inserted: Record<string, unknown>[] = []
	for (const tuple of extractParenGroups(match[3])) {
		const values = splitTopLevel(tuple).map((entry) => parseValue(entry))
		const row: Record<string, unknown> = {}
		columns.forEach((column, index) => {
			row[column] = values[index]
		})
		inserted.push(row)
	}
	table.rows ??= []
	table.rows.push(...inserted)
	return returning ? inserted : []
}

function applyUpdate(
	datatable: BenchmarkDatatableSeed,
	sql: string
): Record<string, unknown>[] {
	const { body, returning } = splitOffReturning(sql)
	const match = body.match(/^\s*update\s+([a-zA-Z_"][\w."]*)\s+set\s+([\s\S]+)$/i)
	if (!match) {
		return []
	}
	const table = resolveTable(datatable, match[1])
	if (!table) {
		return []
	}
	let assignmentText = match[2]
	let whereText: string | undefined
	const whereMatch = maskForClauseScan(assignmentText).match(/\swhere\s/i)
	if (whereMatch && whereMatch.index !== undefined) {
		whereText = assignmentText.slice(whereMatch.index + whereMatch[0].length)
		assignmentText = assignmentText.slice(0, whereMatch.index)
	}
	const predicates = parsePredicates(whereText)
	if (predicates === null) {
		return []
	}
	const assignments: Record<string, unknown> = {}
	for (const entry of splitTopLevel(assignmentText)) {
		const pair = entry.match(/^\s*([a-zA-Z_"][\w."]*)\s*=\s*([\s\S]+?)\s*$/)
		if (pair) {
			assignments[lastIdentifier(pair[1])] = parseValue(pair[2])
		}
	}
	const affected = (table.rows ?? []).filter((row) => rowMatches(row, predicates))
	for (const row of affected) {
		Object.assign(row, assignments)
	}
	return returning ? affected : []
}

function applyDelete(
	datatable: BenchmarkDatatableSeed,
	sql: string
): Record<string, unknown>[] {
	const { body, returning } = splitOffReturning(sql)
	const match = body.match(/^\s*delete\s+from\s+([a-zA-Z_"][\w."]*)\s*([\s\S]*)$/i)
	if (!match) {
		return []
	}
	const table = resolveTable(datatable, match[1])
	if (!table) {
		return []
	}
	const whereText = match[2].replace(/^\s*where\s+/i, '').trim() || undefined
	const predicates = parsePredicates(whereText)
	if (predicates === null) {
		return []
	}
	const rows = table.rows ?? []
	const removed = rows.filter((row) => rowMatches(row, predicates))
	table.rows = rows.filter((row) => !rowMatches(row, predicates))
	return returning ? removed : []
}

// ============= Parsing helpers =============

function resolveTable(
	datatable: BenchmarkDatatableSeed,
	ref: string
): BenchmarkDatatableTableSeed | undefined {
	const { schema, table } = parseRef(ref)
	const direct = datatable.schemas[schema]?.[table]
	if (direct) {
		return direct
	}
	// Bare table name: fall back to searching every schema for a matching table.
	if (!ref.includes('.')) {
		for (const tables of Object.values(datatable.schemas)) {
			if (tables[table]) {
				return tables[table]
			}
		}
	}
	return undefined
}

function firstTable(
	datatable: BenchmarkDatatableSeed
): BenchmarkDatatableTableSeed | undefined {
	for (const tables of Object.values(datatable.schemas)) {
		for (const seed of Object.values(tables)) {
			return seed
		}
	}
	return undefined
}

function parseRef(ref: string): ParsedRef {
	const parts = ref.split('.').map(unquoteIdentifier)
	if (parts.length >= 2) {
		return { schema: parts[parts.length - 2], table: parts[parts.length - 1] }
	}
	return { schema: DEFAULT_SCHEMA, table: parts[0] }
}

/** A WHERE clause with no parseable form returns `null`; absent WHERE returns `[]` (match all). */
function parsePredicates(whereText: string | undefined): Predicate[] | null {
	if (whereText === undefined || whereText.trim() === '') {
		return []
	}
	const predicates: Predicate[] = []
	for (const part of whereText.split(/\s+and\s+/i)) {
		const match = part.match(/^\s*([a-zA-Z_"][\w."]*)\s*=\s*([\s\S]+?)\s*$/)
		if (!match) {
			return null
		}
		predicates.push({ column: lastIdentifier(match[1]), value: parseValue(match[2]) })
	}
	return predicates
}

function rowMatches(row: Record<string, unknown>, predicates: Predicate[]): boolean {
	return predicates.every((predicate) => looseEquals(row[predicate.column], predicate.value))
}

function looseEquals(left: unknown, right: unknown): boolean {
	if (left === null || left === undefined) {
		return right === null || right === undefined
	}
	if (typeof left === 'number' && typeof right === 'number') {
		return left === right
	}
	return String(left) === String(right)
}

function parseValue(raw: string): unknown {
	// Drop a trailing Postgres cast (e.g. `2::int4`) before interpreting the literal.
	const token = raw.trim().replace(/::\s*[a-zA-Z_][\w]*(\([^)]*\))?\s*$/, '').trim()
	const stringMatch = token.match(/^'([\s\S]*)'$/)
	if (stringMatch) {
		return stringMatch[1].replace(/''/g, "'")
	}
	if (/^-?\d+(\.\d+)?$/.test(token)) {
		return Number(token)
	}
	if (/^true$/i.test(token)) {
		return true
	}
	if (/^false$/i.test(token)) {
		return false
	}
	if (/^null$/i.test(token)) {
		return null
	}
	return token
}

function splitOffReturning(sql: string): { body: string; returning: boolean } {
	const match = maskForClauseScan(sql).match(/\sreturning\s/i)
	if (!match || match.index === undefined) {
		return { body: sql, returning: false }
	}
	return { body: sql.slice(0, match.index), returning: true }
}

/**
 * A same-length copy of `sql` with the contents of single-quoted strings and
 * parenthesized groups blanked to spaces, so a top-level keyword scan
 * (WHERE / RETURNING) cannot match inside a string literal or a subquery. Index
 * positions in the result map 1:1 back onto the original.
 */
function maskForClauseScan(sql: string): string {
	let masked = ''
	let depth = 0
	let inString = false
	for (let i = 0; i < sql.length; i++) {
		const char = sql[i]
		if (inString) {
			if (char === "'") {
				if (sql[i + 1] === "'") {
					masked += '  '
					i++
					continue
				}
				inString = false
			}
			masked += ' '
			continue
		}
		if (char === "'") {
			inString = true
			masked += ' '
		} else if (char === '(') {
			depth++
			masked += ' '
		} else if (char === ')') {
			depth = Math.max(0, depth - 1)
			masked += ' '
		} else {
			masked += depth > 0 ? ' ' : char
		}
	}
	return masked
}

/**
 * Inner text of each top-level `( ... )` group in `input`, honoring nested parens
 * (e.g. `now()`, `numeric(10,2)`) and single-quoted strings. Used for the CREATE
 * column-definition group and INSERT value tuples.
 */
function extractParenGroups(input: string): string[] {
	const groups: string[] = []
	let depth = 0
	let inString = false
	let current = ''
	for (let i = 0; i < input.length; i++) {
		const char = input[i]
		if (inString) {
			current += char
			if (char === "'") {
				if (input[i + 1] === "'") {
					current += input[++i]
				} else {
					inString = false
				}
			}
			continue
		}
		if (char === "'") {
			inString = true
			current += char
		} else if (char === '(') {
			depth++
			if (depth === 1) {
				current = ''
			} else {
				current += char
			}
		} else if (char === ')') {
			depth = Math.max(0, depth - 1)
			if (depth === 0) {
				groups.push(current)
				current = ''
			} else {
				current += char
			}
		} else if (depth > 0) {
			current += char
		}
	}
	return groups
}

/** Split on commas that are not inside parentheses or single-quoted strings. */
function splitTopLevel(input: string): string[] {
	const parts: string[] = []
	let depth = 0
	let inString = false
	let current = ''
	for (let i = 0; i < input.length; i++) {
		const char = input[i]
		if (inString) {
			current += char
			if (char === "'") {
				if (input[i + 1] === "'") {
					current += input[++i]
				} else {
					inString = false
				}
			}
			continue
		}
		if (char === "'") {
			inString = true
			current += char
		} else if (char === '(') {
			depth++
			current += char
		} else if (char === ')') {
			depth = Math.max(0, depth - 1)
			current += char
		} else if (char === ',' && depth === 0) {
			parts.push(current)
			current = ''
		} else {
			current += char
		}
	}
	if (current.trim() !== '') {
		parts.push(current)
	}
	return parts
}

function isTableConstraint(def: string): boolean {
	return /^(primary\s+key|foreign\s+key|constraint|unique|check|exclude|like)\b/i.test(def)
}

function unquoteIdentifier(identifier: string): string {
	const trimmed = identifier.trim()
	const quoted = trimmed.match(/^"([\s\S]*)"$/)
	return quoted ? quoted[1] : trimmed
}

/** For a qualified reference like `orders.id`, keep only the final identifier. */
function lastIdentifier(reference: string): string {
	const parts = reference.split('.')
	return unquoteIdentifier(parts[parts.length - 1])
}

function stripTrailingSemicolon(sql: string): string {
	return sql.replace(/;\s*$/, '')
}
