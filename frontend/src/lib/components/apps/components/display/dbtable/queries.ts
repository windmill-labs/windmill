import { type ColumnDef } from './utils'

// INSERT

export function makeMySQLInsertQuery(table: string, columns: ColumnDef[]) {
	if (!table) throw new Error('Table name is required')

	const columnsInsert = columns.filter((x) => !x.hideInsert)
	const columnsDefault = columns.filter(
		(x) => x.hideInsert && (x.overrideDefaultValue || x.defaultvalue === null)
	)

	const allInsertColumns = columnsInsert.concat(columnsDefault)
	const query = `
${buildParamters(columnsInsert, 'mysql')}

INSERT INTO ${table} (${allInsertColumns.map((c) => c.field).join(', ')}) 
VALUES (${columnsInsert.map((c) => `:${c.field}`).join(', ')}${
		columnsDefault.length > 0 ? ',' : ''
	} ${columnsDefault
		.map((c) => (c.defaultValueNull ? 'NULL' : `${c.defaultUserValue}`))
		.join(', ')})`

	return query
}

export function makePostgresInsertQuery(table: string, columns: ColumnDef[]) {
	if (!table) throw new Error('Table name is required')

	const columnsInsert = columns.filter((x) => !x.hideInsert)
	const columnsDefault = columns.filter(
		(x) => x.hideInsert && (x.overrideDefaultValue || x.defaultvalue === null)
	)

	const allInsertColumns = columnsInsert.concat(columnsDefault)
	const query = `
${buildParamters(columnsInsert, 'postgresql')}

INSERT INTO ${table} (${allInsertColumns.map((c) => c.field).join(', ')}) 
VALUES (${columnsInsert.map((c, i) => `$${i + 1}::${c.datatype}`).join(', ')}${
		columnsDefault.length > 0 ? ',' : ''
	} ${columnsDefault
		.map((c) => (c.defaultValueNull ? 'NULL' : `${c.defaultUserValue}::${c.datatype}`))
		.join(', ')})`

	return query
}

export function buildParamters(
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
				case 'mssql':
					return `-- @${column.field} (${column.datatype.split('(')[0]})`
			}
		})
		.join('\n')
}
