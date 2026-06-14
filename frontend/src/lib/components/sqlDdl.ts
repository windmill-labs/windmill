// DDL (Data Definition Language) statements change the structure of a datatable
// rather than its data. Such schema changes should be recorded as migrations so
// they can be versioned, synced with the CLI and re-applied to forks, instead of
// being run ad-hoc and lost. See GIT-890.
const DDL_OPS = ['CREATE', 'ALTER', 'DROP', 'TRUNCATE', 'RENAME', 'COMMENT', 'GRANT', 'REVOKE']

function getStatementVerb(statement: string): string | undefined {
	return statement
		.trim()
		.match(/^[a-zA-Z]+/)?.[0]
		?.toUpperCase()
}

/** Returns true if any of the given SQL statements is a DDL (schema-changing) statement. */
export function containsDdlStatement(statements: string[]): boolean {
	return statements.some((statement) => {
		const verb = getStatementVerb(statement)
		return verb !== undefined && DDL_OPS.includes(verb)
	})
}
