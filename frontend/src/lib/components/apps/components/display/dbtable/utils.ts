import type { ScriptLang } from '$lib/gen'
import type { SQLSchema } from '$lib/stores'
import type { IntrospectionQuery } from 'graphql'

import type { DbType } from '$lib/components/dbTypes'

export enum ColumnIdentity {
	ByDefault = 'By Default',
	Always = 'Always',
	No = 'No'
}

export type ColumnMetadata = {
	field: string
	datatype: string
	defaultvalue: string
	isprimarykey: boolean
	isidentity: ColumnIdentity
	isnullable: 'YES' | 'NO'
	isenum: boolean
	default_constraint_name?: string // MS SQL requires to know this to drop default
}
export type TableMetadata = ColumnMetadata[]

export type ColumnDef = {
	minWidth?: number
	hide?: boolean
	flex?: number
	sort?: 'asc' | 'desc'
	sortIndex?: number
	aggFunc?: string
	pivot?: boolean
	pivotIndex?: number
	pinned?: 'left' | 'right' | boolean
	rowGroup?: boolean
	rowGroupIndex?: number
	valueFormatter?: string
	valueParser?: string
	field?: string
	headerName?: string
	children?: ColumnDef[]
	// DBExplorer
	ignored?: boolean
	hideInsert?: boolean
	editable?: boolean
	overrideDefaultValue?: boolean
	defaultUserValue?: any
	defaultValueNull?: boolean
} & ColumnMetadata

export function resourceTypeToLang(rt: string) {
	if (rt === 'ms_sql_server') {
		return 'mssql'
	} else {
		return rt
	}
}

const legacyScripts: Record<
	string,
	{
		code: string | (() => Promise<string>)
		lang: string
		processingFn?: (any: any) => SQLSchema['schema']
		argName: string
	}
