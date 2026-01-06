import type { ColumnMetadata, TableMetadata } from './utils'

export type TableEditorValues = {
	name: string
	columns: TableEditorValuesColumn[]
	foreignKeys: TableEditorForeignKey[]
	pk_constraint_name?: string // Used by alter table to reference existing constraint
}

export type TableEditorValuesColumn = {
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

export type TableEditorForeignKey = {
	targetTable?: string
	columns: {
		sourceColumn?: string
		targetColumn?: string
	}[]
	onDelete: 'CASCADE' | 'SET NULL' | 'NO ACTION'
	onUpdate: 'CASCADE' | 'SET NULL' | 'NO ACTION'
	fk_constraint_name?: string // Used by alter table to reference existing constraint
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

export function buildTableEditorValues({
	metadata,
	tableName,
	pk_constraint_name,
	foreignKeys
}: {
	metadata: TableMetadata
	tableName: string
	pk_constraint_name: string
	foreignKeys: TableEditorForeignKey[]
}): TableEditorValues {
	return {
		name: tableName,
		columns: metadata.map(columnDefToTableEditorValuesColumn),
		pk_constraint_name,
		foreignKeys
	}
}

export function columnDefToTableEditorValuesColumn(
	colDef: ColumnMetadata
): TableEditorValuesColumn {
	let datatype: string
	let datatype_length: number | undefined

	const match = colDef.datatype?.match(/^([\w\s]+?)(?:\((\d+)\))?(\[\])?$/)
	if (match) {
		datatype = match[1].replace(/\s+/g, ' ').trim().toUpperCase() + (match[3] || '') // Normalize spaces and add '[]' if it's an array type
		datatype_length = match[2] ? parseInt(match[2], 10) : datatypeDefaultLength(datatype)
	} else {
		datatype = colDef.datatype?.replace(/\s+/g, ' ').toUpperCase() || 'UNKNOWN'
	}

	return {
		name: colDef.field,
		primaryKey: colDef.isprimarykey,
		defaultValue: colDef.defaultvalue ? `{${colDef.defaultvalue}}` : undefined,
		nullable: colDef.isnullable !== 'NO',
		datatype,
		datatype_length,
		initialName: colDef.field
	}
}
