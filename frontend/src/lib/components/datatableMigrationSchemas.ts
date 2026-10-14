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

// The statements a scope is read from: the table, index, schema, sequence and type DDL the
// database manager and the schema diff generate. Anything else (a view, a function, a grant, data,
// an extension) is outside the reader, and makes the scope the whole database.
const STATEMENT_SHAPE =
	/^(?:CREATE\s+(?:UNIQUE\s+)?INDEX|DROP\s+INDEX|(?:CREATE|ALTER|DROP)\s+(?:TABLE|SCHEMA|SEQUENCE|TYPE)|COMMENT\s+ON\s+(?:TABLE|COLUMN|SCHEMA))\b/i
const TRANSACTION_CONTROL = /^(?:BEGIN|COMMIT|END|START\s+TRANSACTION)$/i
// Clauses that reach another object without naming it where the reader looks: a query, a copied
// or inherited definition, a sequence named in a string, and a cascade, which drops whatever
// depends on the target wherever it lives. `ON DELETE CASCADE` is a foreign key's action.
const OPAQUE_CLAUSE =
	/\b(?:SELECT|FROM|LIKE|INHERITS?|OF|ATTACH|DETACH|EXECUTE|nextval)\b|::\s*regclass|(?<!\bON\s+(?:DELETE|UPDATE)\s+)\bCASCADE\b/i
// Words a definition puts before `(` that call nothing a search path resolves: clause keywords,
// built-in types with a modifier, index methods, and `pg_catalog` functions.
const PAREN_WORDS = new Set(
	(
		'key unique check references foreign primary exclude constraint as with where include in any ' +
		'and or not cast ' +
		'all array identity generated enum default values filter over partition by ' +
		'varchar char character varying numeric decimal timestamp timestamptz time timetz interval ' +
		'bit varbit float double btree hash gist gin brin spgist ' +
		'now gen_random_uuid uuidv4 uuidv7 clock_timestamp statement_timestamp transaction_timestamp ' +
		'lower upper length char_length coalesce nullif greatest least abs round trunc md5 random ' +
		'date_trunc to_char to_jsonb jsonb_build_object json_build_object array_length cardinality'
	).split(' ')
)
// Built-in types: an unqualified type outside this list may be a user type resolved through the
// search path.
const BUILTIN_TYPES = new Set(
	(
		'int int2 int4 int8 integer smallint bigint serial serial2 serial4 serial8 smallserial ' +
		'bigserial real float float4 float8 double numeric decimal money text varchar char character ' +
		'bpchar name bytea boolean bool date time timetz timestamp timestamptz interval uuid json ' +
		'jsonb xml inet cidr macaddr macaddr8 bit varbit tsvector tsquery point line lseg box path ' +
		'polygon circle int4range int8range numrange tsrange tstzrange daterange int4multirange ' +
		'int8multirange nummultirange tsmultirange tstzmultirange datemultirange oid'
	).split(' ')
)
const BUILTIN_COLLATIONS = new Set(['c', 'posix', 'default', 'ucs_basic', 'unicode'])
// Keywords that follow `ON`, `REFERENCES` or an object keyword where a name could: `ON DELETE`,
// `COMMENT ON TABLE`, `ON COMMIT`, and the optional `IF [NOT] EXISTS` / `ONLY` a match can land
// on when the name after them is qualified.
const NOT_A_NAME = new Set([
	'if',
	'not',
	'exists',
	'only',
	'concurrently',
	'delete',
	'update',
	'commit',
	'table',
	'column',
	'schema',
	'index',
	'type',
	'sequence',
	'constraint'
])
// Words a column list puts first that open a table constraint rather than a column.
const CONSTRAINT_WORDS = new Set([
	'constraint',
	'primary',
	'foreign',
	'unique',
	'check',
	'exclude',
	'like'
])

/**
 * The scope `sql` declares by what it touches, read conservatively: every statement must be one of
 * the DDL shapes above and name every object it reaches with its schema; otherwise this returns
 * `[]`, the whole database. Callers write it without asking, so it may only ever be wider than the
 * SQL, never narrower: a narrower scope lets a run for one schema apply the migration ahead of a
 * pending one it depends on.
 */
export function suggestMigrationSchemas(sql: string): string[] {
	const code = stripLiteralsAndComments(sql)
	if (OPAQUE_CLAUSE.test(code)) return []
	const statements = code
		.split(';')
		.map((s) => s.trim())
		.filter((s) => s !== '' && !TRANSACTION_CONTROL.test(s))
	if (statements.length === 0) return []
	for (const s of statements) {
		// `DROP TABLE a, b` and `DROP SCHEMA a, b` name several targets; the reader checks one.
		if (/^DROP\b/i.test(s) && splitTopLevel(s).length > 1) return []
		if (!STATEMENT_SHAPE.test(s) || !namesAreQualified(s)) return []
	}
	const found = qualifiedSchemas(code)
	// Every qualifier has to be a schema the scope declares: one that is not (an alias, a schema the
	// reader has no position for) cannot be told apart from a dependency.
	for (const m of code.matchAll(new RegExp(String.raw`(${IDENT})\s*\.\s*${IDENT}`, 'g'))) {
		const qualifier = unquote(m[1]).toLowerCase()
		if (!found.includes(qualifier) && !isSystemSchema(qualifier)) return []
	}
	return found
}

