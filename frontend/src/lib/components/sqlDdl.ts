// Heuristics for splitting a SQL script into individual statements and
// detecting Data Definition Language (DDL) statements. Shared by the datatable
// SQL REPL and the postgres editor to steer schema changes into migrations.

// A dollar-quote tag: `$$` or `$name$` (the optional tag follows unquoted
// identifier rules, so it starts with a letter/underscore — `$1` is a
// parameter placeholder, not a dollar quote).
const DOLLAR_QUOTE_START = /^\$([A-Za-z_][A-Za-z0-9_]*)?\$/

// code may be composed of many sql statements separated by ';'
// this splits them while taking into account that ';' may appear inside a
// string, quoted identifier, dollar-quoted body ($$ ... $$), or a comment
// (-- ... or /* ... */) and is then not the end of a statement.
type SqlScan = {
	statements: string[]
	/** Source with comment bodies blanked out, quoted runs left intact. */
	withoutComments: string
	/** The final statement has real content but no closing `;`. */
	unterminatedTail: boolean
}

// Single pass over a SQL script, aware of the regions where `;` and comment
// markers are data rather than syntax: strings, quoted identifiers, backticks,
// dollar-quoted bodies and comments. Everything that needs to reason about SQL
// text shares this scan, so an edge case is fixed once.
// Whether everything on `code`'s line before `i` is whitespace.
function isLineStart(code: string, i: number): boolean {
	for (let j = i - 1; j >= 0 && code[j] !== '\n'; j--) {
		if (code[j] !== ' ' && code[j] !== '\t' && code[j] !== '\r') return false
	}
	return true
}

// `backslashEscapes` selects the string-literal dialect. MySQL (and the other
// databases the datatable REPL talks to that keep it on) escape a quote inside an
// ordinary `'…'` with a backslash, so it defaults to true to preserve that. Standard
// SQL (Postgres/DuckDB) instead escapes by doubling and treats `\` as data — pass
// false there — with backslash escapes still honored inside a Postgres `E'…'` literal.
function scanSql(code: string, slashSlashComments = false, backslashEscapes = true): SqlScan {
	const statements: string[] = []
	let currentStatement = ''
	let withoutComments = ''
	let tailHasContent = false
	let inSingleQuote = false
	// The current single-quoted literal honors C-style backslash escapes: either the
	// dialect backslash-escapes every literal, or this is a Postgres `E'…'` literal.
	let inEscapeString = false
	let inDoubleQuote = false
	let inBacktick = false
	let inLineComment = false
	let inBlockComment = false
	let dollarTag: string | null = null

	// Real content: counts toward an unterminated tail and survives comment removal.
	const keep = (text: string) => {
		currentStatement += text
		withoutComments += text
		if (text.trim() !== '') tailHasContent = true
	}
	// Comment content: kept in the statement, dropped from `withoutComments`, and
	// never marks the tail as real content.
	const comment = (text: string) => {
		currentStatement += text
		withoutComments += ' '
	}

	for (let i = 0; i < code.length; i++) {
		const char = code[i]
		const nextChar = i + 1 < code.length ? code[i + 1] : null

		// Inside a protected region: append verbatim (';' is not a separator)
		// and only watch for the region's end.
		if (inLineComment) {
			comment(char)
			if (char === '\n') {
				inLineComment = false
				withoutComments += '\n'
			}
			continue
		}
		if (inBlockComment) {
			// Look ahead for the close so the '*' of the opening '/*' can't be
			// reused (e.g. `/*/` stays open).
			if (char === '*' && nextChar === '/') {
				comment('*/')
				i += 1
				inBlockComment = false
			} else {
				comment(char)
			}
			continue
		}
		if (dollarTag !== null) {
			if (char === '$' && code.startsWith(dollarTag, i)) {
				keep(dollarTag)
				i += dollarTag.length - 1
				dollarTag = null
			} else {
				keep(char)
			}
			continue
		}
		if (inSingleQuote) {
			keep(char)
			// Ordinary literals escape a quote only by doubling it (`''`), so a
			// `\`-ending literal like `'C:\'` is complete and `'a''b'` embeds one quote.
			// Postgres `E'…'` literals ALSO take C-style backslash escapes, so there a
			// `\` escapes the next char (e.g. `\'` stays inside the string). Getting
			// this wrong splits one literal into a spurious extra statement — a
			// DDL-guard false positive and an ATTACH-detection hazard.
			if (inEscapeString && char === '\\' && nextChar != null) {
				keep(nextChar)
				i += 1
			} else if (char === "'") {
				if (nextChar === "'") {
					keep(nextChar)
					i += 1
				} else {
					inSingleQuote = false
				}
			}
			continue
		}
		if (inDoubleQuote) {
			keep(char)
			if (char === '"') {
				if (nextChar === '"') {
					keep(nextChar)
					i += 1
				} else {
					inDoubleQuote = false
				}
			}
			continue
		}
		if (inBacktick) {
			keep(char)
			if (char === '`') {
				if (nextChar === '`') {
					keep(nextChar)
					i += 1
				} else {
					inBacktick = false
				}
			}
			continue
		}

		// Not in any protected region: detect the start of one.
		if (
			(char === '-' && nextChar === '-') ||
			// `//` is not SQL comment syntax (DuckDB uses it for integer division),
			// so treat it as a comment only at a line's start, where Windmill's
			// annotation headers (`// materialize`, `// measure`) live. `5 // 2` and a
			// `ducklake://` scheme are mid-line and stay code.
			(slashSlashComments && char === '/' && nextChar === '/' && isLineStart(code, i))
		) {
			inLineComment = true
			comment(char)
			continue
		}
		if (char === '/' && nextChar === '*') {
			// Consume both chars so the '*' of '/*' can't double as a '*/' close.
			inBlockComment = true
			comment('/*')
			i += 1
			continue
		}
		if (char === '$') {
			const m = DOLLAR_QUOTE_START.exec(code.slice(i))
			if (m) {
				dollarTag = m[0]
				keep(dollarTag)
				i += dollarTag.length - 1
				continue
			}
		}
		if (char === "'") {
			inSingleQuote = true
			// Backslash-escape this literal when the dialect does so for every string,
			// or when it is a Postgres `E'…'`/`e'…'` escape literal.
			inEscapeString = backslashEscapes || (i > 0 && (code[i - 1] === 'E' || code[i - 1] === 'e'))
			keep(char)
			continue
		}
		if (char === '"') {
			inDoubleQuote = true
			keep(char)
			continue
		}
		if (char === '`') {
			inBacktick = true
			keep(char)
			continue
		}

		if (char === ';') {
			statements.push(currentStatement.trim())
			currentStatement = ''
			withoutComments += ';'
			tailHasContent = false
		} else {
			keep(char)
		}
	}

	if (currentStatement.trim()) {
		statements.push(currentStatement.trim())
	}

	return {
		statements: statements.filter((s) => s.length > 0),
		withoutComments,
		unterminatedTail: tailHasContent
	}
}

