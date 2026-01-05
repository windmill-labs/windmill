import type { DbType } from '$lib/components/dbTypes'
import { deepEqual } from 'fast-equals'
import { dbSupportsSchemas } from '../utils'
import {
	type CreateForeignKey,
	type CreateTableValues,
	type CreateTableValuesColumn
} from './createTable'
import { formatDefaultValue, renderColumn, renderForeignKey } from './dbQueriesUtils'

export type AlterTableValues = {
	name: string
	operations: AlterTableOperation[]
}

type AddColumnOperation = {
	kind: 'addColumn'
	column: CreateTableValuesColumn
}

type DropColumnOperation = {
	kind: 'dropColumn'
	name: string
}

export type AlterColumnOperation = {
	kind: 'alterColumn'
	name: string
	original: CreateTableValuesColumn
	changes: Partial<Omit<CreateTableValuesColumn, 'initialName'>>
}

type AddForeignKeyOperation = {
	kind: 'addForeignKey'
	foreignKey: CreateForeignKey
}

type DropForeignKeyOperation = {
	kind: 'dropForeignKey'
	fk_constraint_name: string
}

type RenameTableOperation = {
	kind: 'renameTable'
	to: string
}

type AddPrimaryKeyOperation = {
	kind: 'addPrimaryKey'
	columns: string[]
}

type DropPrimaryKeyOperation = {
	kind: 'dropPrimaryKey'
	pk_constraint_name: string
}

export type AlterTableOperation =
	| AddColumnOperation
	| DropColumnOperation
	| AlterColumnOperation
	| AddForeignKeyOperation
	| DropForeignKeyOperation
	| RenameTableOperation
	| AddPrimaryKeyOperation
	| DropPrimaryKeyOperation

export function makeAlterTableQuery(
	values: AlterTableValues,
	dbType: DbType,
	schema?: string
): string {
	let queries = makeAlterTableQueries(values, dbType, schema)
	if (queries.length === 0) return ''
	return 'BEGIN;\n' + queries.join('\n') + '\nCOMMIT;'
}

export function makeAlterTableQueries(
	values: AlterTableValues,
	dbType: DbType,
	schema?: string
): string[] {
	const useSchema = dbSupportsSchemas(dbType)
	const tableRef = useSchema && schema ? `${schema.trim()}.${values.name}` : values.name

	const queries: string[] = []

	for (const op of values.operations) {
		switch (op.kind) {
			case 'addColumn':
				queries.push(`ALTER TABLE ${tableRef} ADD COLUMN ${renderColumn(op.column, dbType)};`)
				break

			case 'dropColumn':
				queries.push(`ALTER TABLE ${tableRef} DROP COLUMN ${op.name};`)
				break

			case 'alterColumn':
				queries.push(...renderAlterColumn(tableRef, op, dbType))
				break

			case 'addForeignKey':
				queries.push(
					`ALTER TABLE ${tableRef} ADD ${renderForeignKey(op.foreignKey, {
						useSchema,
						dbType,
						tableName: values.name
					})};`
				)
				break

			case 'dropForeignKey':
				queries.push(renderDropForeignKey(tableRef, op.fk_constraint_name))
				break

			case 'renameTable':
				queries.push(`ALTER TABLE ${tableRef} RENAME TO ${op.to};`)
				break

			case 'addPrimaryKey':
				queries.push(`ALTER TABLE ${tableRef} ADD PRIMARY KEY (${op.columns.join(', ')});`)
				break

			case 'dropPrimaryKey':
				queries.push(renderDropPrimaryKey(tableRef, op.pk_constraint_name))
				break

			default:
				throw new Error('Unimplemented case')
		}
	}

	return queries
}

function renderAlterColumn(tableRef: string, op: AlterColumnOperation, dbType: DbType): string[] {
	const queries: string[] = []
	const { name, changes, original } = op

	if (changes.datatype || changes.datatype_length) {
		const baseDatatype = changes.datatype ?? original.datatype
		const datatypeLength = changes.datatype_length ?? original.datatype_length
		const datatype = datatypeLength ? `${baseDatatype}(${datatypeLength})` : baseDatatype
		queries.push(`ALTER TABLE ${tableRef} ALTER COLUMN ${name} TYPE ${datatype};`)
	}

	if ('defaultValue' in changes) {
		if (!changes.defaultValue && original.defaultValue) {
			queries.push(`ALTER TABLE ${tableRef} ALTER COLUMN ${name} DROP DEFAULT;`)
		} else if (changes.defaultValue) {
			const def = formatDefaultValue(changes.defaultValue, original.datatype ?? '', dbType)
			queries.push(`ALTER TABLE ${tableRef} ALTER COLUMN ${name} SET DEFAULT ${def};`)
		}
	}

	if (typeof changes.nullable === 'boolean') {
		queries.push(
			`ALTER TABLE ${tableRef} ALTER COLUMN ${name} ${changes.nullable ? 'DROP' : 'SET'} NOT NULL;`
		)
	}

	if (changes.name) {
		queries.push(`ALTER TABLE ${tableRef} RENAME COLUMN ${name} TO ${changes.name};`)
	}

	return queries
}

function renderDropForeignKey(tableRef: string, fk_constraint_name: string): string {
	return `ALTER TABLE ${tableRef} DROP CONSTRAINT ${fk_constraint_name};`
}

