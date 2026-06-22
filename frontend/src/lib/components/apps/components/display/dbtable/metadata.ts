import { JobService } from '$lib/gen'

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
import { getDatabaseArg, getDbType } from '$lib/components/dbOps'
import { sendUserToast } from '$lib/toast'

function makeMetadataMarker(
	op: string,
	payload: Record<string, unknown>,
	ducklake: string | undefined
): string {
	if (ducklake) payload.ducklake = ducklake
	return `-- WM_INTERNAL_DB_${op} ${JSON.stringify(payload)}`
}

export async function loadTableMetaData(
	input: DbInput,
	workspace: string | undefined,
	table: string | undefined
): Promise<TableMetadata | undefined> {
	if (!input || !table || !workspace) return undefined

	const dbType = getDbType(input)
	const language = getLanguageByResourceType(dbType)
	const ducklake = input.type === 'ducklake' ? input.ducklake : undefined
	const dbArg = getDatabaseArg(input)

	// MySQL: the metadata query resolves the database name server-side (it falls
	// back to `DATABASE()`), so we don't read the resource value client-side for it.
	const content = makeMetadataMarker('LOAD_TABLE_METADATA', { table }, ducklake)

	const job = await JobService.runScriptPreview({
		workspace,
		requestBody: { language, content, args: dbArg }
	})

	const maxRetries = 8
	let attempts = 0
	while (attempts < maxRetries) {
		try {
			await new Promise((resolve) => setTimeout(resolve, 1000 * (attempts || 0.6)))

			const testResult = (await JobService.getCompletedJob({
				workspace,
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
		const dbType = getDbType(input)
		const dbArg = getDatabaseArg(input)
		const ducklake = input.type === 'ducklake' ? input.ducklake : undefined

		const language = getLanguageByResourceType(dbType)
		// MySQL db name is resolved server-side via `DATABASE()` (see loadTableMetaData);
		// no client-side resource-value read.
		const content = makeMetadataMarker('LOAD_TABLE_METADATA', { table: undefined }, ducklake)

		let result = (await runScriptAndPollResult({
			workspace,
			requestBody: { language, content, args: dbArg }
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
		sendUserToast('Error loading tables metadata: ' + e, 'error')
		throw e
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
	const payload: Record<string, unknown> = {}
	if (tableKey) payload.table = tableKey
	const content = makeMetadataMarker('SNOWFLAKE_PRIMARY_KEYS', payload, undefined)
	return (await JobService.runScriptPreviewAndWaitResult({
		workspace,
		requestBody: {
			language: 'snowflake',
			args: dbArg,
			content
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
				publicOnly: !!schema.public || !!schema.PUBLIC || !!schema.dbo,
				// MySQL introspection selects `DATABASE() AS default_db_name`; carry it
				// so the table picker can tell the default db apart from other visible
				// schemas. Other dbs don't return it (stays undefined).
				defaultDb: Array.isArray(result) ? (result[0] as any)?.default_db_name : undefined
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
	dbType: DbType | undefined
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
			// MySQL introspection lists DATABASE() plus any other visible non-system
			// schemas. Show the default db's tables unprefixed and the rest as
			// `db.table` — matching the pre-removal behavior (which matched the
			// resource's `database`); `defaultDb` is the connection's DATABASE().
			const defaultDb = s && 'defaultDb' in s ? s.defaultDb : undefined
			const paths: string[] = []
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					if (key === defaultDb) {
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
					paths.push(key === 'main' ? `${subKey}` : `${key}.${subKey}`)
				}
			}

			return paths
		}
		default:
			return []
	}
}
