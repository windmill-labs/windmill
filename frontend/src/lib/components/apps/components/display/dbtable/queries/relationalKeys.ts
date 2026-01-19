import type { DbInput, DbType } from '$lib/components/dbTypes'
import { wrapDucklakeQuery } from '$lib/components/ducklake'
import { runScriptAndPollResult } from '$lib/components/jobs/utils'
import type { ScriptLang } from '$lib/gen'
import type { TableEditorForeignKey } from '../tableEditor'

/**
 * Raw foreign key result from database queries
 */
export type RawForeignKey = {
	fk_constraint_name: string
	source_column: string
	target_table: string
	target_column: string
	on_delete: 'CASCADE' | 'SET NULL' | 'NO ACTION'
	on_update: 'CASCADE' | 'SET NULL' | 'NO ACTION'
}

/**
 * Raw primary key constraint result from database queries
 */
export type RawPrimaryKeyConstraint = {
	constraint_name: string
}

/**
 * Generates SQL query to fetch foreign key information for a specific table
 * @param dbType - The database type
 * @param table - The table name (can include schema, e.g., "schema.table")
 * @param defaultSchema - The default schema to use when table doesn't include one
 * @returns SQL query string
 */
export function makeForeignKeysQuery(
	dbType: DbType,
	table: string,
	defaultSchema?: string
): string {
	const tableParts = table.split('.')
	const tableName = tableParts[tableParts.length - 1]
	const schemaName = tableParts.length > 1 ? tableParts[0] : defaultSchema

	switch (dbType) {
		case 'postgresql':
			return makePostgresForeignKeysQuery(tableName, schemaName || 'public')
		case 'mysql':
			return makeMysqlForeignKeysQuery(tableName, schemaName || '')
		case 'ms_sql_server':
			return makeMsSqlForeignKeysQuery(tableName, schemaName || 'dbo')
		case 'snowflake':
			return makeSnowflakeForeignKeysQuery(tableName, schemaName || 'PUBLIC')
		case 'bigquery':
			return makeBigQueryForeignKeysQuery(tableName, schemaName)
		case 'duckdb':
			return makeDuckDbForeignKeysQuery(tableName, schemaName || 'main')
		default:
			throw new Error(`Unsupported database type: ${dbType}`)
	}
}

/**
 * Generates SQL query to fetch primary key constraint name for a specific table
 * @param dbType - The database type
 * @param table - The table name (can include schema, e.g., "schema.table")
 * @param defaultSchema - The default schema to use when table doesn't include one
 * @returns SQL query string
 */
export function makePrimaryKeyConstraintQuery(
	dbType: DbType,
	table: string,
	defaultSchema?: string
): string {
	const tableParts = table.split('.')
	const tableName = tableParts[tableParts.length - 1]
	const schemaName = tableParts.length > 1 ? tableParts[0] : defaultSchema

	switch (dbType) {
		case 'postgresql':
			return makePostgresPrimaryKeyQuery(tableName, schemaName || 'public')
		case 'mysql':
			return makeMysqlPrimaryKeyQuery(tableName, schemaName || '')
		case 'ms_sql_server':
			return makeMsSqlPrimaryKeyQuery(tableName, schemaName || 'dbo')
		case 'snowflake':
			return makeSnowflakePrimaryKeyQuery(tableName, schemaName || 'PUBLIC')
		case 'bigquery':
			return makeBigQueryPrimaryKeyQuery(tableName, schemaName)
		case 'duckdb':
			return makeDuckDbPrimaryKeyQuery(tableName, schemaName || 'main')
		default:
			throw new Error(`Unsupported database type: ${dbType}`)
	}
}

/**
 * Transforms raw foreign key results from database into TableEditorForeignKey format
 * Groups by constraint name and aggregates columns
 */
export function transformForeignKeys(rawResults: RawForeignKey[]): TableEditorForeignKey[] {
	// Group by constraint name
	const grouped = new Map<string, RawForeignKey[]>()

	for (const fk of rawResults) {
		const existing = grouped.get(fk.fk_constraint_name) || []
		existing.push(fk)
		grouped.set(fk.fk_constraint_name, existing)
	}

	// Transform to TableEditorForeignKey format
	return Array.from(grouped.values()).map((fks) => {
		const firstFk = fks[0]
		return {
			fk_constraint_name: firstFk.fk_constraint_name,
			targetTable: firstFk.target_table,
			columns: fks.map((fk) => ({
				sourceColumn: fk.source_column,
				targetColumn: fk.target_column
			})),
			onDelete: firstFk.on_delete,
			onUpdate: firstFk.on_update
		}
	})
}

