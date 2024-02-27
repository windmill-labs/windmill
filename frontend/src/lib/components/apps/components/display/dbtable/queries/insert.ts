import type { AppInput } from '$lib/components/apps/inputType'
import { Preview } from '$lib/gen'
import { buildParamters } from '../utils'
import { getLanguageByResourceType, type ColumnDef } from '../utils'

function makeInsertQuery(table: string, columns: ColumnDef[], dbType: Preview.language) {
	if (!table) throw new Error('Table name is required')

	const columnsInsert = columns.filter((x) => !x.hideInsert)
	const columnsDefault = columns.filter(
		(x) => x.hideInsert && (x.overrideDefaultValue || x.defaultvalue === null)
	)

	const allInsertColumns = columnsInsert.concat(columnsDefault)
	let query = buildParamters(columnsInsert, dbType)

	query += '\n'

	switch (dbType) {
		case Preview.language.MYSQL:
			query += `INSERT INTO ${table} (${allInsertColumns.map((c) => c.field).join(', ')}) 
      VALUES (${columnsInsert.map((c) => `:${c.field}`).join(', ')}${
				columnsDefault.length > 0 ? ',' : ''
			} ${columnsDefault
				.map((c) => (c.defaultValueNull ? 'NULL' : `${c.defaultUserValue}`))
				.join(', ')})`
			break
		case Preview.language.POSTGRESQL:
			query += `INSERT INTO ${table} (${allInsertColumns.map((c) => c.field).join(', ')}) 
      VALUES (${columnsInsert.map((c, i) => `$${i + 1}::${c.datatype}`).join(', ')}${
				columnsDefault.length > 0 ? ',' : ''
			} ${columnsDefault
				.map((c) => (c.defaultValueNull ? 'NULL' : `${c.defaultUserValue}::${c.datatype}`))
				.join(', ')})`
			break
		case Preview.language.MSSQL:
			query += `INSERT INTO ${table} (${allInsertColumns.map((c) => c.field).join(', ')}) 
      VALUES (${columnsInsert.map((c) => `:${c.field}`).join(', ')}${
				columnsDefault.length > 0 ? ',' : ''
			} ${columnsDefault
				.map((c) => (c.defaultValueNull ? 'NULL' : `${c.defaultUserValue}`))
				.join(', ')})`
			break

		default:
			throw new Error('Unsupported database type')
	}

	debugger

	return query
}

export function getInsertInput(
	table: string,
	columns: ColumnDef[],
	resource: string,
	dbType: Preview.language
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
