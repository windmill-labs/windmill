import type { AclGrant, AclSource, AclTarget } from '$lib/gen'

/** The role a data table connects as without roles — `custom_instance_user` in Postgres. */
export const ADMIN_ROLE = 'admin'

/** Privileges Postgres accepts per kind of object. Mirrors the whitelist the backend validates
 * against — a privilege missing here just cannot be built. */
/** `CREATE` on a database is the right to create schemas in it, and the only database privilege
 * handed out here: `CONNECT` is managed with the instance's role catalog. */
export const DATABASE_PRIVILEGES = ['CREATE']
export const SCHEMA_PRIVILEGES = ['USAGE', 'CREATE']
export const TABLE_PRIVILEGES = [
	'SELECT',
	'INSERT',
	'UPDATE',
	'DELETE',
	'TRUNCATE',
	'REFERENCES',
	'TRIGGER'
]
/** Postgres 17 and later only, so it is offered from what the server reports. */
export const MAINTAIN_PRIVILEGE = 'MAINTAIN'
export const SEQUENCE_PRIVILEGES = ['USAGE', 'SELECT', 'UPDATE']
export const FUNCTION_PRIVILEGES = ['EXECUTE']

export type AclScope =
	| 'target'
	| 'all_tables'
	| 'all_sequences'
	| 'all_functions'
	| 'future_tables'
	| 'future_sequences'
	| 'future_functions'

export type AclTargetKind = AclTarget['kind']

/** The scopes a target can grant on, in the order the builder offers them. */
export function scopesOf(kind: AclTargetKind): { value: AclScope; label: string }[] {
	if (kind === 'database') return [{ value: 'target', label: 'the database itself' }]
	if (kind === 'table') return [{ value: 'target', label: 'this table' }]
	return [
		{ value: 'target', label: 'the schema itself' },
		{ value: 'all_tables', label: 'all tables in it' },
		{ value: 'all_sequences', label: 'all sequences in it' },
		{ value: 'all_functions', label: 'all functions in it' },
		{ value: 'future_tables', label: 'tables created later' },
		{ value: 'future_sequences', label: 'sequences created later' },
		{ value: 'future_functions', label: 'functions created later' }
	]
}

export function privilegesOf(
	scope: AclScope,
	kind: AclTargetKind,
	supportsMaintain = false
): string[] {
	const tablePrivileges = supportsMaintain
		? [...TABLE_PRIVILEGES, MAINTAIN_PRIVILEGE]
		: TABLE_PRIVILEGES
	switch (scope) {
		case 'target':
			if (kind === 'database') return DATABASE_PRIVILEGES
			return kind === 'schema' ? SCHEMA_PRIVILEGES : tablePrivileges
		case 'all_tables':
		case 'future_tables':
			return tablePrivileges
		case 'all_sequences':
		case 'future_sequences':
			return SEQUENCE_PRIVILEGES
		case 'all_functions':
		case 'future_functions':
			return FUNCTION_PRIVILEGES
	}
}

/** One row of the grants table: the same privileges on several objects read as one line, since
 * granting them per object is what `ON ALL TABLES` does. */
export type GroupedGrant = {
	grantee: string
	privileges: string[]
	objects: NonNullable<AclGrant['object']>[]
	future?: string
	/** Every role the row's grants come from, each once, with what it gave. */
	sources: AclSource[]
}

export function groupGrants(grants: AclGrant[]): GroupedGrant[] {
	const rows: GroupedGrant[] = []
	for (const grant of grants) {
		const existing = grant.object
			? rows.find(
					(r) =>
						r.grantee === grant.grantee &&
						r.future === grant.future &&
						r.objects[0]?.kind === grant.object?.kind &&
						r.privileges.join() === grant.privileges.join()
				)
			: undefined
		if (existing) {
			existing.objects.push(grant.object!)
			for (const source of grant.sources) {
				const known = existing.sources.find((s) => s.role === source.role)
				// Whether a role's grant can be taken back depends on the object it is on, so a row
				// holds a source as reachable only if it is on every object the row folds.
				if (known) {
					known.reachable &&= source.reachable
					known.privileges = [...new Set([...known.privileges, ...source.privileges])].sort()
				} else {
					existing.sources.push({ ...source, privileges: [...source.privileges] })
				}
			}
		} else {
			rows.push({
				grantee: grant.grantee,
				privileges: grant.privileges,
				objects: grant.object ? [grant.object] : [],
				future: grant.future,
				sources: grant.sources.map((s) => ({ ...s, privileges: [...s.privileges] }))
			})
		}
	}
	return rows
}