// PostgreSQL queries
function makePostgresForeignKeysQuery(tableName: string, schemaName: string): string {
	return `
SELECT
    tc.constraint_name as fk_constraint_name,
    kcu.column_name as source_column,
    ccu.table_schema || '.' || ccu.table_name as target_table,
    ccu.column_name as target_column,
    COALESCE(rc.delete_rule, 'NO ACTION') as on_delete,
    COALESCE(rc.update_rule, 'NO ACTION') as on_update
FROM
    information_schema.table_constraints AS tc
    JOIN information_schema.key_column_usage AS kcu
        ON tc.constraint_name = kcu.constraint_name
        AND tc.table_schema = kcu.table_schema
    JOIN information_schema.constraint_column_usage AS ccu
        ON ccu.constraint_name = tc.constraint_name
        AND ccu.table_schema = tc.table_schema
    LEFT JOIN information_schema.referential_constraints AS rc
        ON rc.constraint_name = tc.constraint_name
        AND rc.constraint_schema = tc.table_schema
WHERE
    tc.constraint_type = 'FOREIGN KEY'
    AND tc.table_name = '${tableName}'
    AND tc.table_schema = '${schemaName}'
ORDER BY
    tc.constraint_name, kcu.ordinal_position;
`.trim()
}

function makePostgresPrimaryKeyQuery(tableName: string, schemaName: string): string {
	return `
SELECT
    tc.constraint_name
FROM
    information_schema.table_constraints AS tc
WHERE
    tc.constraint_type = 'PRIMARY KEY'
    AND tc.table_name = '${tableName}'
    AND tc.table_schema = '${schemaName}'
LIMIT 1;
`.trim()
}

// MySQL queries
function makeMysqlForeignKeysQuery(tableName: string, schemaName: string): string {
	const whereClause = schemaName
		? `AND kcu.TABLE_SCHEMA = '${schemaName}'`
		: `AND kcu.TABLE_SCHEMA = DATABASE()`

	return `
SELECT
    kcu.CONSTRAINT_NAME as fk_constraint_name,
    kcu.COLUMN_NAME as source_column,
    CONCAT(kcu.REFERENCED_TABLE_SCHEMA, '.', kcu.REFERENCED_TABLE_NAME) as target_table,
    kcu.REFERENCED_COLUMN_NAME as target_column,
    COALESCE(rc.DELETE_RULE, 'NO ACTION') as on_delete,
    COALESCE(rc.UPDATE_RULE, 'NO ACTION') as on_update
FROM
    INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu
    JOIN INFORMATION_SCHEMA.REFERENTIAL_CONSTRAINTS rc
        ON kcu.CONSTRAINT_NAME = rc.CONSTRAINT_NAME
        AND kcu.TABLE_SCHEMA = rc.CONSTRAINT_SCHEMA
WHERE
    kcu.TABLE_NAME = '${tableName}'
    ${whereClause}
    AND kcu.REFERENCED_TABLE_NAME IS NOT NULL
ORDER BY
    kcu.CONSTRAINT_NAME, kcu.ORDINAL_POSITION;
`.trim()
}

function makeMysqlPrimaryKeyQuery(tableName: string, schemaName: string): string {
	const whereClause = schemaName
		? `AND tc.TABLE_SCHEMA = '${schemaName}'`
		: `AND tc.TABLE_SCHEMA = DATABASE()`

	return `
SELECT
    tc.CONSTRAINT_NAME as constraint_name
FROM
    INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc
WHERE
    tc.CONSTRAINT_TYPE = 'PRIMARY KEY'
    AND tc.TABLE_NAME = '${tableName}'
    ${whereClause}
LIMIT 1;
`.trim()
}

// MS SQL Server queries
function makeMsSqlForeignKeysQuery(tableName: string, schemaName: string): string {
	return `
SELECT
    fk.name as fk_constraint_name,
    COL_NAME(fkc.parent_object_id, fkc.parent_column_id) as source_column,
    OBJECT_SCHEMA_NAME(fkc.referenced_object_id) + '.' + OBJECT_NAME(fkc.referenced_object_id) as target_table,
    COL_NAME(fkc.referenced_object_id, fkc.referenced_column_id) as target_column,
    CASE fk.delete_referential_action
        WHEN 0 THEN 'NO ACTION'
        WHEN 1 THEN 'CASCADE'
        WHEN 2 THEN 'SET NULL'
        WHEN 3 THEN 'SET DEFAULT'
    END as on_delete,
    CASE fk.update_referential_action
        WHEN 0 THEN 'NO ACTION'
        WHEN 1 THEN 'CASCADE'
        WHEN 2 THEN 'SET NULL'
        WHEN 3 THEN 'SET DEFAULT'
    END as on_update
FROM
    sys.foreign_keys fk
    INNER JOIN sys.foreign_key_columns fkc
        ON fk.object_id = fkc.constraint_object_id
    INNER JOIN sys.tables t
        ON fk.parent_object_id = t.object_id
    INNER JOIN sys.schemas s
        ON t.schema_id = s.schema_id
WHERE
    t.name = '${tableName}'
    AND s.name = '${schemaName}'
ORDER BY
    fk.name, fkc.constraint_column_id;
`.trim()
}

