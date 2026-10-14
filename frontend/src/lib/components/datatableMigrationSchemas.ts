/**
 * A migration declares the schemas it touches in its own SQL, as a `-- schemas sales, billing`
 * annotation, the same way it declares its role (see `datatableMigrationRole.ts`). A migration that
 * declares none applies to the whole database: every later migration waits for it, so the editors
 * suggest a scope from the SQL rather than leaving it out.
 *
 * Mirrors `parse_migration_schemas` in `datatable_migration_scope.rs`: read from the leading comment
 * block only, names lowercased, and a leading comment whose first word is `schemas` is an
 * annotation attempt, so a malformed one is an error.
 */

export type MigrationSchemas =
	| { kind: 'none' }
	| { kind: 'schemas'; schemas: string[] }
	| { kind: 'malformed'; line: string }

const KEYWORD = 'schemas'

export function isAnnotatableSchemaName(name: string): boolean {
	return /^[A-Za-z0-9_$-]{1,63}$/.test(name)
}

/** The rest of a leading comment line that attempts a scope annotation, or undefined. */
function schemasAttempt(line: string): string | undefined {
	if (!line.startsWith('--')) return undefined
	const body = line.slice(2).trimStart()
	if (body.slice(0, KEYWORD.length).toLowerCase() !== KEYWORD) return undefined
	const after = body.slice(KEYWORD.length)
	if (after !== '' && !/^[\s:=]/.test(after)) return undefined
	return after
}

function parseAttempt(after: string): string[] | undefined {
	let rest = after.trimStart()
	if (rest.startsWith(':') || rest.startsWith('=')) rest = rest.slice(1)
	rest = rest.trim()
	if (rest.endsWith(';')) rest = rest.slice(0, -1)
	const schemas: string[] = []
	for (const piece of rest.split(',')) {
		const name = piece.trim()
		if (!isAnnotatableSchemaName(name)) return undefined
		const lower = name.toLowerCase()
		if (!schemas.includes(lower)) schemas.push(lower)
	}
	return schemas
}

export function parseMigrationSchemas(sql: string): MigrationSchemas {
	for (const raw of sql.split('\n')) {
		const line = raw.trim()
		if (line === '') continue
		if (!line.startsWith('--')) break
		const after = schemasAttempt(line)
		if (after === undefined) continue
		const schemas = parseAttempt(after)
		return schemas === undefined ? { kind: 'malformed', line } : { kind: 'schemas', schemas }
	}
	return { kind: 'none' }
}

/**
 * `sql` declaring `schemas`: any scope annotation attempt in the leading comment block is removed,
 * and `-- schemas <names>` is prepended above everything, or nothing when `schemas` is empty.
 */
export function withMigrationSchemas(sql: string, schemas: string[]): string {
	for (const s of schemas) {
		if (!isAnnotatableSchemaName(s)) throw new Error(`Invalid schema name '${s}'`)
	}
	const lines = sql.split('\n')
	const kept: string[] = []
	let i = 0
	for (; i < lines.length; i++) {
		const line = lines[i].trim()
		if (line !== '' && !line.startsWith('--')) break
		if (schemasAttempt(line) === undefined) kept.push(lines[i])
	}
	const rest = [...kept, ...lines.slice(i)]
	while (rest.length > 0 && rest[0].trim() === '') rest.shift()
	return schemas.length === 0
		? rest.join('\n')
		: [`-- ${KEYWORD} ${schemas.join(', ')}`, ...rest].join('\n')
}

/** Whether two scopes overlap; `undefined` is the whole database, which overlaps everything. */
export function scopesOverlap(a: string[] | undefined, b: string[] | undefined): boolean {
	if (a === undefined || b === undefined) return true
	return a.some((s) => b.includes(s))
}

/** The scope a migration's SQL declares, `undefined` for the whole database. */
export function migrationScope(sql: string): string[] | undefined {
	const parsed = parseMigrationSchemas(sql)
	return parsed.kind === 'schemas' ? parsed.schemas : undefined
}

const IDENT = String.raw`(?:"(?:[^"]|"")+"|[A-Za-z_][A-Za-z0-9_$]*)`
const SYSTEM_SCHEMAS = ['pg_catalog', 'information_schema', 'pg_toast', 'pg_temp']

