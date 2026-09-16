export type DbInput =
	| {
			type: 'database'
			resourceType: DbType
			resourcePath: string
			/** The data table role to connect as; the data table's default when unset. Only
			 * meaningful for a `datatable://` path. */
			role?: string
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

/** `datatable://<name>`, with `?role=<role>` when a role is named. Throws rather than build a
 * reference the executor would refuse, or one that would silently mean another role. */
export function datatableReference(name: string, role: string | undefined): string {
	if (role === undefined) return `datatable://${name}`
	if (!isDatatableRoleName(role)) {
		throw new Error(
			`Invalid data table role '${role}': only letters, digits, '_' and '-' are allowed`
		)
	}
	return `datatable://${name}?role=${role}`
}

export type DatatableRowAction = 'migrations' | 'roles' | 'export' | 'import'
