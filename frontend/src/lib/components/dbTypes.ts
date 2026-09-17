export type DbInput =
	| {
			type: 'database'
			resourceType: DbType
			resourcePath: string
			/** The data table role to connect as; the data table's default when unset. Only
			 * meaningful for a `datatable://` path. */
			role?: string
			/** The role migrations written through this input declare when `role` is unset. A
			 * migration declaring none runs as admin, not as the role the manager connects as. */
			migrationRole?: string
			specificSchema?: string
			specificTable?: string
	  }
	| {
			type: 'ducklake'
			ducklake: string
			specificSchema?: string
			specificTable?: string
	  }

export type DbType = (typeof dbTypes)[number]
export const dbTypes = [
	'mysql',
	'ms_sql_server',
	'postgresql',
	'snowflake',
	'bigquery',
	'duckdb'
] as const
export const isDbType = (str?: string): str is DbType => !!str && dbTypes.includes(str as DbType)

/** The role every data table has: the one it connects as when it is not under roles. */
export const ADMIN_DATATABLE_ROLE = 'admin'

/** What the server accepts in `-- role <name>` and `?role=<name>`. */
export function isDatatableRoleName(name: string): boolean {
	return /^[A-Za-z0-9_-]{1,63}$/.test(name)
}

/** Whether a role can be named in a reference to this data table. A name stored before names were
 * restricted may contain `?`, and the server reads such a whole reference as that name first, so
 * `?role=` after it would be taken as part of the name or refused. */
export function datatableNameTakesRole(name: string): boolean {
	return !name.includes('?')
}

/** The `migrationRole` of a data table that cannot name a role in its reference: it connects as
 * its default role, which its migrations must then declare. */
export function defaultMigrationRole(
	name: string,
	permissioned: boolean | undefined,
	defaultRole: string | undefined
): string | undefined {
	return permissioned && !datatableNameTakesRole(name) ? defaultRole : undefined
}

/** `datatable://<name>`, with `?role=<role>` when a role is named. Throws rather than build a
 * reference the executor would refuse, or one that would silently mean another role. */
export function datatableReference(name: string, role: string | undefined): string {
	if (role === undefined) return `datatable://${name}`
	if (!isDatatableRoleName(role)) {
		throw new Error(
			`Invalid data table role '${role}': only letters, digits, '_' and '-' are allowed`
		)
	}
	if (!datatableNameTakesRole(name)) {
		throw new Error(
			`Data table '${name}' has a '?' in its name, so it can only be used here as its default role. Rename it to connect as role '${role}'.`
		)
	}
	return `datatable://${name}?role=${role}`
}

export type DatatableRowAction = 'migrations' | 'roles' | 'export' | 'import'
