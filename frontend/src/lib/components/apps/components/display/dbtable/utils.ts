import { JobService, type Preview, ResourceService } from '$lib/gen'
import type { DBSchema, DBSchemas, GraphqlSchema, SQLSchema } from '$lib/stores'
import {
	buildClientSchema,
	getIntrospectionQuery,
	printSchema,
	type IntrospectionQuery
} from 'graphql'
import { tryEvery } from '$lib/utils'
import { stringifySchema } from '$lib/components/copilot/lib'

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

export async function loadTableMetaData(
	resource: string,
	workspace: string | undefined,
	table: string | undefined,
	resourceType: string
): Promise<TableMetadata | undefined> {
	if (!resource || !table || !workspace) {
		return undefined
	}

	let code: string = ''

	if (resourceType === 'mysql') {
		const resourceObj = (await ResourceService.getResourceValue({
			workspace,
			path: resource.split(':')[1]
		})) as any
		code = `
	SELECT 
			COLUMN_NAME as field,
			COLUMN_TYPE as DataType,
			COLUMN_DEFAULT as DefaultValue,
			CASE WHEN COLUMN_KEY = 'PRI' THEN 'YES' ELSE 'NO' END as IsPrimaryKey,
			CASE WHEN EXTRA like '%auto_increment%' THEN 'YES' ELSE 'NO' END as IsIdentity,
			CASE WHEN IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,
			CASE WHEN DATA_TYPE = 'enum' THEN true ELSE false END as IsEnum
	FROM 
			INFORMATION_SCHEMA.COLUMNS
	WHERE 
			TABLE_NAME = '${table.split('.').reverse()[0]}' AND TABLE_SCHEMA = '${
				table.split('.').reverse()[1] ?? resourceObj?.database ?? ''
			}'
	ORDER BY 
			ORDINAL_POSITION;
	`
	} else if (resourceType === 'postgresql') {
		code = `
		SELECT 
		a.attname as field,
		pg_catalog.format_type(a.atttypid, a.atttypmod) as DataType,
		(SELECT substring(pg_catalog.pg_get_expr(d.adbin, d.adrelid, true) for 128)
		 FROM pg_catalog.pg_attrdef d
		 WHERE d.adrelid = a.attrelid AND d.adnum = a.attnum AND a.atthasdef) as DefaultValue,
		(SELECT CASE WHEN i.indisprimary THEN true ELSE 'NO' END
		 FROM pg_catalog.pg_class tbl, pg_catalog.pg_class idx, pg_catalog.pg_index i, pg_catalog.pg_attribute att
		 WHERE tbl.oid = a.attrelid AND idx.oid = i.indexrelid AND att.attrelid = tbl.oid
								 AND i.indrelid = tbl.oid AND att.attnum = any(i.indkey) AND att.attname = a.attname LIMIT 1) as IsPrimaryKey,
		CASE a.attidentity
				WHEN 'd' THEN 'By Default'
				WHEN 'a' THEN 'Always'
				ELSE 'No'
		END as IsIdentity,
		CASE a.attnotnull
				WHEN false THEN 'YES'
				ELSE 'NO'
		END as IsNullable,
		(SELECT true
		 FROM pg_catalog.pg_enum e
		 WHERE e.enumtypid = a.atttypid FETCH FIRST ROW ONLY) as IsEnum
	FROM pg_catalog.pg_attribute a
	WHERE a.attrelid = (SELECT c.oid FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace ns ON c.relnamespace = ns.oid WHERE relname = '${
		table.split('.').reverse()[0]
	}' AND ns.nspname = '${table.split('.').reverse()[1] ?? 'public'}')
		AND a.attnum > 0 AND NOT a.attisdropped
	ORDER BY a.attnum;
	
	`
	} else if (resourceType === 'ms_sql_server') {
		code = `
		SELECT 
    COLUMN_NAME as field,
    DATA_TYPE as DataType,
    COLUMN_DEFAULT as DefaultValue,
    CASE WHEN COLUMNPROPERTY(OBJECT_ID(TABLE_NAME), COLUMN_NAME, 'IsIdentity') = 1 THEN 'By Default' ELSE 'No' END as IsIdentity,
    CASE WHEN COLUMNPROPERTY(OBJECT_ID(TABLE_NAME), COLUMN_NAME, 'IsIdentity') = 1 THEN 1 ELSE 0 END as IsPrimaryKey, -- This line still needs correction for primary key identification
    CASE WHEN IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,
    CASE WHEN DATA_TYPE = 'enum' THEN 1 ELSE 0 END as IsEnum
FROM    
    INFORMATION_SCHEMA.COLUMNS
WHERE   
    TABLE_NAME = '${table}'
ORDER BY
    ORDINAL_POSITION;

	`
	} else if (resourceType === 'snowflake' || resourceType === 'snowflake_oauth') {
		code = `
		select COLUMN_NAME as field,
		DATA_TYPE as DataType,
		COLUMN_DEFAULT as DefaultValue,
		CASE WHEN COLUMN_DEFAULT like 'AUTOINCREMENT%' THEN 'By Default' ELSE 'No' END as IsIdentity,
		CASE WHEN COLUMN_DEFAULT like 'AUTOINCREMENT%' THEN 1 ELSE 0 END as IsPrimaryKey,
		CASE WHEN IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,
		CASE WHEN DATA_TYPE = 'enum' THEN 1 ELSE 0 END as IsEnum
	from information_schema.columns
	where table_name = '${table.split('.').reverse()[0]}' and table_schema = '${
		table.split('.').reverse()[1] ?? 'PUBLIC'
	}'
	order by ORDINAL_POSITION;
	`
	} else if (resourceType === 'bigquery') {
		code = `SELECT 
    c.COLUMN_NAME as field,
    DATA_TYPE as DataType,
    CASE WHEN COLUMN_DEFAULT = 'NULL' THEN '' ELSE COLUMN_DEFAULT END as DefaultValue,
    CASE WHEN constraint_name is not null THEN true ELSE false END as IsPrimaryKey,
    'No' as IsIdentity,
    IS_NULLABLE as IsNullable,
    false as IsEnum
FROM
    ${table.split('.')[0]}.INFORMATION_SCHEMA.COLUMNS c
    LEFT JOIN
    test_dataset.INFORMATION_SCHEMA.KEY_COLUMN_USAGE p
    on c.table_name = p.table_name AND c.column_name = p.COLUMN_NAME
WHERE   
    c.TABLE_NAME = "${table.split('.')[1]}"
order by c.ORDINAL_POSITION;`
	} else {
		throw new Error('Unsupported database type:' + resourceType)
	}

	const job = await JobService.runScriptPreview({
		workspace: workspace,
		requestBody: {
			language: getLanguageByResourceType(resourceType),
			content: code,
			args: {
				database: resource
			}
		}
	})

	const maxRetries = 8
	let attempts = 0
	while (attempts < maxRetries) {
		try {
			await new Promise((resolve) => setTimeout(resolve, 1000 * (attempts || 0.6)))

			const testResult = (await JobService.getCompletedJob({
				workspace: workspace,
				id: job
			})) as any

			if (testResult.success) {
				attempts = maxRetries

				if (resourceType === 'ms_sql_server') {
					return lowercaseKeys(testResult.result[0])
				} else {
					return lowercaseKeys(testResult.result)
				}
			} else {
				attempts++
			}
		} catch (error) {
			attempts++
		}
	}

	console.error('Failed to load table metadata after maximum retries.')
	return undefined
}

