import type { DataMetric } from '$lib/gen'
import { splitSqlStatements, stripSqlComments } from '../sqlDdl'

function quoteIdent(name: string): string {
	return `"${name.replaceAll('"', '""')}"`
}

/**
 * All generated identifiers are quoted. Quoting conditionally would mean tracking
 * DuckDB's reserved-word set, which shifts between versions: a name like `select`,
 * `order` or `lambda` is rejected bare, and a list that misses one silently emits
 * invalid SQL. Quoting always is version-proof and needs no maintenance.
 */
function ident(name: string): string {
	return quoteIdent(name)
}

/**
 * Alias used when the snippet attaches the lake itself. `dl` is the convention
 * across the codebase's DuckLake scripts; the lake's own name is only a fallback
 * for when the script already binds `dl` to a different lake.
 */
export const DEFAULT_LAKE_ALIAS = 'dl'

function escapeRegex(s: string): string {
	return s.replaceAll(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

type TableParts = { lake: string; schema: string; table: string }

/** Splits `<lake>/<table>` or `<lake>/<schema>.<table>`, defaulting the schema. */
export function splitTablePath(tablePath: string): TableParts | undefined {
	const path = tablePath.startsWith('ducklake://')
		? tablePath.slice('ducklake://'.length)
		: tablePath
	const slash = path.indexOf('/')
	if (slash < 0) return undefined
	const lake = path.slice(0, slash)
	const rest = path.slice(slash + 1)
	const dot = rest.indexOf('.')
	const schema = dot < 0 ? 'main' : rest.slice(0, dot)
	const table = dot < 0 ? rest : rest.slice(dot + 1)
	if (!lake || !table) return undefined
	return { lake, schema, table }
}

/**
 * The alias `code` attaches `lake` under, if it attaches it at all.
 *
 * DuckDB names the catalog after the ATTACH alias, not after the lake, and the
 * prevailing convention is an arbitrary short alias (`AS dl`). Qualifying a table
 * with the lake name would therefore fail against most real scripts.
 *
 * `ATTACH 'ducklake'` with no `://` part refers to the lake named `main`, so that
 * shorthand has to resolve here too or a `main` table gets a redundant second
 * attachment under a different alias.
 */
// SQL statements, comments stripped, split on real (non-literal) `;`. Matching
// ATTACH per statement (anchored at its start) ignores attachment-like text
// buried in a string or dollar-quoted literal, e.g. `SELECT $$ATTACH … AS x$$`.
// `false` selects standard-SQL string quoting: this is DuckDB, where `\` is data
// (so a `'C:\'` literal does not swallow the following ATTACH) and quotes double.
function statements(code: string): string[] {
	return splitSqlStatements(stripSqlComments(code, true, false), false).map((s) => s.trim())
}

export function attachAliasFor(code: string, lake: string): string | undefined {
	// SQL keywords match case-insensitively; the lake name must not. DuckLake
	// config keys are case-sensitive, so `sales` and `Sales` are different lakes,
	// and folding them would reuse an alias bound to the wrong one. So the URI is
	// captured and compared exactly. A `ducklake` with no `://` refers to `main`.
	// A quoted alias may contain doubled quotes (`"a""b"` = the identifier `a"b`);
	// capture the whole quoted run and unescape, and require an unquoted alias to
	// end on a token boundary.
	const re =
		/^ATTACH\s+(?:DATABASE\s+)?(?:IF\s+NOT\s+EXISTS\s+)?'ducklake(:\/\/[^']*)?'\s+AS\s+(?:"((?:[^"]|"")*)"|([A-Za-z_][\w$]*))(?![\w$])/i
	for (const stmt of statements(code)) {
		const m = re.exec(stmt)
		if (m) {
			const attachedLake = m[1] ? m[1].slice('://'.length) : 'main'
			if (attachedLake === lake) {
				return m[2] !== undefined ? m[2].replaceAll('""', '"') : m[3]
			}
		}
	}
	return undefined
}

/**
 * Whether `code` already attaches anything under `alias`. Deliberately not
 * limited to DuckLake: a SQLite or Postgres attachment occupies the alias just
 * as effectively, and re-using it would fail on the duplicate name.
 */