export function splitSqlStatements(code: string, backslashEscapes = true): string[] {
	return scanSql(code, false, backslashEscapes).statements
}

/**
 * Whether the script's final statement is missing its `;`. Callers appending a
 * new statement need this: a `--` or `;` inside a string or dollar-quoted body
 * must not be mistaken for a comment or a terminator.
 */
export function endsWithUnterminatedStatement(code: string, backslashEscapes = true): boolean {
	return scanSql(code, false, backslashEscapes).unterminatedTail
}

/**
 * Source with comment bodies removed, quoted runs left intact. Set
 * `slashSlashComments` for sources that may carry Windmill's `//` annotation
 * headers, which are not SQL comments. Set `backslashEscapes` to false for
 * standard-SQL (Postgres/DuckDB) sources where `\` is data, not an escape.
 */
export function stripSqlComments(
	code: string,
	slashSlashComments = false,
	backslashEscapes = true
): string {
	return scanSql(code, slashSlashComments, backslashEscapes).withoutComments
}

export function pruneComments(code: string): string {
	return code
		.replace(/--.*?(\r?\n|$)/g, '')
		.replace(/\/\*[\s\S]*?\*\//g, '')
		.trim()
}

// Schema-changing keywords. GRANT/REVOKE are included since permission changes
// also belong in migrations rather than ad-hoc execution.
const DDL_KEYWORDS = ['CREATE', 'ALTER', 'DROP', 'TRUNCATE', 'RENAME', 'COMMENT', 'GRANT', 'REVOKE']

/** Heuristic: a statement is DDL if its first keyword is schema-changing. */
export function isDdlStatement(statement: string): boolean {
	const firstWord = pruneComments(statement).trim().split(/\s+/)[0]?.toUpperCase()
	return !!firstWord && DDL_KEYWORDS.includes(firstWord)
}