function makeMsSqlPrimaryKeyQuery(tableName: string, schemaName: string): string {
	return `
SELECT
    kc.name as constraint_name
FROM
    sys.key_constraints kc
    INNER JOIN sys.tables t
        ON kc.parent_object_id = t.object_id
    INNER JOIN sys.schemas s
        ON t.schema_id = s.schema_id
WHERE
    kc.type = 'PK'
    AND t.name = '${tableName}'
    AND s.name = '${schemaName}';
`.trim()
}

// Snowflake queries
function makeSnowflakeForeignKeysQuery(tableName: string, schemaName: string): string {
	return `SHOW IMPORTED KEYS IN TABLE ${schemaName}.${tableName}`
}

/**
 * Transforms Snowflake SHOW IMPORTED KEYS result to RawForeignKey format
 * Snowflake returns columns: fk_database_name, fk_schema_name, fk_table_name, fk_column_name,
 * pk_database_name, pk_schema_name, pk_table_name, pk_column_name, key_sequence,
 * update_rule, delete_rule, fk_name, pk_name, deferrability
 */
function transformSnowflakeForeignKeys(snowflakeResults: any[]): RawForeignKey[] {
	if (!snowflakeResults || !Array.isArray(snowflakeResults)) {
		return []
	}

	return snowflakeResults.map((row) => ({
		fk_constraint_name: row.fk_name || row.FK_NAME || '',
		source_column: row.fk_column_name || row.FK_COLUMN_NAME || '',
		target_table: `${row.pk_schema_name || row.PK_SCHEMA_NAME || ''}.${row.pk_table_name || row.PK_TABLE_NAME || ''}`,
		target_column: row.pk_column_name || row.PK_COLUMN_NAME || '',
		on_delete: (row.delete_rule || row.DELETE_RULE || 'NO ACTION').toUpperCase() as 'CASCADE' | 'SET NULL' | 'NO ACTION',
		on_update: (row.update_rule || row.UPDATE_RULE || 'NO ACTION').toUpperCase() as 'CASCADE' | 'SET NULL' | 'NO ACTION'
	}))
}

function makeSnowflakePrimaryKeyQuery(tableName: string, schemaName: string): string {
	return `
SELECT
    constraint_name
FROM
    information_schema.table_constraints
WHERE
    constraint_type = 'PRIMARY KEY'
    AND table_name = '${tableName}'
    AND table_schema = '${schemaName}'
LIMIT 1;
`.trim()
}

// BigQuery queries
function makeBigQueryForeignKeysQuery(tableName: string, schemaName?: string): string {
	if (!schemaName) {
		throw new Error('BigQuery requires a dataset (schema) name')
	}

	return `
SELECT
    tc.constraint_name as fk_constraint_name,
    kcu.column_name as source_column,
    ccu.table_name as target_table,
    ccu.column_name as target_column,
    'NO ACTION' as on_delete,
    'NO ACTION' as on_update
FROM
    \`${schemaName}.INFORMATION_SCHEMA.TABLE_CONSTRAINTS\` tc
    JOIN \`${schemaName}.INFORMATION_SCHEMA.KEY_COLUMN_USAGE\` kcu
        ON tc.constraint_name = kcu.constraint_name
    JOIN \`${schemaName}.INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE\` ccu
        ON tc.constraint_name = ccu.constraint_name
WHERE
    tc.constraint_type = 'FOREIGN KEY'
    AND tc.table_name = '${tableName}'
ORDER BY
    tc.constraint_name, kcu.ordinal_position;
`.trim()
}

function makeBigQueryPrimaryKeyQuery(tableName: string, schemaName?: string): string {
	if (!schemaName) {
		throw new Error('BigQuery requires a dataset (schema) name')
	}

	return `
SELECT
    constraint_name
FROM
    \`${schemaName}.INFORMATION_SCHEMA.TABLE_CONSTRAINTS\`
WHERE
    constraint_type = 'PRIMARY KEY'
    AND table_name = '${tableName}'
LIMIT 1;
`.trim()
}

