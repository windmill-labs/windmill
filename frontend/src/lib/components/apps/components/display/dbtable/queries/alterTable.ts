import type { DbType } from '$lib/components/dbTypes'
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
	changes: Partial<Omit<CreateTableValuesColumn, 'initialName'>>
}

type AddForeignKeyOperation = {
	kind: 'addForeignKey'
	foreignKey: CreateForeignKey
}

type DropForeignKeyOperation = {
	kind: 'dropForeignKey'
	name: string
}

type RenameTableOperation = {
	kind: 'renameTable'
	to: string
}

export type AlterTableOperation =
	| AddColumnOperation
	| DropColumnOperation
	| AlterColumnOperation
	| AddForeignKeyOperation
	| DropForeignKeyOperation
	| RenameTableOperation

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
				queries.push(renderDropForeignKey(tableRef, op.name, dbType))
				break

			case 'renameTable':
				queries.push(`ALTER TABLE ${tableRef} RENAME TO ${op.to};`)
				break

			default:
				throw new Error('Unimplemented case')
		}
	}

	return queries
}

function renderAlterColumn(tableRef: string, op: AlterColumnOperation, dbType: DbType): string[] {
	const queries: string[] = []
	const { name, changes } = op

	if (changes.datatype) {
		const datatype = changes.datatype_length
			? `${changes.datatype}(${changes.datatype_length})`
			: changes.datatype

		queries.push(`ALTER TABLE ${tableRef} ALTER COLUMN ${name} TYPE ${datatype};`)
	}

	if ('defaultValue' in changes) {
		if (!changes.defaultValue) {
			queries.push(`ALTER TABLE ${tableRef} ALTER COLUMN ${name} DROP DEFAULT;`)
		} else {
			const def = formatDefaultValue(changes.defaultValue, changes.datatype ?? '', dbType)
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

function renderDropForeignKey(tableRef: string, name: string, dbType: DbType): string {
	if (dbType === 'postgresql') {
		return `ALTER TABLE ${tableRef} DROP CONSTRAINT ${name};`
	}
	return `ALTER TABLE ${tableRef} DROP FOREIGN KEY ${name};`
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
				operations.push({ kind: 'alterColumn', name: originalCol.name, changes })
			}
		}
	}

	// Check for renamed table.
	if (original.name !== updated.name) {
		operations.push({ kind: 'renameTable', to: updated.name })
	}

	// TODO : foreign keys

	// TODO : sort operations to avoid dependency issues (e.g. drop FK then drop column then create column)

	return {
		name: original.name,
		operations
	}
}