function renderDropPrimaryKey(tableRef: string, pk_constraint_name: string): string {
	return `ALTER TABLE ${tableRef} DROP CONSTRAINT ${pk_constraint_name};`
}

export function diffCreateTableValues(
	original: CreateTableValues,
	updated: CreateTableValues
): AlterTableValues {
	const operations: AlterTableOperation[] = []

	// Check for dropped columns
	for (const originalCol of original.columns) {
		const stillExists = updated.columns.some((upd) => upd.initialName === originalCol.name)
		if (!stillExists) {
			operations.push({ kind: 'dropColumn', name: originalCol.name })
		}
	}

	// Check for added or modified columns
	for (const updatedCol of updated.columns) {
		const originalCol =
			(!!updatedCol.initialName || undefined) &&
			original.columns.find((og) => og.name === updatedCol.initialName)
		if (!originalCol) {
			// New column
			operations.push({ kind: 'addColumn', column: updatedCol })
		} else {
			// Existing column - check for modifications
			const changes: AlterColumnOperation['changes'] = {}
			if (
				originalCol.datatype !== updatedCol.datatype ||
				originalCol.datatype_length !== updatedCol.datatype_length
			) {
				changes.datatype = updatedCol.datatype
				changes.datatype_length = updatedCol.datatype_length
			}
			if (originalCol.defaultValue !== updatedCol.defaultValue) {
				changes.defaultValue = updatedCol.defaultValue
			}
			if (originalCol.nullable !== updatedCol.nullable) {
				changes.nullable = updatedCol.nullable
			}
			if (originalCol.name !== updatedCol.name) {
				changes.name = updatedCol.name
			}
			if (Object.keys(changes).length > 0) {
				operations.push({
					kind: 'alterColumn',
					name: originalCol.name,
					changes,
					original: originalCol
				})
			}
		}
	}

	// Check for renamed table.
	if (original.name !== updated.name) {
		operations.push({ kind: 'renameTable', to: updated.name })
	}

	// Check for foreign key changes
	const originalForeignKeys = original.foreignKeys ?? []
	const updatedForeignKeys = updated.foreignKeys ?? []

	// Check for dropped foreign keys
	for (let i = 0; i < originalForeignKeys.length; i++) {
		const originalFk = originalForeignKeys[i]
		const stillExists = updatedForeignKeys.some((updFk) => fkEqual(originalFk, updFk))
		const fk_constraint_name = originalFk.fk_constraint_name || 'fk_constraint_name_not_found'
		if (!stillExists) {
			operations.push({ kind: 'dropForeignKey', fk_constraint_name })
		}
	}

	// Check for added foreign keys
	for (const updatedFk of updatedForeignKeys) {
		const isNew = !originalForeignKeys.some((origFk) => fkEqual(origFk, updatedFk))
		if (isNew) {
			operations.push({ kind: 'addForeignKey', foreignKey: updatedFk })
		}
	}

	// Check for primary key changes
	const originalPkColumns = original.columns.filter((c) => c.primaryKey).map((c) => c.name)
	const updatedPkColumns = updated.columns.filter((c) => c.primaryKey).map((c) => c.name)

	const pkChanged =
		originalPkColumns.length !== updatedPkColumns.length ||
		!originalPkColumns.every((col) => updatedPkColumns.includes(col))

	if (pkChanged) {
		// Drop old primary key if it exists
		if (originalPkColumns.length > 0) {
			const pk_constraint_name = original.pk_constraint_name || 'pk_constraint_name_not_found'
			operations.push({ kind: 'dropPrimaryKey', pk_constraint_name })
		}
		// Add new primary key if columns are specified
		if (updatedPkColumns.length > 0) {
			operations.push({ kind: 'addPrimaryKey', columns: updatedPkColumns })
		}
	}

	// Sort operations to avoid dependency issues
	const sortedOperations = sortOperations(operations)

	return {
		name: original.name,
		operations: sortedOperations
	}
}

/**
 * Sort operations to avoid dependency issues.
 *
 * Order of operations:
 * 1. Drop foreign keys (must come first as they depend on columns)
 * 2. Drop primary keys (must come before dropping columns that are part of PK)
 * 3. Drop columns
 * 4. Alter columns (modify existing columns)
 * 5. Add columns (new columns must exist before adding FKs/PKs to them)
 * 6. Add primary keys (must come after columns exist)
 * 7. Add foreign keys (must come last as they depend on columns existing)
 * 8. Rename table (should be last to avoid confusion with table references)
 */
function sortOperations(operations: AlterTableOperation[]): AlterTableOperation[] {
	const operationPriority: Record<AlterTableOperation['kind'], number> = {
		dropForeignKey: 1,
		dropPrimaryKey: 2,
		dropColumn: 3,
		alterColumn: 4,
		addColumn: 5,
		addPrimaryKey: 6,
		addForeignKey: 7,
		renameTable: 8
	}

	return [...operations].sort((a, b) => {
		return operationPriority[a.kind] - operationPriority[b.kind]
	})
}

function fkEqual(fk1: CreateForeignKey, fk2: CreateForeignKey): boolean {
	return (
		deepEqual(fk1.columns, fk2.columns) &&
		fk1.onDelete === fk2.onDelete &&
		fk1.onUpdate === fk2.onUpdate &&
		fk1.targetTable === fk2.targetTable
	)
}
