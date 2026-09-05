/**
 * A migration carries the data table role it runs as in its own SQL, as a
 * `-- role <name>` annotation above everything else. There is no separate field:
 * the annotation is what the postgres executor reads, and keeping it in the SQL
 * is what lets it survive a `wmill sync` round-trip, which only moves .sql files.
 *
 * These mirror `SqlAnnotations::datatable_role` on the backend — annotations are
 * only read from the leading comment block, and only an exact `-- role <name>`
 * line counts, so prose like `-- role based access is handled below` is not one.
 */

const ROLE_NAME_RE = /^[a-zA-Z0-9_-]+$/

/** The role a migration declares, or undefined when it runs as the default. */
export function parseMigrationRole(sql: string): string | undefined {
	for (const raw of sql.split('\n')) {
		const line = raw.trim()
		if (line === '') continue
		if (!line.startsWith('--')) break
		const tokens = line.slice(2).trim().split(/\s+/)
		if (tokens[0] === 'role' && tokens.length === 2 && ROLE_NAME_RE.test(tokens[1])) {
			return tokens[1]
		}
	}
	return undefined
}

/**
 * Return `sql` declaring `role` — replacing any annotation it already carries,
 * or dropping it when `role` is undefined. The annotation has to lead: parsing
 * stops at the first non-comment line, and a migration's SQL usually opens with
 * `BEGIN`.
 */
export function withMigrationRole(sql: string, role: string | undefined): string {
	const lines = sql.split('\n')
	// Only the leading comment block can hold the annotation, so only strip there.
	let i = 0
	const kept: string[] = []
	for (; i < lines.length; i++) {
		const line = lines[i].trim()
		if (line === '') {
			kept.push(lines[i])
			continue
		}
		if (!line.startsWith('--')) break
		const tokens = line.slice(2).trim().split(/\s+/)
		const isRoleLine = tokens[0] === 'role' && tokens.length === 2 && ROLE_NAME_RE.test(tokens[1])
		if (!isRoleLine) kept.push(lines[i])
	}
	const rest = [...kept, ...lines.slice(i)]
	// Drop leading blank lines the removal may have left behind.
	while (rest.length > 0 && rest[0].trim() === '') rest.shift()
	return role ? [`-- role ${role}`, ...rest].join('\n') : rest.join('\n')
}