> = {
	postgresql: {
		code: `SELECT table_name, column_name, udt_name, column_default, is_nullable, table_schema FROM information_schema.columns WHERE table_schema != 'pg_catalog' AND table_schema != 'information_schema'`,
		processingFn: (rows) => {
			const schemas = rows.reduce((acc, a) => {
				const table_schema = a.table_schema
				delete a.table_schema
				acc[table_schema] = acc[table_schema] || []
				if (a.table_name || a.column_name) acc[table_schema].push(a)
				return acc
			}, {})

			const data = {}
			for (const key in schemas) {
				data[key] = schemas[key].reduce((acc, a) => {
					const table_name = a.table_name
					delete a.table_name
					acc[table_name] = acc[table_name] || {}
					const p: {
						type: string
						required: boolean
						default?: string
					} = {
						type: a.udt_name,
						required: a.is_nullable === 'NO'
					}
					if (a.column_default) {
						p.default = a.column_default
					}
					acc[table_name][a.column_name] = p
					return acc
				}, {})
			}

			return data
		},
		lang: 'postgresql',
		argName: 'database'
	},
	mysql: {
		code: "SELECT DATABASE() AS default_db_name, TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT FROM information_schema.columns WHERE table_schema = DATABASE() OR table_schema NOT IN ('information_schema', 'mysql', 'performance_schema', 'sys', '_vt');",
		processingFn: (rows) => {
			const schemas = rows.reduce((acc, a) => {
				const table_schema = a.TABLE_SCHEMA
				delete a.TABLE_SCHEMA
				acc[table_schema] = acc[table_schema] || []
				acc[table_schema].push(a)
				return acc
			}, {})

			const data = {}
			for (const key in schemas) {
				data[key] = schemas[key].reduce((acc, a) => {
					const table_name = a.TABLE_NAME
					delete a.TABLE_NAME
					acc[table_name] = acc[table_name] || {}
					const p: {
						type: string
						required: boolean
						default?: string
					} = {
						type: a.DATA_TYPE,
						required: a.is_nullable === 'NO'
					}
					if (a.column_default) {
						p.default = a.COLUMN_DEFAULT
					}
					acc[table_name][a.COLUMN_NAME] = p
					return acc
				}, {})
			}
			return data
		},
		lang: 'mysql',
		argName: 'database'
	},
	graphql: {
		code: () => import('graphql').then((m) => m.getIntrospectionQuery()),
		lang: 'graphql',
		argName: 'api'
	},
	bigquery: {
		code: `import { BigQuery } from '@google-cloud/bigquery@7.5.0';
export async function main(args: bigquery) {
const bq = new BigQuery({
	credentials: args
})
const [datasets] = await bq.getDatasets();
if (!datasets) return {}
const schema = {} as any
let queries = datasets.map(dataset => \`
	SELECT 
		table_name, 
		'\${dataset.id}' as dataset,
		ARRAY_AGG(STRUCT(
			if(is_nullable = 'YES', true, false) AS required,
			column_name AS name,
			data_type AS type,
			if(column_default = 'NULL', null, column_default) AS \\\`default\\\`
		) ORDER BY ordinal_position)
		AS schema 
	FROM \\\`\${dataset.id}\\\`.INFORMATION_SCHEMA.COLUMNS 
	GROUP BY table_name\`
)
let query = queries.join('\\nUNION ALL \\n')
const [rows] = await bq.query(query)
for (const row of rows) {
	schema[row.dataset] ??= {}
	schema[row.dataset][row.table_name] = {}
	for (const col of row.schema) {
		const colName = col.name
		delete col.name
		if (col.default === null) {
			delete col.default
		}
		schema[row.dataset][row.table_name][colName] = col
	}
}
return schema
}`, // nested template literals
		lang: 'bun',
		argName: 'args'
	},
	snowflake: {
		code: `select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT, IS_NULLABLE from information_schema.columns where table_schema != 'INFORMATION_SCHEMA'`,
		lang: 'snowflake',
		processingFn: (rows) => {
			const schema = {}
			for (const row of rows) {
				if (!(row.TABLE_SCHEMA in schema)) {
					schema[row.TABLE_SCHEMA] = {}
				}
				if (!(row.TABLE_NAME in schema[row.TABLE_SCHEMA])) {
					schema[row.TABLE_SCHEMA][row.TABLE_NAME] = {}
				}
				schema[row.TABLE_SCHEMA][row.TABLE_NAME][row.COLUMN_NAME] = {
					type: row.DATA_TYPE,
					required: row.IS_NULLABLE === 'YES'
				}
				if (row.COLUMN_DEFAULT !== null) {
					schema[row.TABLE_SCHEMA][row.TABLE_NAME][row.COLUMN_NAME]['default'] = row.COLUMN_DEFAULT
				}
			}
			return schema
		},
		argName: 'database'
	},
	snowflake_oauth: {
		code: `select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT, IS_NULLABLE from information_schema.columns where table_schema != 'INFORMATION_SCHEMA'`,
		lang: 'snowflake',
		processingFn: (rows) => {
			const schema = {}
			for (const row of rows) {
				if (!(row.TABLE_SCHEMA in schema)) {
					schema[row.TABLE_SCHEMA] = {}
				}
				if (!(row.TABLE_NAME in schema[row.TABLE_SCHEMA])) {
					schema[row.TABLE_SCHEMA][row.TABLE_NAME] = {}
				}
				schema[row.TABLE_SCHEMA][row.TABLE_NAME][row.COLUMN_NAME] = {
					type: row.DATA_TYPE,
					required: row.IS_NULLABLE === 'YES'
				}
				if (row.COLUMN_DEFAULT !== null) {
					schema[row.TABLE_SCHEMA][row.TABLE_NAME][row.COLUMN_NAME]['default'] = row.COLUMN_DEFAULT
				}
			}
			return schema
		},
		argName: 'database'
	},
	mssql: {
		argName: 'database',
		code: `select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT from information_schema.columns where table_schema != 'sys'`,
		lang: 'mssql',
		processingFn: (rows) => {
			if (!rows || rows.length === 0) return {}
			const schemas = rows.reduce((acc, a) => {
				const table_schema = a.TABLE_SCHEMA
				delete a.TABLE_SCHEMA
				acc[table_schema] = acc[table_schema] || []
				acc[table_schema].push(a)
				return acc
			}, {})
			const data = {}
			for (const key in schemas) {
				data[key] = schemas[key].reduce((acc, a) => {
					const table_name = a.TABLE_NAME
					delete a.TABLE_NAME
					acc[table_name] = acc[table_name] || {}
					const p: {
						type: string
						required: boolean
						default?: string
					} = {
						type: a.DATA_TYPE,
						required: a.is_nullable === 'NO'
					}
					if (a.column_default) {
						p.default = a.COLUMN_DEFAULT
					}
					acc[table_name][a.COLUMN_NAME] = p
					return acc
				}, {})
			}
			return data
		}
	}
}

// We cannot modify the original legacyScripts because they are used to calculate app policies in AppDbExplorer
// TODO: Refactor the app policy system to avoid this
const scriptsV2: typeof legacyScripts = {
	...legacyScripts,
	postgresql: {
		...legacyScripts.postgresql,
		code: `
SELECT table_name, column_name, udt_name, column_default, is_nullable, nsp.nspname AS table_schema FROM information_schema.columns
RIGHT JOIN pg_namespace nsp ON table_schema = nsp.nspname WHERE nsp.nspname NOT IN ('information_schema', 'pg_toast', 'pg_catalog')`
	}
}

export { legacyScripts, scriptsV2 }

