import type { Relations } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { emptyString } from '$lib/utils'

type RelationError = {
	schemaIndex: number
	tableIndex: number
	schemaError: boolean
	tableError: boolean
	schemaName?: string
	trackAllTablesInSchema: boolean
	trackSpecificColumnsInTable: boolean
	duplicateSchemaName: boolean | undefined
}
export function invalidRelations(
	relations: Relations[],
	options?: {
		trackSchemaTableError?: boolean
		showError?: boolean
	}
): boolean {
	let error: RelationError = {
		schemaIndex: -1,
		tableIndex: -1,
		schemaError: false,
		tableError: false,
		trackAllTablesInSchema: false,
		trackSpecificColumnsInTable: false,
		duplicateSchemaName: undefined
	}

	const duplicateName: Set<string> = new Set()
	for (const [schemaIndex, relation] of relations.entries()) {
		error.schemaIndex = schemaIndex + 1
		error.schemaName = relation.schema_name
		if (emptyString(relation.schema_name)) {
			error.schemaError = true
			break
		} else {
			if (duplicateName.has(relation.schema_name)) {
				error.duplicateSchemaName = true
				break
			}
			duplicateName.add(relation.schema_name)
			const tableToTrack = relation.table_to_track
			if (tableToTrack.length > 0) {
				for (const [tableIndex, table] of tableToTrack.entries()) {
					if (emptyString(table.table_name)) {
						error.tableError = true
						error.tableIndex = tableIndex + 1
						break
					}
					if (
						!error.trackSpecificColumnsInTable &&
						table.columns_name &&
						table.columns_name.length > 0
					) {
						error.trackSpecificColumnsInTable = true
					}
				}
				if (error.tableError) {
					break
				}
			} else if (!error.trackAllTablesInSchema) {
				error.trackAllTablesInSchema = true
			}

			if (
				options?.trackSchemaTableError &&
				error.trackAllTablesInSchema &&
				error.trackSpecificColumnsInTable
			) {
				break
			}
		}
	}
	const errorFound =
		error.tableError ||
		error.schemaError ||
		error.duplicateSchemaName ||
		((options?.trackSchemaTableError ?? false) &&
			error.trackAllTablesInSchema &&
			error.trackSpecificColumnsInTable)
	if ((options?.showError ?? false) && errorFound) {
		let errorMessage: string = ''

		if (error.schemaError) {
			errorMessage = `Schema Error: Please enter a name for schema number ${error.schemaIndex}`
		} else if (error.tableError) {
			errorMessage = `Table Error: Please enter a name for table number ${error.tableIndex} inside schema number ${error.schemaIndex}`
			errorMessage += emptyString(error.schemaName) ? '' : ` named: ${error.schemaName}`
		} else if (error.duplicateSchemaName) {
			errorMessage = `Schema Error: schema name '${error.schemaName}' is already taken`
		} else {
			errorMessage =
				'Configuration Error: Schema-level tracking and specific table tracking with column selection cannot be used together. Refer to the documentation for valid configurations.'
		}
		sendUserToast(errorMessage, true)
	}

	return errorFound
}

export function getDefaultTableToTrack(pg14: boolean): Relations[] {
  return [
    {
      schema_name: 'public',
      table_to_track: pg14 ? [{ table_name: '' }] : []
    }
  ];
}