import type { DbType } from '$lib/components/dbTypes'
import { deepEqual } from 'fast-equals'
import { datatypeHasLength, dbSupportsSchemas, renderDbQuotedIdentifier } from '../utils'
import {
	type TableEditorForeignKey,
	type TableEditorValues,
	type TableEditorValuesColumn
} from '../tableEditor'
import { formatDefaultValue, renderColumn, renderForeignKey } from './dbQueriesUtils'
import { clone } from '$lib/utils'

export type AlterTableValues = {
	name: string
	operations: AlterTableOperation[]
}

type AddColumnOperation = {
	kind: 'addColumn'
	column: TableEditorValuesColumn
}

type DropColumnOperation = {
	kind: 'dropColumn'
	name: string
}

export type AlterColumnOperation = {
	kind: 'alterColumn'
	original: TableEditorValuesColumn
	defaultConstraintName?: string // MS SQL requires it to drop default
	changes: Partial<Omit<TableEditorValuesColumn, 'initialName'>>
}

type AddForeignKeyOperation = {
	kind: 'addForeignKey'
	foreignKey: TableEditorForeignKey
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
	pk_constraint_name?: string
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
	let queriesStr = queries.join('\n')
	if (dbSupportsTransactionalDdl(dbType)) {
		if (dbType === 'ms_sql_server')
			return 'BEGIN TRANSACTION;\n' + queriesStr + '\nCOMMIT TRANSACTION;'
		return 'BEGIN;\n' + queriesStr + '\nCOMMIT;'
	}
	return queriesStr
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
				queries.push(
					`ALTER TABLE ${tableRef} DROP COLUMN ${renderDbQuotedIdentifier(op.name, dbType)};`
				)
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
				queries.push(renderDropForeignKey(tableRef, op.fk_constraint_name, dbType))
				break

			case 'renameTable':
				queries.push(`ALTER TABLE ${tableRef} RENAME TO ${op.to};`)
				break

			case 'addPrimaryKey':
				const notEnforced = dbType === 'bigquery' ? ' NOT ENFORCED' : ''
				queries.push(
					`ALTER TABLE ${tableRef} ADD PRIMARY KEY (${op.columns.join(', ')})${notEnforced};`
				)
				break

			case 'dropPrimaryKey':
				queries.push(renderDropPrimaryKey(tableRef, dbType, op.pk_constraint_name))
				break

			default:
				throw new Error('Unimplemented case')
		}
	}

	return queries
}

function renderAlterColumn(tableRef: string, op: AlterColumnOperation, dbType: DbType): string[] {
	const queries: string[] = []
	const { changes, original } = op

	const baseDatatype = changes.datatype ?? original.datatype
	const datatypeLength = datatypeHasLength(baseDatatype)
		? (changes.datatype_length ?? original.datatype_length)
		: undefined
	const datatype = datatypeLength ? `${baseDatatype}(${datatypeLength})` : baseDatatype

	if (changes.datatype || changes.datatype_length) {
		queries.push(renderAlterDatatype(tableRef, original.name, datatype, dbType))
	}

	if ('defaultValue' in changes) {
		if (changes.defaultValue === undefined && original.defaultValue) {
			queries.push(
				renderDropDefaultValue(tableRef, original.name, datatype, dbType, op.defaultConstraintName)
			)
		} else if (changes.defaultValue !== undefined) {
			const def = formatDefaultValue(changes.defaultValue, original.datatype ?? '', dbType)
			queries.push(renderAddDefaultValue(tableRef, original.name, def, dbType))
		}
	}

	if (typeof changes.nullable === 'boolean') {
		queries.push(renderAlterNullable(tableRef, original.name, changes.nullable, datatype, dbType))
	}

	if (changes.name) {
		queries.push(renderRenameColumn(tableRef, original.name, changes.name, dbType))
	}

	return queries
}

