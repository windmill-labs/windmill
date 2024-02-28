import type { AppInput } from '$lib/components/apps/inputType'
import { buildParamters, type DbType } from '../utils'
import { getLanguageByResourceType, type ColumnDef } from '../utils'

function makeInsertQuery(table: string, columns: ColumnDef[], dbType: DbType) {
	if (!table) throw new Error('Table name is required')

	const columnsInsert = columns.filter((x) => !x.hideInsert)
	const columnsDefault = columns.filter(
		(x) => x.hideInsert && (x.overrideDefaultValue || x.defaultvalue === null)
	)

	const allInsertColumns = columnsInsert.concat(columnsDefault)
	let query = buildParamters(columnsInsert, dbType)

	query += '\n'

	switch (dbType) {
		case 'mysql':
			query += `INSERT INTO ${table} (${allInsertColumns.map((c) => c.field).join(', ')}) 
      VALUES (${columnsInsert.map((c) => `:${c.field}`).join(', ')}${
				columnsDefault.length > 0 ? ',' : ''
			} ${columnsDefault
				.map((c) => (c.defaultValueNull ? 'NULL' : `${c.defaultUserValue}`))
				.join(', ')})`
			break
		case 'postgresql':
			query += `INSERT INTO ${table} (${allInsertColumns.map((c) => c.field).join(', ')}) 
      VALUES (${columnsInsert.map((c, i) => `$${i + 1}::${c.datatype}`).join(', ')}${
				columnsDefault.length > 0 ? ',' : ''
			} ${columnsDefault
				.map((c) => (c.defaultValueNull ? 'NULL' : `${c.defaultUserValue}::${c.datatype}`))
				.join(', ')})`
			break
		case 'ms_sql_server':
			query += `INSERT INTO ${table} (${allInsertColumns.map((c) => c.field).join(', ')}) 
      VALUES (${columnsInsert.map((c, i) => `@p${i + 1}`).join(', ')}${
				columnsDefault.length > 0 ? ',' : ''
			} ${columnsDefault
				.map((c) => (c.defaultValueNull ? 'NULL' : `${c.defaultUserValue}`))
				.join(', ')})`
			break

		default:
			throw new Error('Unsupported database type')
	}

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