export function attachAliasTaken(code: string, alias: string): boolean {
	const re = new RegExp(
		`^ATTACH\\s+(?:DATABASE\\s+)?(?:IF\\s+NOT\\s+EXISTS\\s+)?'[^']*'\\s+AS\\s+"?${escapeRegex(alias)}"?(?![\\w$])`,
		'i'
	)
	return statements(code).some((stmt) => re.test(stmt))
}

/**
 * First alias not already attached in `code`: the conventional `dl`, then the
 * lake's own name, then numbered variants. Checking each matters because falling
 * back blindly can land on a name that is itself taken.
 */
export function pickAttachAlias(code: string, lake: string): string {
	if (!attachAliasTaken(code, DEFAULT_LAKE_ALIAS)) return DEFAULT_LAKE_ALIAS
	if (!attachAliasTaken(code, lake)) return lake
	// Unbounded on purpose, and it terminates: a finite script can only occupy
	// finitely many aliases, so some `dlN` is always free. Returning a taken one
	// instead would emit an ATTACH that fails on the duplicate name.
	let n = 2
	while (attachAliasTaken(code, `${DEFAULT_LAKE_ALIAS}${n}`)) n += 1
	return `${DEFAULT_LAKE_ALIAS}${n}`
}

/**
 * Whether `code` looks like it references this table. Used only to preselect a
 * table in the picker, so a false positive costs nothing.
 */
export function codeMentionsTable(code: string, tablePath: string): boolean {
	const parts = splitTablePath(tablePath)
	if (!parts) return false
	return new RegExp(`\\b${escapeRegex(parts.table)}\\b`, 'i').test(code)
}

/**
 * A measure renders as its declared aggregate plus, when it carries a predicate,
 * a trailing `FILTER (WHERE …)`. FILTER rather than a shared `WHERE` is what lets
 * measures with different predicates sit under one GROUP BY.
 */
function measureExpr(m: DataMetric): string {
	return m.filter ? `${m.expr} FILTER (WHERE ${m.filter})` : m.expr
}

/**
 * Composes a plain SELECT from a metric selection. The result is ordinary
 * editable SQL: nothing re-reads or rewrites it later, so a user is free to
 * change it after inserting.
 */
export function composeMetricSql(opts: {
	tablePath: string
	measures: DataMetric[]
	dimensions: DataMetric[]
	/**
	 * The alias the script already attaches this lake under. When absent the
	 * snippet attaches the lake itself under `attachAs`.
	 */
	existingAlias?: string
	/** Alias to attach under when the script has none. Defaults to `dl`. */
	attachAs?: string
}): string {
	const { tablePath, measures, dimensions, existingAlias, attachAs } = opts
	const parts = splitTablePath(tablePath)
	if (!parts) return ''
	// Qualify with the alias in force, so the snippet resolves against the
	// catalog the script actually has attached.
	const catalog = existingAlias ?? attachAs ?? DEFAULT_LAKE_ALIAS
	const tableRef = `${ident(catalog)}.${ident(parts.schema)}.${ident(parts.table)}`

	const selected = [
		...dimensions.map((d) => `  ${d.expr} AS ${ident(d.name)}`),
		...measures.map((m) => `  ${measureExpr(m)} AS ${ident(m.name)}`)
	]

	const lines: string[] = []
	if (!existingAlias) {
		// Escape the lake as a SQL string literal. The deploy path already rejects
		// unsafe names, but the composed query is copied and run standalone, so it
		// must not trust the catalog value blindly.
		const lakeLiteral = parts.lake.replaceAll("'", "''")
		lines.push(`ATTACH 'ducklake://${lakeLiteral}' AS ${ident(catalog)};`, '')
	}
	lines.push('SELECT', selected.join(',\n'), `FROM ${tableRef}`)
	if (dimensions.length > 0) {
		// Group by ordinal so a dimension expression is written once.
		const ordinals = dimensions.map((_, i) => i + 1).join(', ')
		lines.push(`GROUP BY ${ordinals}`, `ORDER BY ${ordinals}`)
	}
	return lines.join('\n')
}