export function resourceTypeToLang(rt: string) {
	if (rt === 'ms_sql_server') {
		return 'mssql'
	} else {
		return rt
	}
}

function lowercaseKeys(arr: Array<Record<string, any>>): Array<any> {
	return arr.map((obj) => {
		const newObj = {}
		Object.keys(obj).forEach((key) => {
			newObj[key.toLowerCase()] = obj[key]
		})
		return newObj
	})
}

const scripts: Record<
	string,
	{
		code: string
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
				acc[table_schema].push(a)
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
		code: getIntrospectionQuery(),
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
const schema = {}
for (const dataset of datasets) {
	schema[dataset.id] = {}
	const query = "SELECT table_name, ARRAY_AGG(STRUCT(if(is_nullable = 'YES', true, false) AS required, column_name AS name, data_type AS type, if(column_default = 'NULL', null, column_default) AS \`default\`) ORDER BY ordinal_position) AS schema \
FROM \`{dataset.id}\`.INFORMATION_SCHEMA.COLUMNS \
GROUP BY table_name".replace('{dataset.id}', dataset.id)
	const [rows] = await bq.query(query)
	for (const row of rows) {
		schema[dataset.id][row.table_name] = {}
		for (const col of row.schema) {
			const colName = col.name
			delete col.name
			if (col.default === null) {
				delete col.default
			}
			schema[dataset.id][row.table_name][colName] = col
		}
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
	ms_sql_server: {
		argName: 'database',
		code: `select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT from information_schema.columns where table_schema != 'sys'`,
		lang: 'mssql',
		processingFn: (rows) => {
			const schemas = rows[0].reduce((acc, a) => {
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

export { scripts }
export async function getDbSchemas(
	resourceType: string,
	resourcePath: string,
	workspace: string | undefined,
	dbSchemas: DBSchemas,
	errorCallback: (message: string) => void
): Promise<void> {
	return new Promise(async (resolve, reject) => {
		if (!resourceType || !resourcePath || !workspace) {
			resolve()
			return
		}

		const job = await JobService.runScriptPreview({
			workspace: workspace,
			requestBody: {
				language: scripts[resourceType].lang as Preview['language'],
				content: scripts[resourceType].code,
				args: {
					[scripts[resourceType].argName]: '$res:' + resourcePath
				}
			}
		})

		tryEvery({
			tryCode: async () => {
				if (resourcePath) {
					const testResult = await JobService.getCompletedJob({
						workspace,
						id: job
					})
					if (!testResult.success) {
						console.error(testResult.result?.['error']?.['message'])
					} else {
						if (testResult.result === 'WINDMILL_TOO_BIG') {
							console.info('Result is too big, fetching result separately')
							const data = await JobService.getCompletedJobResult({
								workspace,
								id: job
							})
							testResult.result = data
						}
						if (resourceType !== undefined) {
							if (resourceType !== 'graphql') {
								const { processingFn } = scripts[resourceType]
								let schema: any
								try {
									schema =
										processingFn !== undefined ? processingFn(testResult.result) : testResult.result
								} catch (e) {
									console.error(e)
									errorCallback('Error processing schema')
									resolve()
									return
								}
								const dbSchema = {
									lang: resourceTypeToLang(resourceType) as SQLSchema['lang'],
									schema,
									publicOnly: !!schema.public || !!schema.PUBLIC || !!schema.dbo
								}
								dbSchemas[resourcePath] = {
									...dbSchema,
									stringified: stringifySchema(dbSchema)
								}
							} else {
								if (
									typeof testResult.result !== 'object' ||
									!('__schema' in (testResult?.result ?? {}))
								) {
									console.error('Invalid GraphQL schema')

									errorCallback('Invalid GraphQL schema')
								} else {
									const dbSchema = {
										lang: 'graphql' as GraphqlSchema['lang'],
										schema: testResult.result
									}
									dbSchemas[resourcePath] = {
										...(dbSchema as any),
										stringified: stringifySchema(dbSchema as any)
									}
								}
							}
						}
					}
					resolve()
				}
			},
			timeoutCode: async () => {
				console.error('Could not query schema within 5s')
				errorCallback('Could not query schema within 5s')
				try {
					await JobService.cancelQueuedJob({
						workspace,
						id: job,
						requestBody: {
							reason: 'Could not query schema within 5s'
						}
					})
				} catch (err) {
					console.error(err)
				}
				reject()
			},
			interval: 500,
			timeout: 5000
		})
	})
}

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

export function formatGraphqlSchema(schema: IntrospectionQuery): string {
	return printSchema(buildClientSchema(schema))
}

export type DbType = (typeof dbTypes)[number]
export const dbTypes = ['mysql', 'ms_sql_server', 'postgresql', 'snowflake', 'bigquery'] as const
export const isDbType = (str?: string): str is DbType => !!str && dbTypes.includes(str as DbType)

export function buildVisibleFieldList(columnDefs: ColumnDef[], dbType: DbType) {
	// Filter out hidden columns to avoid counting the wrong number of rows
	return columnDefs
		.filter((columnDef: ColumnDef) => columnDef && columnDef.ignored !== true)
		.map((column) => {
			switch (dbType) {
				case 'postgresql':
					return `"${column?.field}"` // PostgreSQL uses double quotes for identifiers
				case 'ms_sql_server':
					return `[${column?.field}]` // MSSQL uses square brackets for identifiers
				case 'mysql':
					return `\`${column?.field}\`` // MySQL uses backticks
				case 'snowflake':
					return `"${column?.field}"` // Snowflake uses double quotes for identifiers
				case 'bigquery':
					return `\`${column?.field}\`` // BigQuery uses backticks
				default:
					throw new Error('Unsupported database type')
			}
		})
}

export function getLanguageByResourceType(name: string): Preview['language'] {
	const language = {
		postgresql: 'postgresql',
		mysql: 'mysql',
		ms_sql_server: 'mssql',
		snowflake: 'snowflake',
		snowflake_oauth: 'snowflake',
		bigquery: 'bigquery'
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

export async function getTablesByResource(
	schema: Partial<Record<string, DBSchema>>,
	dbType: DbType | undefined,
	resourcePath: string,
	workspace: string
): Promise<string[]> {
	const s = Object.values(schema)?.[0]
	switch (dbType) {
		case 'ms_sql_server': {
			const paths: string[] = []
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					if (key === 'dbo') {
						paths.push(`${subKey}`)
					}
				}
			}
			return paths
		}
		case 'mysql': {
			const resourceObj = (await ResourceService.getResourceValue({
				workspace,
				path: resourcePath
			})) as any
			const paths: string[] = []
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					if (key === resourceObj?.database) {
						paths.push(`${subKey}`)
					} else {
						paths.push(`${key}.${subKey}`)
					}
				}
			}
			return paths
		}
		case 'snowflake': {
			const paths: string[] = []
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					if (key === 'PUBLIC') {
						paths.push(`${subKey}`)
					} else {
						paths.push(`${key}.${subKey}`)
					}
				}
			}
			return paths
		}
		case 'postgresql': {
			const paths: string[] = []
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					if (key === 'public') {
						paths.push(`${subKey}`)
					} else {
						paths.push(`${key}.${subKey}`)
					}
				}
			}
			return paths
		}
		case 'bigquery': {
			const paths: string[] = []
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					paths.push(`${key}.${subKey}`)
				}
			}
			return paths
		}

		default:
			return []
	}
}

export function dbSupportsSchemas(dbType: DbType): boolean {
	return dbType === 'postgresql' || dbType === 'snowflake'
}

export function datatypeHasLength(datatype: string): boolean {
	datatype = datatype.toLowerCase()
	const lengthDataTypes = ['varchar', 'char', 'nvarchar', 'nchar', 'varbinary', 'binary', 'bit']
	return lengthDataTypes.some((type) => datatype === type)
}