function unquote(ident: string): string {
	return ident.startsWith('"') ? ident.slice(1, -1).replaceAll('""', '"') : ident
}

// Words that can follow an object keyword without being the object's name.
// `ON UPDATE CASCADE`, `FOR UPDATE SKIP LOCKED` and `JOIN LATERAL` among them.
const NOT_A_NAME = [
	'if',
	'only',
	'not',
	'concurrently',
	'exists',
	'cascade',
	'restrict',
	'set',
	'no',
	'skip',
	'nowait',
	'of',
	'lateral'
]

/**
 * The schemas `sql` names explicitly, as a suggested scope: those it creates, alters or grants on,
 * and the schema of every qualified object it defines or writes. An unqualified name resolves
 * through the search path, which the SQL does not show, so one anywhere makes the suggestion empty:
 * declaring only the qualified schemas would let the migration run ahead of one it depends on.
 * Callers write the suggestion without asking, so it must never be narrower than the SQL.
 */
export function suggestMigrationSchemas(sql: string): string[] {
	const code = stripLiteralsAndComments(sql)
	const unqualified = new RegExp(
		String.raw`\b(?:(?:CREATE|ALTER|DROP)\s+(?:OR\s+REPLACE\s+)?(?:TEMP(?:ORARY)?\s+|UNLOGGED\s+|MATERIALIZED\s+)?(?:TABLE|VIEW|SEQUENCE|TYPE|FUNCTION|PROCEDURE|DOMAIN)|REFERENCES|INSERT\s+INTO|UPDATE|DELETE\s+FROM|JOIN|INDEX\s+(?:CONCURRENTLY\s+)?(?:IF\s+NOT\s+EXISTS\s+)?(?:${IDENT}\s+)?ON)\s+(?:IF\s+(?:NOT\s+)?EXISTS\s+)?(?:ONLY\s+)?(${IDENT})(?![A-Za-z0-9_$"]|\s*\.)`,
		'gi'
	)
	for (const m of code.matchAll(unqualified)) {
		if (!NOT_A_NAME.includes(unquote(m[1]).toLowerCase())) return []
	}
	return qualifiedSchemas(code)
}

function stripLiteralsAndComments(sql: string): string {
	return sql
		.replace(/\$([A-Za-z_]*)\$[\s\S]*?\$\1\$/g, ' ')
		.replace(/'(?:[^']|'')*'/g, ' ')
		.replace(/\/\*[\s\S]*?\*\//g, ' ')
		.replace(/--[^\n]*/g, ' ')
}

function qualifiedSchemas(code: string): string[] {
	const found: string[] = []
	const add = (ident: string) => {
		const name = unquote(ident).toLowerCase()
		if (
			isAnnotatableSchemaName(name) &&
			!SYSTEM_SCHEMAS.includes(name) &&
			!name.startsWith('pg_temp') &&
			!found.includes(name)
		) {
			found.push(name)
		}
	}
	const patterns = [
		new RegExp(String.raw`\bSCHEMA\s+(?:IF\s+(?:NOT\s+)?EXISTS\s+)?(${IDENT})`, 'gi'),
		new RegExp(
			String.raw`\b(?:TABLE|VIEW|SEQUENCE|TYPE|FUNCTION|PROCEDURE|DOMAIN|INDEX|TRIGGER|COLUMN|REFERENCES|INTO|UPDATE|FROM|JOIN|EXISTS|ONLY)\s+(${IDENT})\s*\.\s*${IDENT}`,
			'gi'
		),
		// `ON` also opens a join condition (`ON a.id = b.id`), so only the forms that name a relation
		// count: an index, trigger, policy, grant or comment target.
		new RegExp(
			String.raw`\bON\s+(?:ONLY\s+)?(${IDENT})\s*\.\s*${IDENT}(?=\s*(?:\(|USING\b|FOR\b|TO\b|AS\b|IS\b|;|$))`,
			'gim'
		)
	]
	for (const re of patterns) {
		for (const m of code.matchAll(re)) add(m[1])
	}
	return found
}
