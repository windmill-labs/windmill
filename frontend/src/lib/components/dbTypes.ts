export type DbInput =
	| {
			type: 'database'
			resourceType: DbType
			resourcePath: string
			specificTable?: string
	  }
	| {
			type: 'ducklake'
			ducklake: string
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
