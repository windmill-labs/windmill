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