/** The roles that gave some of `privileges` and that this data table's connection cannot act for.
 * Only they can take those grants back, so a revoke of `privileges` is not offered. */
export function blockingSources(grant: GroupedGrant, privileges: string[]): string[] {
	return grant.sources
		.filter((s) => !s.reachable && s.privileges.some((p) => privileges.includes(p)))
		.map((s) => s.role)
}

/** Which of `roles` a "created later" row granted for some of them does not cover. A default
 * privilege binds only the creating roles it was granted for, so what the others create stays out
 * of it. A row none of `roles` set — the instance's own, say — was never meant to cover them, and
 * names none. */
export function uncoveredCreators(grant: GroupedGrant, roles: string[]): string[] {
	if (!grant.future || !grant.sources.some((s) => roles.includes(s.role))) return []
	return roles.filter((r) => !grant.sources.some((s) => s.role === r))
}

/** A row's identity. Two rows may share a grantee and an object name — a table `orders` and a
 * function `orders()` — so the kind and the privileges are part of it too. */
export function grantKey(grant: GroupedGrant): string {
	return [
		grant.grantee,
		grant.future ?? '',
		grant.privileges.join(','),
		...grant.objects.map((o) => `${o.kind}:${o.name}(${o.args ?? ''})`)
	].join('|')
}

/** The scope a revoke of this row takes, or `undefined` when the builder cannot express it —
 * Postgres also records privileges on types, present and default, which nothing here grants and
 * the API has no scope for. */
export function revokeScopeOf(grant: GroupedGrant): AclScope | undefined {
	if (!grant.future) return grant.objects.some((o) => o.kind === 'TYPE') ? undefined : 'target'
	const scope = `future_${grant.future.toLowerCase()}`
	return (['future_tables', 'future_sequences', 'future_functions'] as const).find(
		(s) => s === scope
	)
}

/** The privileges of a row a revoke may take back. On the database that is `CREATE` alone:
 * `CONNECT` belongs to the role catalog, which would grant it again, and `TEMPORARY` is not one
 * the editor hands out — a row holding only those has nothing to revoke here. A database's rows
 * "created later" are default privileges set database-wide, which nothing here revokes. */
export function revocablePrivileges(grant: GroupedGrant, target: AclTarget): string[] {
	if (target.kind === 'database' && grant.objects.length === 0) {
		if (grant.future) return []
		return grant.privileges.filter((p) => DATABASE_PRIVILEGES.includes(p))
	}
	return grant.privileges
}

/** How many objects a folded row names before it summarizes the rest. */
const LISTED_OBJECTS = 3

/** What a row covers, when that is not the editor's own object: the objects inside it, or what is
 * created later. Undefined for a grant on the object itself, which needs no saying. */
export function grantCoverage(grant: GroupedGrant, target: AclTarget): string | undefined {
	if (grant.future) {
		const created = `${grant.future.toLowerCase()} created later`
		return target.kind === 'database' ? `${created}, in every schema` : created
	}
	if (grant.objects.length === 0) return undefined
	// A routine's arguments are part of what it is: two of the same name are different objects.
	const names = grant.objects.map((o) => (o.args !== undefined ? `${o.name}(${o.args})` : o.name))
	const listed = names.slice(0, LISTED_OBJECTS).join(', ')
	const rest = names.length - LISTED_OBJECTS
	const kind = grant.objects[0].kind.toLowerCase()
	return `${names.length > 1 ? `${kind}s` : kind} ${listed}${rest > 0 ? ` and ${rest} more` : ''}`
}
