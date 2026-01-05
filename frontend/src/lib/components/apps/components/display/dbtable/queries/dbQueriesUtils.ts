import type { DbType } from '$lib/components/dbTypes'
import type { CreateForeignKey, CreateTableValuesColumn } from './createTable'

export function formatDefaultValue(str: string, datatype: string, resourceType: DbType): string {
	if (!str) return ''
	if (str.startsWith('{') && str.endsWith('}')) {
		return str.slice(1, str.length - 1)
	}
	if (resourceType === 'postgresql') {
		return `CAST('${str}' AS ${datatype})`
	}
	return `'${str}'`
}

export function renderColumn(
	c: CreateTableValuesColumn,
	dbType: DbType,
	primaryKeyModifier: boolean = false
): string {
	const datatype = c.datatype_length ? `${c.datatype}(${c.datatype_length})` : c.datatype
	const defValue = c.defaultValue && formatDefaultValue(c.defaultValue, datatype, dbType)

	let str = `${c.name} ${datatype}`
	if (!c.nullable) str += ' NOT NULL'
	if (defValue) str += ` DEFAULT ${defValue}`
	if (primaryKeyModifier) str += ' PRIMARY KEY'
	return str
}

export function renderForeignKey(
	fk: CreateForeignKey,
	options: {
		useSchema: boolean
		dbType: DbType
		tableName: string
	}
): string {
	const sourceColumns = fk.columns.map((c) => c.sourceColumn).filter(Boolean)
	const targetColumns = fk.columns.map((c) => c.targetColumn).filter(Boolean)
	const targetTable =
		options.useSchema || !fk.targetTable?.includes('.')
			? fk.targetTable
			: fk.targetTable?.split('.').pop()

	let sql = ''

	// In postgres we need to name the constraint in case it's dropped later
	// Convention: use fk_source_target to generate the foreign key name
	sql += `CONSTRAINT fk_${[options.tableName, ...sourceColumns, targetTable, ...targetColumns]
		.join('_')
		.replaceAll('.', '_')} `

	sql += ` FOREIGN KEY (${sourceColumns.join(
		', '
	)}) REFERENCES ${targetTable} (${targetColumns.join(', ')})`
	if (fk.onDelete !== 'NO ACTION') sql += ` ON DELETE ${fk.onDelete}`
	if (fk.onUpdate !== 'NO ACTION') sql += ` ON UPDATE ${fk.onUpdate}`
	return sql
}