function renderAlterDatatype(
	tableRef: string,
	columnName: string,
	datatype: string,
	dbType: DbType
): string {
	switch (dbType) {
		case 'postgresql':
		case 'duckdb':
			return `ALTER TABLE ${tableRef} ALTER COLUMN ${columnName} TYPE ${datatype};`
		case 'ms_sql_server':
			return `ALTER TABLE ${tableRef} ALTER COLUMN ${columnName} ${datatype};`
		case 'mysql':
			return `ALTER TABLE ${tableRef} MODIFY COLUMN ${columnName} ${datatype};`
		case 'snowflake':
		case 'bigquery':
			return `ALTER TABLE ${tableRef} ALTER COLUMN ${columnName} SET DATA TYPE ${datatype};`
		default:
			throw new Error(`Unsupported database type: ${dbType}`)
	}
}

function renderDropDefaultValue(
	tableRef: string,
	columnName: string,
	datatype: string,
	dbType: DbType,
	defaultConstraintName?: string
): string {
	switch (dbType) {
		case 'postgresql':
		case 'duckdb':
		case 'mysql':
		case 'snowflake':
		case 'bigquery':
			return `ALTER TABLE ${tableRef} ALTER COLUMN ${columnName} DROP DEFAULT;`
		case 'ms_sql_server':
			return `ALTER TABLE ${tableRef} DROP CONSTRAINT ${defaultConstraintName};`
		default:
			throw new Error(`Unsupported database type: ${dbType}`)
	}
}

function renderAddDefaultValue(
	tableRef: string,
	columnName: string,
	defaultValue: string,
	dbType: DbType
): string {
	switch (dbType) {
		case 'postgresql':
		case 'duckdb':
		case 'mysql':
		case 'snowflake':
		case 'bigquery':
			return `ALTER TABLE ${tableRef} ALTER COLUMN ${columnName} SET DEFAULT ${defaultValue};`
		case 'ms_sql_server':
			// MS SQL uses constraints for defaults
			return `ALTER TABLE ${tableRef} ADD CONSTRAINT DF_${tableRef.replace('.', '_')}_${columnName} DEFAULT ${defaultValue} FOR ${columnName};`
		default:
			throw new Error(`Unsupported database type: ${dbType}`)
	}
}

function renderAlterNullable(
	tableRef: string,
	columnName: string,
	nullable: boolean,
	datatype: string,
	dbType: DbType
): string {
	switch (dbType) {
		case 'postgresql':
		case 'duckdb':
		case 'snowflake':
		case 'bigquery':
			return `ALTER TABLE ${tableRef} ALTER COLUMN ${columnName} ${nullable ? 'DROP' : 'SET'} NOT NULL;`
		case 'ms_sql_server':
			// MS SQL requires specifying the datatype when altering nullability
			return `ALTER TABLE ${tableRef} ALTER COLUMN ${columnName} ${datatype} ${nullable ? 'NULL' : 'NOT NULL'};`
		case 'mysql':
			return `ALTER TABLE ${tableRef} MODIFY COLUMN ${columnName} ${datatype} ${nullable ? 'NULL' : 'NOT NULL'};`
		default:
			throw new Error(`Unsupported database type: ${dbType}`)
	}
}

function renderRenameColumn(
	tableRef: string,
	oldName: string,
	newName: string,
	dbType: DbType
): string {
	switch (dbType) {
		case 'postgresql':
		case 'duckdb':
		case 'snowflake':
		case 'bigquery':
		case 'mysql':
			return `ALTER TABLE ${tableRef} RENAME COLUMN ${oldName} TO ${newName};`
		case 'ms_sql_server':
			return `EXEC sp_rename '${tableRef}.${oldName}', '${newName}', 'COLUMN';`
		default:
			throw new Error(`Unsupported database type: ${dbType}`)
	}
}

function renderDropForeignKey(
	tableRef: string,
	fk_constraint_name: string,
	dbType: DbType
): string {
	if (dbType === 'mysql')
		return `ALTER TABLE ${tableRef} DROP FOREIGN KEY ${renderDbQuotedIdentifier(fk_constraint_name, dbType)};`
	return `ALTER TABLE ${tableRef} DROP CONSTRAINT ${fk_constraint_name};`
}

