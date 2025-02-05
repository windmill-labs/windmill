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
}
export function invalidRelations(relations: Relations[], showError?: boolean): boolean {
	let result: RelationError = {
		schemaIndex: -1,
		tableIndex: -1,
		schemaError: false,
		tableError: false,
		trackAllTablesInSchema: false,
		trackSpecificColumnsInTable: false
	}
	for (const [schemaIndex, relation] of relations.entries()) {
		if (emptyString(relation.schema_name)) {
			result.schemaError = true
			result.schemaIndex = schemaIndex + 1
			break
		} else {
			const tableToTrack = relation.table_to_track
			if (tableToTrack.length > 0) {
				for (const [tableIndex, table] of tableToTrack.entries()) {
					if (emptyString(table.table_name)) {
						result.tableError = true
						result.tableIndex = tableIndex + 1
						result.schemaName = relation.schema_name
						result.schemaIndex = schemaIndex + 1
						break
					}
					if (
						!result.trackSpecificColumnsInTable &&
						table.columns_name &&
						table.columns_name.length > 0
					) {
						result.trackSpecificColumnsInTable = true
					}
				}
				if (result.tableError) {
					break
				}
			} else if (!result.trackAllTablesInSchema) {
				result.trackAllTablesInSchema = true
			}

			if (result.trackAllTablesInSchema && result.trackSpecificColumnsInTable) {
				break
			}
		}
	}

	const error =
		result.tableError ||
		result.schemaError ||
		(result.trackAllTablesInSchema && result.trackSpecificColumnsInTable)
	if (showError) {
		if (error === true) {
			let errorMessage: string = ''

			if (result.schemaError) {
				errorMessage = `Schema Error: Please enter a name for schema number ${result.schemaIndex}`
			} else if (result.tableError) {
				errorMessage = `Table Error: Please enter a name for table number ${result.tableIndex} inside schema number ${result.schemaIndex}`
				errorMessage += emptyString(result.schemaName) ? '' : ` named: ${result.schemaName}`
			} else {
				errorMessage =
					'Configuration Error: Schema-level tracking and specific table tracking with column selection cannot be used together. Refer to the documentation for valid configurations.'
			}
			sendUserToast(errorMessage, true)
		}
	}

	return error
}
