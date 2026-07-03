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
export function splitSqlStatements(code: string): string[] {
	const statements: string[] = []
	let currentStatement = ''
	let inSingleQuote = false
	let inDoubleQuote = false
	let inBacktick = false
	let inLineComment = false
	let inBlockComment = false
	let dollarTag: string | null = null

	for (let i = 0; i < code.length; i++) {
		const char = code[i]
		const prevChar = i > 0 ? code[i - 1] : null
		const nextChar = i + 1 < code.length ? code[i + 1] : null

		// Inside a protected region: append verbatim (';' is not a separator)
		// and only watch for the region's end.
		if (inLineComment) {
			currentStatement += char
			if (char === '\n') inLineComment = false
			continue
		}
		if (inBlockComment) {
			// Look ahead for the close so the '*' of the opening '/*' can't be
			// reused (e.g. `/*/` stays open).
			if (char === '*' && nextChar === '/') {
				currentStatement += '*/'
				i += 1
				inBlockComment = false
			} else {
				currentStatement += char
			}
			continue
		}
		if (dollarTag !== null) {
			if (char === '$' && code.startsWith(dollarTag, i)) {
				currentStatement += dollarTag
				i += dollarTag.length - 1
				dollarTag = null
			} else {
				currentStatement += char
			}
			continue
		}
		if (inSingleQuote) {
			currentStatement += char
			if (char === "'" && prevChar !== '\\') inSingleQuote = false
			continue
		}
		if (inDoubleQuote) {
			currentStatement += char
			if (char === '"' && prevChar !== '\\') inDoubleQuote = false
			continue
		}
		if (inBacktick) {
			currentStatement += char
			if (char === '`' && prevChar !== '\\') inBacktick = false
			continue
		}

		// Not in any protected region: detect the start of one.
		if (char === '-' && nextChar === '-') {
			inLineComment = true
			currentStatement += char
			continue
		}
		if (char === '/' && nextChar === '*') {
			// Consume both chars so the '*' of '/*' can't double as a '*/' close.
			inBlockComment = true
			currentStatement += '/*'
			i += 1
			continue
		}
		if (char === '$') {
			const m = DOLLAR_QUOTE_START.exec(code.slice(i))
			if (m) {
				dollarTag = m[0]
				currentStatement += dollarTag
				i += dollarTag.length - 1
				continue
			}
		}
		if (char === "'") {
			inSingleQuote = true
			currentStatement += char
			continue
		}
		if (char === '"') {
			inDoubleQuote = true
			currentStatement += char
			continue
		}
		if (char === '`') {
			inBacktick = true
			currentStatement += char
			continue
		}

		if (char === ';') {
			statements.push(currentStatement.trim())
			currentStatement = ''
		} else {
			currentStatement += char
		}
	}

	if (currentStatement.trim()) {
		statements.push(currentStatement.trim())
	}

	return statements.filter((s) => s.length > 0)
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