function renderDropPrimaryKey(
	tableRef: string,
	dbType: DbType,
	pk_constraint_name?: string
): string {
	if (dbType === 'mysql' || !pk_constraint_name) return `ALTER TABLE ${tableRef} DROP PRIMARY KEY;`
	return `ALTER TABLE ${tableRef} DROP CONSTRAINT ${renderDbQuotedIdentifier(pk_constraint_name, dbType)};`
}

export function diffTableEditorValues(
	original: TableEditorValues,
	updated: TableEditorValues
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
		const originalCol = updatedCol.initialName
			? original.columns.find((og) => og.name === updatedCol.initialName)
			: undefined
		if (!originalCol) {
			// New column
			operations.push({ kind: 'addColumn', column: updatedCol })
		} else {
			// Existing column - check for modifications
			const changes: AlterColumnOperation['changes'] = {}
			if (
				originalCol.datatype !== updatedCol.datatype ||
				originalCol.datatype_length != updatedCol.datatype_length
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
					changes,
					original: originalCol,
					defaultConstraintName: originalCol.default_constraint_name
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
		const stillExists = updatedForeignKeys.some((updFk) =>
			fkEqual(originalFk, normalizeNewFkToOldColNames(updFk, updated))
		)
		const fk_constraint_name = originalFk.fk_constraint_name
		if (!fk_constraint_name) {
			throw new Error(
				'Original foreign key missing constraint name : ' + JSON.stringify(originalFk)
			)
		}
		if (!stillExists) {
			operations.push({ kind: 'dropForeignKey', fk_constraint_name })
		}
	}

	// Check for added foreign keys
	for (const updatedFk of updatedForeignKeys) {
		const isNew = !originalForeignKeys.some((origFk) =>
			fkEqual(origFk, normalizeNewFkToOldColNames(updatedFk, updated))
		)
		if (isNew) {
			operations.push({ kind: 'addForeignKey', foreignKey: updatedFk })
		}
	}

	// Check for primary key changes
	const originalPkColumns = original.columns.filter((c) => c.primaryKey).map((c) => c.name)
	const updatedPkColumns = updated.columns.filter((c) => c.primaryKey)

	const pkChanged =
		originalPkColumns.length !== updatedPkColumns.length ||
		!originalPkColumns.every((col) => updatedPkColumns.map((c) => c.initialName).includes(col))

	if (pkChanged) {
		// Drop old primary key if it exists
		if (originalPkColumns.length > 0) {
			operations.push({ kind: 'dropPrimaryKey', pk_constraint_name: original.pk_constraint_name })
		}
		// Add new primary key if columns are specified
		if (updatedPkColumns.length > 0) {
			operations.push({ kind: 'addPrimaryKey', columns: updatedPkColumns.map((c) => c.name) })
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

function fkEqual(fk1: TableEditorForeignKey, fk2: TableEditorForeignKey): boolean {
	return (
		deepEqual(fk1.columns, fk2.columns) &&
		fk1.onDelete === fk2.onDelete &&
		fk1.onUpdate === fk2.onUpdate &&
		fk1.targetTable === fk2.targetTable
	)
}

// Avoid detecting fk change if we just renamed the source columns
function normalizeNewFkToOldColNames(
	fk: TableEditorForeignKey,
	updated: TableEditorValues
): TableEditorForeignKey {
	let fkCopy = clone(fk)
	for (let col of fkCopy.columns) {
		let col2 = updated.columns.find((c) => c.name === col.sourceColumn)
		col.sourceColumn = col2 ? col2.initialName : col.sourceColumn
	}
	return fkCopy
}

export function dbSupportsTransactionalDdl(dbType: DbType): boolean {
	return dbType === 'postgresql' || dbType === 'ms_sql_server' || dbType === 'duckdb'
}