/** Whether a statement of an accepted shape reaches nothing through the search path. */
function namesAreQualified(s: string): boolean {
	const lower = (ident: string) => unquote(ident).toLowerCase()
	const unqualifiedAfter = (prefix: string) =>
		new RegExp(String.raw`${prefix}\s+(${IDENT})(?![A-Za-z0-9_$"]|\s*\.)`, 'gi')
	// The object a statement creates, changes or drops, and the table an index or foreign key
	// targets. `IF [NOT] EXISTS` and `ONLY` come before the name, not instead of it.
	const relation = String.raw`(?:\s+IF\s+(?:NOT\s+)?EXISTS)?(?:\s+ONLY)?`
	for (const re of [
		unqualifiedAfter(
			String.raw`\b(?:CREATE|ALTER|DROP)\s+(?:UNLOGGED\s+)?(?:TABLE|SEQUENCE|TYPE)${relation}`
		),
		unqualifiedAfter(String.raw`\bDROP\s+INDEX(?:\s+CONCURRENTLY)?${relation}`),
		unqualifiedAfter(String.raw`\bON${relation}`),
		unqualifiedAfter(String.raw`\bREFERENCES`),
		unqualifiedAfter(String.raw`\bCOMMENT\s+ON\s+(?:TABLE|COLUMN)`)
	]) {
		for (const m of s.matchAll(re)) {
			if (!NOT_A_NAME.has(lower(m[1]))) return false
		}
	}
	for (const m of s.matchAll(new RegExp(String.raw`::\s*(${IDENT})(?!\s*\.)`, 'g'))) {
		if (!BUILTIN_TYPES.has(lower(m[1]))) return false
	}
	// Calls: a word right before `(` that is not part of a qualified name.
	for (const m of s.matchAll(new RegExp(String.raw`(?<![.\w$"])(${IDENT})\s*\(`, 'g'))) {
		if (!PAREN_WORDS.has(lower(m[1]))) return false
	}
	for (const m of s.matchAll(new RegExp(String.raw`\bCOLLATE\s+(${IDENT})(?!\s*\.)`, 'gi'))) {
		if (!BUILTIN_COLLATIONS.has(lower(m[1]))) return false
	}
	// Column types: in a table's column list, after `ADD [COLUMN]`, and after `TYPE`.
	const types: string[] = []
	const body = /^CREATE\s+(?:UNLOGGED\s+)?TABLE\b[^(]*\(([\s\S]*)\)/i.exec(s)?.[1]
	if (body !== undefined) {
		for (const element of splitTopLevel(body)) {
			const m = new RegExp(String.raw`^(${IDENT})\s+(${IDENT})(?!\s*\.)`).exec(element.trim())
			if (m && !CONSTRAINT_WORDS.has(lower(m[1]))) types.push(m[2])
		}
	}
	for (const m of s.matchAll(
		new RegExp(
			String.raw`\bADD\s+(?:COLUMN\s+)?(?:IF\s+NOT\s+EXISTS\s+)?(${IDENT})\s+(${IDENT})(?!\s*\.)`,
			'gi'
		)
	)) {
		if (!CONSTRAINT_WORDS.has(lower(m[1]))) types.push(m[2])
	}
	for (const m of s.matchAll(new RegExp(String.raw`\bTYPE\s+(${IDENT})(?!\s*\.)`, 'gi'))) {
		types.push(m[1])
	}
	return types.every((t) => BUILTIN_TYPES.has(lower(t)))
}

/** `body` split on the commas outside parentheses. */
function splitTopLevel(body: string): string[] {
	const parts: string[] = []
	let depth = 0
	let start = 0
	for (let i = 0; i < body.length; i++) {
		if (body[i] === '(') depth++
		else if (body[i] === ')') depth--
		else if (body[i] === ',' && depth === 0) {
			parts.push(body.slice(start, i))
			start = i + 1
		}
	}
	parts.push(body.slice(start))
	return parts
}

function isSystemSchema(name: string): boolean {
	return SYSTEM_SCHEMAS.includes(name) || name.startsWith('pg_temp')
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
		if (isAnnotatableSchemaName(name) && !isSystemSchema(name) && !found.includes(name)) {
			found.push(name)
		}
	}
	const patterns = [
		new RegExp(String.raw`\bSCHEMA\s+(?:IF\s+(?:NOT\s+)?EXISTS\s+)?(${IDENT})`, 'gi'),
		// The schema a rename creates, and the one `SET SCHEMA` moves an object into.
		new RegExp(String.raw`\bALTER\s+SCHEMA\s+${IDENT}\s+RENAME\s+TO\s+(${IDENT})`, 'gi'),
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
