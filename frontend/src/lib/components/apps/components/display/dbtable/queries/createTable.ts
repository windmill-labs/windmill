import { dbSupportsSchemas, type DbType } from '../utils'

export type CreateTableValues = {
	name: string
	columns: CreateTableValuesColumn[]
	foreignKeys: {
		targetTable?: string
		columns: {
			sourceColumn?: string
			targetColumn?: string
		}[]
		onDelete: 'CASCADE' | 'SET NULL' | 'NO ACTION'
		onUpdate: 'CASCADE' | 'SET NULL' | 'NO ACTION'
	}[]
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
	const pkCount = values.columns.reduce((p, c) => p + (c.primaryKey ? 1 : 0), 0)

	function transformColumn(c: CreateTableValuesColumn): string {
		const datatype = c.datatype_length ? `${c.datatype}(${c.datatype_length})` : c.datatype
		const defValue = c.defaultValue && formatDefaultValue(c.defaultValue, datatype, resourceType)

		let str = `  ${c.name} ${datatype}`
		if (c.not_null) str += ' NOT NULL'
		if (defValue) str += ` DEFAULT ${defValue}`
		if (pkCount === 1 && c.primaryKey) str += ' PRIMARY KEY'
		return str
	}

	function transformFk(fk: CreateTableValues['foreignKeys'][number]): string {
		const sourceColumns = fk.columns.map((c) => c.sourceColumn).filter(Boolean)
		const targetColumns = fk.columns.map((c) => c.targetColumn).filter(Boolean)
		const targetTable =
			useSchema || !fk.targetTable?.includes('.')
				? fk.targetTable
				: fk.targetTable?.split('.').pop()

		let l = `  FOREIGN KEY (${sourceColumns.join(', ')}) REFERENCES ${targetTable} (${targetColumns.join(
			', '
		)})`
		if (fk.onDelete !== 'NO ACTION') l += ` ON DELETE ${fk.onDelete}`
		if (fk.onUpdate !== 'NO ACTION') l += ` ON UPDATE ${fk.onUpdate}`
		return l
	}

	const useSchema = dbSupportsSchemas(resourceType)

	const lines = values.columns.map(transformColumn)
	lines.push(...values.foreignKeys.map(transformFk))
	if (pkCount > 1) {
		const pks = values.columns.filter((c) => c.primaryKey)
		lines.push(`  PRIMARY KEY (${pks.map((c) => c.name).join(', ')})`)
	}

	return `CREATE TABLE ${useSchema && schema ? schema.trim() + '.' : ''}${values.name.trim()} (
${lines.join(',\n')}
);`
}

function formatDefaultValue(str: string, datatype: string, resourceType: DbType): string {
	if (!str) return ''
	if (str.startsWith('{') && str.endsWith('}')) {
		return str.slice(1, str.length - 1)
	}
	if (resourceType === 'postgresql') {
		return `CAST('${str}' AS ${datatype})`
	}
	return `'${str}'`
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
