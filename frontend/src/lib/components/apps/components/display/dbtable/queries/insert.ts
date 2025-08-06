import type { AppInput } from '$lib/components/apps/inputType'
import { buildParameters, ColumnIdentity, type DbType } from '../utils'
import { getLanguageByResourceType, type ColumnDef } from '../utils'

function formatInsertValues(columns: ColumnDef[], dbType: DbType, startIndex: number = 1): string {
	switch (dbType) {
		case 'mysql':
			return columns.map((c) => `:${c.field}`).join(', ')
		case 'postgresql':
			return columns.map((c, i) => `$${startIndex + i}::${c.datatype}`).join(', ')
		case 'ms_sql_server':
			return columns.map((c, i) => `@p${startIndex + i}`).join(', ')
		case 'snowflake':
			return columns.map(() => `?`).join(', ')
		case 'bigquery':
			return columns.map((c) => `@${c.field}`).join(', ')
		case 'duckdb':
			return columns.map((c) => `$${c.field}`).join(', ')
		default:
			throw new Error('Unsupported database type')
	}
}

function formatColumnNames(columns: ColumnDef[]): string {
	return columns.map((c) => c.field).join(', ')
}

function getUserDefaultValue(column: ColumnDef) {
	if (column.defaultValueNull) {
		return 'NULL'
	} else if (column.defaultUserValue) {
		return typeof column.defaultUserValue === 'string'
			? `'${column.defaultUserValue}'`
			: column.defaultUserValue
	}
}

function formatDefaultValues(columns: ColumnDef[]): string {
	const defaultValues = columns
		.map((c) => {
			const userDefaultValue = getUserDefaultValue(c)
			if (c.overrideDefaultValue === true) {
				return userDefaultValue
			}

			return userDefaultValue ?? c.defaultvalue
		})
		.join(', ')

	return defaultValues
}

function shouldOmitColumnInInsert(column: ColumnDef) {
	if (!column.hideInsert || column.isidentity === ColumnIdentity.Always) {
		return true
	}

	const userDefaultValue =
		(column.defaultUserValue !== undefined && column.defaultUserValue !== '') ||
		column.defaultValueNull === true
	const dbDefaultValue = Boolean(column.defaultvalue)

	if (column.isnullable === 'NO') {
		if (!userDefaultValue && !dbDefaultValue && column.isidentity === ColumnIdentity.No) {
			throw new Error(`Column ${column.field} is not nullable and has no default value`)
		}

		if (!userDefaultValue && !dbDefaultValue) {
			// Should be omitted if it's an identity column and we have no default value
			return column.isidentity !== ColumnIdentity.No
		}

		// Should be omitted if the user had not provided a default value and the database has a default value
		return !userDefaultValue && dbDefaultValue
	} else if (column.isnullable === 'YES') {
		return !userDefaultValue
	}

	return false
}

export function makeInsertQuery(table: string, columns: ColumnDef[], dbType: DbType) {
	if (!table) throw new Error('Table name is required')

	const columnsInsert = columns.filter((x) => !x.hideInsert)
	const columnsDefault = columns.filter((c) => !shouldOmitColumnInInsert(c))
	const allInsertColumns = columnsInsert.concat(columnsDefault)

	let query = buildParameters(columnsInsert, dbType)

	query += '\n'

	const shouldInsertComma = columnsDefault.length > 0
	const columnNames = formatColumnNames(allInsertColumns)
	const insertValues = formatInsertValues(columnsInsert, dbType)
	const defaultValues = formatDefaultValues(columnsDefault)
	const commaOrEmpty = shouldInsertComma ? ', ' : ''

	query += `INSERT INTO ${table} (${columnNames}) VALUES (${insertValues}${commaOrEmpty}${defaultValues})`

	return query
}

export function getInsertInput(
	table: string,
	columns: ColumnDef[],
	resource: string,
	dbType: DbType
): AppInput {
	return {
		runnable: {
			name: 'AppDbExplorer',
			type: 'runnableByName',
			inlineScript: {
				content: makeInsertQuery(table, columns, dbType),
				language: getLanguageByResourceType(dbType),
				schema: {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					properties: {},
					required: ['database'],
					type: 'object'
				}
			}
		},
		fields: {
			database: {
				type: 'static',
				value: resource,
				fieldType: 'object',
				format: `resource-${dbType}`
			}
		},
		type: 'runnable',
		fieldType: 'object'
	}
}
