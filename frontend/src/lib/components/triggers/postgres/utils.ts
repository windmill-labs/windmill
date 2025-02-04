import type { Relations } from "$lib/gen"
import { sendUserToast } from "$lib/toast"
import  { emptyString } from "$lib/utils"

type RelationError = {
    schemaIndex: number
    tableIndex: number
    schemaError: boolean
    tableError: boolean
    schemaName?: string
    isError: boolean
}
export function invalidRelations(
    relations: Relations[],
    showError?: boolean
): RelationError {
    let result: RelationError = {
        schemaIndex: -1,
        tableIndex: -1,
        schemaError: false,
        tableError: false,
        isError: false
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
                }
                if (result.tableError) {
                    break
                }
            }
        }
    }

    const error = result.tableError || result.schemaError
    result.isError = error
    if (showError) {
        if (error === true) {
            let errorMessage = result.schemaError
                ? `Schema Error: Please enter a name for schema number ${result.schemaIndex}`
                : `Table Error: Please enter a name for table number ${result.tableIndex} inside schema number ${result.schemaIndex}`
            errorMessage += emptyString(result.schemaName) ? '' : ` named: ${result.schemaName}`
            sendUserToast(errorMessage, true)
        }
    }

    return result
}