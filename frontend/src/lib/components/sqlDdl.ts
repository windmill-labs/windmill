// Heuristics for splitting a SQL script into individual statements and
// detecting Data Definition Language (DDL) statements. Shared by the datatable
// SQL REPL and the postgres editor to steer schema changes into migrations.

// code may be composed of many sql statements separated by ';'
// this splits them while taking into account that ';' may appear inside a
// string or quoted identifier and is then not the end of a statement.
export function splitSqlStatements(code: string): string[] {
	const statements: string[] = []
	let currentStatement = ''
	let inSingleQuote = false
	let inDoubleQuote = false
	let inBacktick = false

	for (let i = 0; i < code.length; i++) {
		const char = code[i]
		const prevChar = i > 0 ? code[i - 1] : null

		if (char === "'" && !inDoubleQuote && !inBacktick && prevChar !== '\\') {
			inSingleQuote = !inSingleQuote
		} else if (char === '"' && !inSingleQuote && !inBacktick && prevChar !== '\\') {
			inDoubleQuote = !inDoubleQuote
		} else if (char === '`' && !inSingleQuote && !inDoubleQuote && prevChar !== '\\') {
			inBacktick = !inBacktick
		}

		if (char === ';' && !inSingleQuote && !inDoubleQuote && !inBacktick) {
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
