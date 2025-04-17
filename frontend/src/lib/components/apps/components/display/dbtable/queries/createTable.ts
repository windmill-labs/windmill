import { dbSupportsSchemas, type DbType } from '../utils'

export type CreateTableValues = {
	name: string
	columns: CreateTableValuesColumn[]
}

type CreateTableValuesColumn = {
	name: string
	datatype: string
	primaryKey?: boolean
	defaultValue?: string
	not_null?: boolean
	datatype_length?: number // e.g varchar(255)
}

export function makeCreateTableQuery(
	values: CreateTableValues,
	resourceType: DbType,
	schema?: string
) {
	function transformColumn(c: CreateTableValuesColumn): string {
		const defValue = c.defaultValue && formatDefaultValue(c.defaultValue, c.datatype, resourceType)

		let str = `  ${c.name} ${c.datatype}`
		if (c.datatype_length) str += ` (${c.datatype_length})`
		if (c.not_null) str += ' NOT NULL'
		if (defValue) str += ` DEFAULT ${defValue}`
		if (c.primaryKey) str += ' PRIMARY KEY'
		return str
	}

	const useSchema = dbSupportsSchemas(resourceType)

	return `CREATE TABLE ${useSchema && schema ? schema.trim() + '.' : ''}${values.name.trim()} (
${values.columns.map(transformColumn).join(',\n')}
);`
}

function formatDefaultValue(str: string, datatype: string, resourceType: DbType): string {
	if (!str) return ''
	if (str.startsWith('{') && str.endsWith('}')) {
		return str.slice(1, str.length - 1)
	}
	switch (resourceType) {
		case 'postgresql':
			return `'${str}'::${datatype}`
		default:
			throw 'TODO: Unimplemented db type (formatDefaultValue()) !'
	}
}

export function datatypeDefaultLength(datatype: string): number {
	datatype = datatype.toLowerCase()
	if (datatype == 'bit') return 1
	if (['varchar', 'char', 'nvarchar', 'nchar', 'varbinary', 'binary'].includes(datatype)) {
		return 255
	} else {
		return 10
	}
}