// DuckDB queries
function makeDuckDbForeignKeysQuery(tableName: string, schemaName: string): string {
	// DuckDB supports foreign keys starting from v0.9.0
	return `
SELECT
    fk_constraint.constraint_name as fk_constraint_name,
    kcu.column_name as source_column,
    ccu.table_schema || '.' || ccu.table_name as target_table,
    ccu.column_name as target_column,
    'NO ACTION' as on_delete,
    'NO ACTION' as on_update
FROM
    information_schema.table_constraints fk_constraint
    JOIN information_schema.key_column_usage kcu
        ON fk_constraint.constraint_name = kcu.constraint_name
        AND fk_constraint.constraint_schema = kcu.constraint_schema
    JOIN information_schema.constraint_column_usage ccu
        ON fk_constraint.constraint_name = ccu.constraint_name
        AND fk_constraint.constraint_schema = ccu.constraint_schema
WHERE
    fk_constraint.constraint_type = 'FOREIGN KEY'
    AND fk_constraint.table_name = '${tableName}'
    AND fk_constraint.table_schema = '${schemaName}'
ORDER BY
    fk_constraint.constraint_name, kcu.ordinal_position;
`.trim()
}

function makeDuckDbPrimaryKeyQuery(tableName: string, schemaName: string): string {
	return `
SELECT
    constraint_name
FROM
    information_schema.table_constraints
WHERE
    constraint_type = 'PRIMARY KEY'
    AND table_name = '${tableName}'
    AND table_schema = '${schemaName}'
LIMIT 1;
`.trim()
}

export async function fetchTableRelationalKeys(
	input: DbInput,
	dbType: DbType,
	table: string,
	schema: string | undefined,
	workspace: string,
	dbArg: Record<string, any>,
	language: ScriptLang
): Promise<{
	foreignKeys: TableEditorForeignKey[]
	pk_constraint_name?: string
}> {
	let fkPromise = async () => {
		try {
			if (dbType !== 'bigquery') {
				let fkQuery = makeForeignKeysQuery(dbType, table, schema)
				if (input.type === 'ducklake') fkQuery = wrapDucklakeQuery(fkQuery, input.ducklake)

				const fkResult = await runScriptAndPollResult({
					workspace,
					requestBody: { args: dbArg, content: fkQuery, language }
				})

				let rawForeignKeys: RawForeignKey[]

				// Snowflake returns a different format from SHOW IMPORTED KEYS
				if (dbType === 'snowflake') {
					rawForeignKeys = transformSnowflakeForeignKeys(fkResult as any[])
				} else {
					rawForeignKeys = fkResult as RawForeignKey[]

					if (rawForeignKeys && Array.isArray(rawForeignKeys)) {
						// Lowercase keys for consistency
						rawForeignKeys = rawForeignKeys.map((fk) => {
							const lowerFk: any = {}
							Object.keys(fk).forEach((key) => {
								lowerFk[key.toLowerCase()] = fk[key]
							})
							return lowerFk
						})
					}
				}

				if (rawForeignKeys && Array.isArray(rawForeignKeys)) {
					return transformForeignKeys(rawForeignKeys)
				}
			}
		} catch (e) {
			console.warn('Failed to fetch foreign keys:', e)
		}
		return []
	}

	let pkPromise = async () => {
		try {
			if (dbType !== 'bigquery' && dbType !== 'mysql') {
				let pkQuery = makePrimaryKeyConstraintQuery(dbType, table, schema)
				if (input.type === 'ducklake') pkQuery = wrapDucklakeQuery(pkQuery, input.ducklake)

				const pkResult = await runScriptAndPollResult({
					workspace,
					requestBody: { args: dbArg, content: pkQuery, language }
				})

				let rawPkResult = pkResult as RawPrimaryKeyConstraint[]

				if (rawPkResult && Array.isArray(rawPkResult) && rawPkResult.length > 0) {
					const pkRecord: any = rawPkResult[0]
					const pk_constraint_name: string =
						pkRecord?.constraint_name || pkRecord?.CONSTRAINT_NAME || ''
					return pk_constraint_name
				}
			}
		} catch (e) {
			console.warn('Failed to fetch primary key constraint:', e)
		}
	}

	const [foreignKeys, pk_constraint_name] = await Promise.all([fkPromise(), pkPromise()])

	return { foreignKeys, pk_constraint_name }
}