export function formatSchema(dbSchema: {
	lang: SQLSchema['lang']
	schema: SQLSchema['schema']
	publicOnly: SQLSchema['publicOnly']
}) {
	if (dbSchema.publicOnly) {
		return dbSchema.schema.public || dbSchema.schema.PUBLIC || dbSchema.schema.dbo || dbSchema
	} else {
		return dbSchema.schema
	}
}

export async function formatGraphqlSchema(schema: IntrospectionQuery): Promise<string> {
	const { buildClientSchema, printSchema } = await import('graphql')
	return printSchema(buildClientSchema(schema))
}

export function buildVisibleFieldList(columnDefs: ColumnDef[], dbType: DbType) {
	// Filter out hidden columns to avoid counting the wrong number of rows
	return columnDefs
		.filter((columnDef: ColumnDef) => columnDef && columnDef.ignored !== true)
		.map((column) => renderDbQuotedIdentifier(column?.field, dbType))
}

export function renderDbQuotedIdentifier(identifier: string, dbType: DbType): string {
	switch (dbType) {
		case 'postgresql':
			return `"${identifier}"` // PostgreSQL uses double quotes for identifiers
		case 'ms_sql_server':
			return `[${identifier}]` // MSSQL uses square brackets for identifiers
		case 'mysql':
			return `\`${identifier}\`` // MySQL uses backticks
		case 'snowflake':
			return `"${identifier}"` // Snowflake uses double quotes for identifiers
		case 'bigquery':
			return `\`${identifier}\`` // BigQuery uses backticks
		case 'duckdb':
			return `"${identifier}"` // DuckDB uses double quotes for identifiers
		default:
			throw new Error('Unsupported database type: ' + dbType)
	}
}

export function getLanguageByResourceType(name: string): ScriptLang {
	const language = {
		postgresql: 'postgresql',
		mysql: 'mysql',
		ms_sql_server: 'mssql',
		mssql: 'mssql',
		snowflake: 'snowflake',
		snowflake_oauth: 'snowflake',
		bigquery: 'bigquery',
		duckdb: 'duckdb'
	}
	return language[name]
}

export function buildParameters(
	columns: Array<{
		field: string
		datatype: string
	}>,
	databaseType: string
) {
	return columns
		.map((column, i) => {
			switch (databaseType) {
				case 'postgresql':
					return `-- $${i + 1} ${column.field}`
				case 'mysql':
					return `-- :${column.field} (${column.datatype.split('(')[0]})`
				case 'ms_sql_server':
					return `-- @p${i + 1} ${column.field} (${column.datatype.split('(')[0]})`
				case 'snowflake':
					return `-- ? ${column.field} (${column.datatype.split('(')[0]})`
				case 'bigquery':
					return `-- @${column.field} (${column.datatype.split('(')[0]})`
				case 'duckdb':
					return `-- $${column.field} (${column.datatype.split('(')[0]})`
			}
		})
		.join('\n')
}

export function getPrimaryKeys(tableMetadata?: TableMetadata): string[] {
	let r = tableMetadata?.filter((x) => x.isprimarykey)?.map((x) => x.field) ?? []
	if (r?.length === 0) {
		r = tableMetadata?.map((x) => x.field) ?? []
	}
	return r ?? []
}

export function dbSupportsSchemas(dbType: DbType): boolean {
	return dbType === 'postgresql' || dbType === 'snowflake' || dbType === 'bigquery'
}

export function datatypeHasLength(datatype: string): boolean {
	datatype = datatype.toLowerCase()
	const lengthDataTypes = [
		'varchar',
		'char',
		'nvarchar',
		'nchar',
		'varbinary',
		'binary',
		'bit',
		'character varying',
		'character'
	]
	return lengthDataTypes.some((type) => datatype === type)
}

export function sqlDataTypeToJsTypeHeuristic(datatype: string): string {
	datatype = datatype.toLowerCase()
	if (
		datatype.includes('int') ||
		datatype === 'decimal' ||
		datatype === 'numeric' ||
		datatype === 'float' ||
		datatype === 'real' ||
		datatype === 'double'
	) {
		return 'number'
	} else if (
		datatype === 'varchar' ||
		datatype === 'char' ||
		datatype === 'text' ||
		datatype === 'nvarchar' ||
		datatype === 'nchar' ||
		datatype === 'string'
	) {
		return 'string'
	} else if (datatype === 'boolean' || datatype === 'bool' || datatype === 'bit') {
		return 'boolean'
	} else if (
		datatype === 'date' ||
		datatype === 'datetime' ||
		datatype === 'timestamp' ||
		datatype === 'timestamptz'
	) {
		return 'Date'
	} else if (datatype === 'json' || datatype === 'jsonb') {
		return 'object'
	} else {
		return 'any'
	}
}
