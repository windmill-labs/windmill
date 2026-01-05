import { dbSupportsSchemas } from '../utils'
import type { DbType } from '$lib/components/dbTypes'
import { formatDefaultValue } from './dbQueriesUtils'

export type CreateTableValues = {
	name: string
	columns: CreateTableValuesColumn[]
	foreignKeys: CreateForeignKey[]
	pk_constraint_name?: string // Used by alter table to reference existing constraint
}

export type CreateTableValuesColumn = {
	name: string
	datatype: string
	primaryKey?: boolean
	defaultValue?: string
	nullable?: boolean
	datatype_length?: number // e.g varchar(255)

	// Used by alter table. We need to track the original column for data consistency
	// e.g '(a, b) => (x, y, z)' : can't know which column was renamed without this
	initialName?: string
}

export type CreateForeignKey = {
	targetTable?: string
	columns: {
		sourceColumn?: string
		targetColumn?: string
	}[]
	onDelete: 'CASCADE' | 'SET NULL' | 'NO ACTION'
	onUpdate: 'CASCADE' | 'SET NULL' | 'NO ACTION'
	fk_constraint_name?: string // Used by alter table to reference existing constraint
}

export function makeCreateTableQuery(values: CreateTableValues, dbType: DbType, schema?: string) {
	const pkCount = values.columns.reduce((p, c) => p + (c.primaryKey ? 1 : 0), 0)

	function transformColumn(c: CreateTableValuesColumn): string {
		const datatype = c.datatype_length ? `${c.datatype}(${c.datatype_length})` : c.datatype
		const defValue = c.defaultValue && formatDefaultValue(c.defaultValue, datatype, dbType)

		let str = `  ${c.name} ${datatype}`
		if (!c.nullable) str += ' NOT NULL'
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

	const useSchema = dbSupportsSchemas(dbType)

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

export function datatypeDefaultLength(datatype: string): number {
	datatype = datatype.toLowerCase()
	if (datatype == 'bit') return 1
	if (['varchar', 'char', 'nvarchar', 'nchar', 'varbinary', 'binary'].includes(datatype)) {
		return 255
	} else {
		return 10
	}
}
