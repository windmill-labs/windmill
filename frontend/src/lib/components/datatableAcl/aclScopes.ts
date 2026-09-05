import type { AclGrant, AclTarget } from '$lib/gen'

/** Privileges Postgres accepts per kind of object. Mirrors the whitelist the
 * backend validates against — a privilege missing here just cannot be built. */
/** `CREATE` on a database is the right to create schemas in it. `CONNECT` is
 * not offered: every role a data table creates is granted it at creation, so
 * building that statement by hand would only ever restate what is already true.
 * The backend still accepts it, so a CONNECT grant read off the catalog can be
 * revoked. */
export const DATABASE_PRIVILEGES = ['CREATE', 'TEMPORARY']
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

/** What a statement built at this scope reads as, for the builder's own preview. */
export function scopeSql(scope: AclScope, target: AclTarget, dbname?: string): string {
	if (target.kind === 'database') return `DATABASE ${dbname ?? ''}`.trim()
	const schema = target.schema
	switch (scope) {
		case 'target':
			return target.kind === 'schema' ? `SCHEMA ${schema}` : `TABLE ${schema}.${target.table}`
		case 'all_tables':
			return `ALL TABLES IN SCHEMA ${schema}`
		case 'all_sequences':
			return `ALL SEQUENCES IN SCHEMA ${schema}`
		case 'all_functions':
			return `ALL FUNCTIONS IN SCHEMA ${schema}`
		case 'future_tables':
			return `TABLES (default privileges in ${schema})`
		case 'future_sequences':
			return `SEQUENCES (default privileges in ${schema})`
		case 'future_functions':
			return `FUNCTIONS (default privileges in ${schema})`
	}
}

/** One row of the grants table: the same privileges on several objects read as
 * one line, since granting them per object is what `ON ALL TABLES` does. */
export type GroupedGrant = {
	grantee: string
	privileges: string[]
	objects: NonNullable<AclGrant['object']>[]
	future?: string
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
		} else {
			rows.push({
				grantee: grant.grantee,
				privileges: grant.privileges,
				objects: grant.object ? [grant.object] : [],
				future: grant.future
			})
		}
	}
	return rows
}

/** The scope a revoke of this row takes, or `undefined` when the builder cannot
 * express it — Postgres also records default privileges on types, which nothing
 * here grants and the API has no scope for. */
export function revokeScopeOf(grant: GroupedGrant): AclScope | undefined {
	if (!grant.future) return 'target'
	const scope = `future_${grant.future.toLowerCase()}`
	return (['future_tables', 'future_sequences', 'future_functions'] as const).find(
		(s) => s === scope
	)
}

/** How a row reads back: what it covers, in one phrase. */
export function grantScopeLabel(grant: GroupedGrant): string {
	if (grant.future) return `${grant.future.toLowerCase()} created later`
	if (grant.objects.length === 1) {
		const object = grant.objects[0]
		// A routine's arguments are part of what it is, so two of the same name
		// would otherwise read as one row twice.
		const args = object.args !== undefined ? `(${object.args})` : ''
		return `${object.kind.toLowerCase()} ${object.name}${args}`
	}
	if (grant.objects.length > 1)
		return `${grant.objects.length} ${grant.objects[0].kind.toLowerCase()}s`
	return 'itself'
}
