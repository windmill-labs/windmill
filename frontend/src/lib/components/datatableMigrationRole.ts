import { isDatatableRoleName } from './dbTypes'

/**
 * A migration carries the data table role it runs as in its own SQL, as a `-- role <name>`
 * annotation. There is no separate field: the annotation is what the server reads, and keeping
 * it in the SQL is what lets it survive a `wmill sync` round-trip.
 *
 * Mirrors `SqlAnnotations::datatable_role` on the backend. It is only read from the leading
 * comment block, so an annotation below `BEGIN;` is ignored and the migration runs as admin. A
 * leading comment whose first word is `role` is an annotation attempt, and a malformed one is an
 * error there, so it is one here too.
 */

export type MigrationRole =
	| { kind: 'none' }
	| { kind: 'role'; role: string }
	| { kind: 'malformed'; line: string }

/** The body of a leading comment line that attempts a role annotation, or undefined. */
function roleAttempt(line: string): string | undefined {
	if (!line.startsWith('--')) return undefined
	const body = line.slice(2).trimStart()
	if (body.slice(0, 4).toLowerCase() !== 'role') return undefined
	const after = body.slice(4)
	if (after !== '' && !/^[\s:=]/.test(after)) return undefined
	return after
}

function parseAttempt(after: string): string | undefined {
	let rest = after.trimStart()
	if (rest.startsWith(':') || rest.startsWith('=')) rest = rest.slice(1)
	const tokens = rest.split(/\s+/).filter((t) => t !== '')
	if (tokens.length !== 1) return undefined
	const role = tokens[0].endsWith(';') ? tokens[0].slice(0, -1) : tokens[0]
	return isDatatableRoleName(role) ? role : undefined
}

export function parseMigrationRole(sql: string): MigrationRole {
	for (const raw of sql.split('\n')) {
		const line = raw.trim()
		if (line === '') continue
		if (!line.startsWith('--')) break
		const after = roleAttempt(line)
		if (after === undefined) continue
		const role = parseAttempt(after)
		return role === undefined ? { kind: 'malformed', line } : { kind: 'role', role }
	}
	return { kind: 'none' }
}

/**
 * `sql` declaring `role`: any role annotation attempt in the leading comment block is removed,
 * and `-- role <role>` is prepended above everything, or nothing when `role` is undefined.
 */
export function withMigrationRole(sql: string, role: string | undefined): string {
	if (role !== undefined && !isDatatableRoleName(role)) {
		throw new Error(`Invalid data table role '${role}'`)
	}
	const lines = sql.split('\n')
	const kept: string[] = []
	let i = 0
	for (; i < lines.length; i++) {
		const line = lines[i].trim()
		if (line !== '' && !line.startsWith('--')) break
		if (roleAttempt(line) === undefined) kept.push(lines[i])
	}
	const rest = [...kept, ...lines.slice(i)]
	while (rest.length > 0 && rest[0].trim() === '') rest.shift()
	return role === undefined ? rest.join('\n') : [`-- role ${role}`, ...rest].join('\n')
}
