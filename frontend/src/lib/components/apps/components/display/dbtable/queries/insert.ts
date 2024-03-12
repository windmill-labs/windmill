import type { AppInput } from '$lib/components/apps/inputType'
import { buildParameters, type DbType } from '../utils'
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
		default:
			throw new Error('Unsupported database type')
	}
}

function formatColumnNames(columns: ColumnDef[]): string {
	return columns.map((c) => c.field).join(', ')
}

function formatDefaultValues(columns: ColumnDef[]): string {
	const defaultValues = columns
		.map((c) => {
			if (c.defaultValueNull) {
				return 'NULL'
			} else {
				return typeof c.defaultUserValue === 'string'
					? `'${c.defaultUserValue}'`
					: c.defaultUserValue
			}
		})
		.join(', ')

	return defaultValues
}

export function makeInsertQuery(table: string, columns: ColumnDef[], dbType: DbType) {
	if (!table) throw new Error('Table name is required')

	const columnsInsert = columns.filter((x) => !x.hideInsert)
	const columnsDefault = columns.filter(
		(x) => x.hideInsert && (x.overrideDefaultValue || x.defaultvalue === null) && !x.isidentity
	)

	const allInsertColumns = columnsInsert.concat(columnsDefault)

	let query = buildParameters(allInsertColumns, dbType)

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
