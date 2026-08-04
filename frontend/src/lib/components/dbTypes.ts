export type DbInput =
	| {
			type: 'database'
			resourceType: DbType
			resourcePath: string
			/** Data table role to connect as. Only meaningful for a `datatable://`
			 * resourcePath; absent means the data table's default role. */
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
