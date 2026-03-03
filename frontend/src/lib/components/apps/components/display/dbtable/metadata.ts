import { JobService, ResourceService, type ScriptLang } from '$lib/gen'

import { runScriptAndPollResult } from '$lib/components/jobs/utils'
import type { DbInput } from '$lib/components/dbTypes'
import {
	getLanguageByResourceType,
	resourceTypeToLang,
	legacyScripts,
	scriptsV2,
	type TableMetadata
} from './utils'

import { type Preview } from '$lib/gen'
import type { DBSchema, GraphqlSchema, SQLSchema } from '$lib/stores'

import { stringifyGraphqlSchema, stringifySchema } from '$lib/components/copilot/lib'
import type { DbType } from '$lib/components/dbTypes'
import { getDatabaseArg } from '$lib/components/dbOps'

export async function loadTableMetaData(
	input: DbInput,
	workspace: string | undefined,
	table: string | undefined
): Promise<TableMetadata | undefined> {
	if (!input || !table || !workspace) return undefined

	let { language, query } = await makeLoadTableMetaDataQuery(input, workspace, table)

	const job = await JobService.runScriptPreview({
		workspace: workspace,
		requestBody: { language, content: query, args: getDatabaseArg(input) }
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

				const result = testResult.result.map(lowercaseKeys)

				// For Snowflake, fetch primary keys separately
				if (
					input.type === 'database' &&
					(input.resourceType === 'snowflake' || (input.resourceType as any) === 'snowflake_oauth')
				) {
					const map: Record<string, TableMetadata> = { [table]: result }
					await fetchAndAddSnowflakePrimaryKeysInMap(map, input, workspace, table)
					return map[table]
				}

				return result
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

export async function loadAllTablesMetaData(
	workspace: string | undefined,
	input: DbInput
): Promise<Record<string, TableMetadata> | undefined> {
	if (!input || !workspace) return undefined

	try {
		let { language, query } = await makeLoadTableMetaDataQuery(input, workspace, undefined)
		let result = (await runScriptAndPollResult({
			workspace: workspace,
			requestBody: { language, content: query, args: getDatabaseArg(input) }
		})) as ({ table_name: string; schema_name?: string } & object)[]
		const map: Record<string, TableMetadata> = {}

		for (const _col of result) {
			const col = lowercaseKeys(_col)
			const tableKey = col.schema_name ? `${col.schema_name}.${col.table_name}` : col.table_name
			map[tableKey] ??= []
			map[tableKey].push(col)
		}

		await fetchAndAddSnowflakePrimaryKeysInMap(map, input, workspace)

		return map
	} catch (e) {
		throw new Error('Error loading all tables metadata: ' + e)
	}
}

async function makeLoadTableMetaDataQuery(
	input: DbInput,
	workspace: string,
	table: string | undefined
): Promise<{ query: string; language: ScriptLang }> {
	if (input.type === 'ducklake') {
		const query = `ATTACH 'ducklake://${input.ducklake}' AS __ducklake__;
		SELECT
			COLUMN_NAME as field,
			DATA_TYPE as DataType,
			COLUMN_DEFAULT as DefaultValue,
			false as IsPrimaryKey,
			false as IsIdentity,
			IS_NULLABLE as IsNullable,
			false as IsEnum,
			TABLE_NAME as table_name
		FROM information_schema.columns c
		WHERE table_catalog = '__ducklake__' AND table_schema = current_schema()`
		return { query, language: 'duckdb' }
	} else if (input.resourceType === 'mysql') {
		const resourceObj = (await ResourceService.getResourceValue({
			workspace,
			path: input.resourcePath
		})) as any
		const query = `
	SELECT 
			COLUMN_NAME as field,
			COLUMN_TYPE as DataType,
			COLUMN_DEFAULT as DefaultValue,
			CASE WHEN COLUMN_KEY = 'PRI' THEN 1 ELSE 0 END as IsPrimaryKey,
			CASE WHEN EXTRA like '%auto_increment%' THEN 'YES' ELSE 'NO' END as IsIdentity,
			CASE WHEN IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,
			CASE WHEN DATA_TYPE = 'enum' THEN true ELSE false END as IsEnum${
				table
					? ''
					: `,
			TABLE_NAME as table_name`
			}
	FROM 
			INFORMATION_SCHEMA.COLUMNS${
				table
					? `
	WHERE
			TABLE_NAME = '${table.split('.').reverse()[0]}' AND TABLE_SCHEMA = '${
				table.split('.').reverse()[1] ?? resourceObj?.database ?? ''
			}'`
					: `
	WHERE
			TABLE_SCHEMA NOT IN ('mysql', 'performance_schema', 'information_schema', 'sys')`
			}
	ORDER BY
			TABLE_NAME,
			ORDINAL_POSITION;
	`
		return { query, language: 'mysql' }
	} else if (input.resourceType === 'postgresql') {
		const query = `
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
		 WHERE e.enumtypid = a.atttypid FETCH FIRST ROW ONLY) as IsEnum${
				table
					? ''
					: `,
    ns.nspname AS schema_name,
    c.relname AS table_name`
			}
	FROM pg_catalog.pg_attribute a${
		table
			? `
	WHERE a.attrelid = (SELECT c.oid FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace ns ON c.relnamespace = ns.oid WHERE relname = '${
		table.split('.').reverse()[0]
	}' AND ns.nspname = '${table.split('.').reverse()[1] ?? 'public'}')
		AND a.attnum > 0 AND NOT a.attisdropped
		`
			: `
	JOIN pg_catalog.pg_class c ON a.attrelid = c.oid
	JOIN pg_catalog.pg_namespace ns ON c.relnamespace = ns.oid
	WHERE c.relkind = 'r' AND a.attnum > 0 AND NOT a.attisdropped
		AND ns.nspname != 'pg_catalog' AND ns.nspname != 'information_schema'`
	}
	ORDER BY ${table ? 'a.attnum' : 'ns.nspname, c.relname, a.attnum'};
	
	`
		return { query, language: 'postgresql' }
	} else if (input.resourceType === 'ms_sql_server') {
		const query = `
		SELECT
    c.COLUMN_NAME as field,
    c.DATA_TYPE as DataType,
    c.COLUMN_DEFAULT as DefaultValue,
    CASE WHEN COLUMNPROPERTY(OBJECT_ID(c.TABLE_SCHEMA + '.' + c.TABLE_NAME), c.COLUMN_NAME, 'IsIdentity') = 1 THEN 'By Default' ELSE 'No' END as IsIdentity,
    CASE WHEN pk.COLUMN_NAME IS NOT NULL THEN 1 ELSE 0 END as IsPrimaryKey,
    CASE WHEN c.IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,
    CASE WHEN c.DATA_TYPE = 'enum' THEN 1 ELSE 0 END as IsEnum,
    dc.name as default_constraint_name${
			table
				? ''
				: `,
		c.TABLE_NAME as table_name`
		}
FROM
    INFORMATION_SCHEMA.COLUMNS c
    LEFT JOIN (
        SELECT
            ku.TABLE_SCHEMA,
            ku.TABLE_NAME,
            ku.COLUMN_NAME
        FROM
            INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc
            INNER JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE ku
                ON tc.CONSTRAINT_TYPE = 'PRIMARY KEY'
                AND tc.CONSTRAINT_NAME = ku.CONSTRAINT_NAME
                AND tc.TABLE_SCHEMA = ku.TABLE_SCHEMA
                AND tc.TABLE_NAME = ku.TABLE_NAME
    ) pk ON c.TABLE_SCHEMA = pk.TABLE_SCHEMA
        AND c.TABLE_NAME = pk.TABLE_NAME
        AND c.COLUMN_NAME = pk.COLUMN_NAME
    LEFT JOIN sys.default_constraints dc
        ON dc.parent_object_id = OBJECT_ID(c.TABLE_SCHEMA + '.' + c.TABLE_NAME)
        AND dc.parent_column_id = COLUMNPROPERTY(OBJECT_ID(c.TABLE_SCHEMA + '.' + c.TABLE_NAME), c.COLUMN_NAME, 'ColumnId')${
					table
						? `
WHERE
    c.TABLE_NAME = '${table}'`
						: ''
				}
ORDER BY
    c.ORDINAL_POSITION;
	`
		return { query, language: 'mssql' }
	} else if (
		input.resourceType === 'snowflake' ||
		(input.resourceType as any) === 'snowflake_oauth'
	) {
		const query = `
		select COLUMN_NAME as field,
		DATA_TYPE as DataType,
		COLUMN_DEFAULT as DefaultValue,
		CASE WHEN COLUMN_DEFAULT like 'AUTOINCREMENT%' THEN 'By Default' ELSE 'No' END as IsIdentity,
		0 as IsPrimaryKey, -- a one-query solution is not trivial, we will use SHOW PRIMARY KEYS separately
		CASE WHEN IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,
		CASE WHEN DATA_TYPE = 'enum' THEN 1 ELSE 0 END as IsEnum${
			table
				? ''
				: `,
		table_name as table_name,
		table_schema as schema_name`
		}
	from information_schema.columns${
		table
			? `
	where table_name = '${table.split('.').reverse()[0]}' and table_schema = '${
		table.split('.').reverse()[1] ?? 'PUBLIC'
	}'`
			: "\nwhere table_schema <> 'INFORMATION_SCHEMA'\n"
	}
	order by ORDINAL_POSITION;
	`
		return { query, language: 'snowflake' }
	} else if (input.resourceType === 'bigquery') {
		if (table) {
			const query = `SELECT 
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
    ${table.split('.')[0]}.INFORMATION_SCHEMA.KEY_COLUMN_USAGE p
    on c.table_name = p.table_name AND c.column_name = p.COLUMN_NAME
WHERE   
    c.TABLE_NAME = '${table.split('.')[1]}'
order by c.ORDINAL_POSITION;`
			return { query, language: 'bigquery' }
		} else {
			const query = `import { BigQuery } from '@google-cloud/bigquery@7.5.0';
export async function main(database: bigquery) {
const bq = new BigQuery({
	credentials: database
})
const [datasets] = await bq.getDatasets();
if (!datasets) return {}
const schema = {} as any
let queries = datasets.map(dataset => \`
	(SELECT 
    c.COLUMN_NAME as field,
		'\${dataset.id}' as schema_name,
		c.TABLE_NAME as table_name,
    DATA_TYPE as DataType,
    CASE WHEN COLUMN_DEFAULT = 'NULL' THEN '' ELSE COLUMN_DEFAULT END as DefaultValue,
    CASE WHEN constraint_name is not null THEN true ELSE false END as IsPrimaryKey,
    'No' as IsIdentity,
    IS_NULLABLE as IsNullable,
    false as IsEnum
FROM
    \\\`\${dataset.id}\\\`.INFORMATION_SCHEMA.COLUMNS c
    LEFT JOIN
    \\\`\${dataset.id}\\\`.INFORMATION_SCHEMA.KEY_COLUMN_USAGE p
    on c.table_name = p.table_name AND c.column_name = p.COLUMN_NAME
ORDER BY c.ORDINAL_POSITION)\`
)
let query = queries.join('\\nUNION ALL \\n')
const [rows] = await bq.query(query)
return rows
}`
			return { query, language: 'bun' }
		}
	} else {
		throw new Error('Unsupported database type:' + input.resourceType)
	}
}

type SnowflakeShowPrimaryKeysResult = {
	column_name: string
	database_name: string
	schema_name: string
	table_name: string
}

// We can't get primary keys in a single query for Snowflake, so we fetch them separately
async function fetchAndAddSnowflakePrimaryKeysInMap(
	map: Record<string, TableMetadata>,
	input: DbInput,
	workspace: string,
	tableKey?: string
) {
	if (
		input.type == 'database' &&
		(input.resourceType === 'snowflake' || (input.resourceType as any) === 'snowflake_oauth')
	) {
		let pkResult = await fetchSnowflakePrimaryKeys(workspace, getDatabaseArg(input), tableKey)
		for (const pk of pkResult) {
			const pkTableKey = `${pk.schema_name}.${pk.table_name}`.toUpperCase()
			// Also check the original casing and the provided tableKey
			const keysToCheck = [pkTableKey, `${pk.schema_name}.${pk.table_name}`, tableKey].filter(
				Boolean
			) as string[]
			for (const key of keysToCheck) {
				if (key in map) {
					for (const col of map[key]) {
						if (col.field.toLowerCase() === pk.column_name.toLowerCase()) {
							col.isprimarykey = true
						}
					}
				}
			}
		}
	}
}

async function fetchSnowflakePrimaryKeys(
	workspace: string,
	dbArg: any,
	tableKey?: string
): Promise<SnowflakeShowPrimaryKeysResult[]> {
	return (await JobService.runScriptPreviewAndWaitResult({
		workspace,
		requestBody: {
			language: 'snowflake',
			args: dbArg,
			content: tableKey ? `SHOW PRIMARY KEYS IN TABLE ${tableKey}` : 'SHOW PRIMARY KEYS IN ACCOUNT'
		}
	})) as SnowflakeShowPrimaryKeysResult[]
}

function lowercaseKeys(obj: Record<string, any>): any {
	const newObj = {}
	Object.keys(obj).forEach((key) => {
		newObj[key.toLowerCase()] = obj[key]
	})
	return newObj
}

export async function getDbSchemas(
	resourceType: string,
	resourcePath: string,
	workspace: string | undefined,
	errorCallback: (message: string) => void,
	options: {
		useLegacyScripts?: boolean // To avoid breaking app policies
		customTag?: string
	} = {}
): Promise<DBSchema | undefined> {
	let scripts = options.useLegacyScripts ? legacyScripts : scriptsV2
	let sqlScript = scripts[getLanguageByResourceType(resourceType)]

	if (!resourceType || !resourcePath || !workspace || !sqlScript) return

	let result: unknown
	try {
		result = await JobService.runScriptPreviewAndWaitResult({
			workspace,
			requestBody: {
				language: sqlScript.lang as Preview['language'],
				content: typeof sqlScript.code === 'function' ? await sqlScript.code() : sqlScript.code,
				tag: options.customTag,
				args: {
					[sqlScript.argName]: resourcePath.startsWith('datatable://')
						? resourcePath
						: '$res:' + resourcePath
				}
			}
		})
	} catch (e) {
		console.error(e)
		errorCallback('Error fetching schema: ' + ((e as Error)?.message || e))
		return
	}

	if (resourceType !== undefined) {
		if (resourceType !== 'graphql') {
			const { processingFn } = sqlScript
			let schema: any
			try {
				schema = processingFn !== undefined ? processingFn(result) : result
			} catch (e) {
				console.error(e)
				errorCallback('Error processing schema')
				return
			}
			const dbSchema = {
				lang: resourceTypeToLang(resourceType) as SQLSchema['lang'],
				schema,
				publicOnly: !!schema.public || !!schema.PUBLIC || !!schema.dbo
			}
			return { ...dbSchema, stringified: stringifySchema(dbSchema) }
		} else {
			if (typeof result !== 'object' || !('__schema' in (result ?? {}))) {
				console.error('Invalid GraphQL schema')
				errorCallback('Invalid GraphQL schema')
				return
			} else {
				const dbSchema = {
					lang: 'graphql' as GraphqlSchema['lang'],
					schema: result
				}
				return {
					...(dbSchema as any),
					stringified: await stringifyGraphqlSchema(result)
				}
			}
		}
	}
}

export async function getTablesByResource(
	schema: Partial<Record<string, DBSchema>>,
	dbType: DbType | undefined,
	dbPath: string,
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
				path: dbPath.split('$res:')[1]
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
		case 'duckdb': {
			const paths: string[] = []
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					paths.push(`${subKey}`)
				}
			}

			return paths
		}
		default:
			return []
	}
}
