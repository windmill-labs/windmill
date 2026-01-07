import { dbSupportsSchemas } from '../utils'
import type { DbType } from '$lib/components/dbTypes'
import { formatDefaultValue } from './dbQueriesUtils'
import type { TableEditorValues, TableEditorValuesColumn } from '../tableEditor'

export function makeCreateTableQuery(values: TableEditorValues, dbType: DbType, schema?: string) {
	const pkCount = values.columns.reduce((p, c) => p + (c.primaryKey ? 1 : 0), 0)

	function transformColumn(c: TableEditorValuesColumn): string {
		const datatype = c.datatype_length ? `${c.datatype}(${c.datatype_length})` : c.datatype
		const defValue = c.defaultValue && formatDefaultValue(c.defaultValue, datatype, dbType)

		let str = `  ${c.name} ${datatype}`
		if (!c.nullable) str += ' NOT NULL'
		if (defValue) str += ` DEFAULT ${defValue}`
		if (pkCount === 1 && c.primaryKey) {
			if (dbType === 'bigquery') str += ' PRIMARY KEY NOT ENFORCED'
			else str += ' PRIMARY KEY'
		}
		return str
	}

	function transformFk(fk: TableEditorValues['foreignKeys'][number]): string {
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

	const useSchema = dbSupportsSchemas(dbType)

	const lines = values.columns.map(transformColumn)
	lines.push(...values.foreignKeys.map(transformFk))
	if (pkCount > 1) {
		const pks = values.columns.filter((c) => c.primaryKey)
		let pk = `  PRIMARY KEY (${pks.map((c) => c.name).join(', ')})`
		if (dbType === 'bigquery') pk += ' NOT ENFORCED'
		lines.push(pk)
	}

	return `CREATE TABLE ${useSchema && schema ? schema.trim() + '.' : ''}${values.name.trim()} (
${lines.join(',\n')}
);`
}
