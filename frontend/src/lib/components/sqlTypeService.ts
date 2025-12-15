/**
 * Service for communicating SQL query type information to the TypeScript worker
 *
 * This module handles the communication between the Monaco editor and the
 * custom TypeScript worker that injects SQL type annotations.
 */

import type { InferAssetsSqlQueryDetails } from '$lib/infer'
import { languages, Uri } from 'monaco-editor'

/**
 * Cached promise for the TypeScript worker client
 * We lazily initialize this when first needed
 */
let workerClientPromise: Promise<any> | null = null

/**
 * Update SQL query type information in the TypeScript worker
 *
 * This function sends the parsed SQL query details to the custom TypeScript worker,
 * which will then inject type parameters into the code that TypeScript analyzes.
 *
 * @param fileUri - Monaco URI or string path of the file being edited
 * @param queries - Array of SQL query details from the WASM parser
 * @returns Promise that resolves when the update is complete
 */
export async function updateSqlQueriesInWorker(
	fileUri: string | Uri,
	queries: InferAssetsSqlQueryDetails[]
): Promise<void> {
	try {
		// Convert string to Uri if needed
		const uri = typeof fileUri === 'string' ? Uri.parse(fileUri) : fileUri
		const uriString = uri.toString()

		console.log(`[SqlTypeService] Updating ${queries.length} SQL queries for ${uriString}`)

		// Get or create the worker client
		if (!workerClientPromise) {
			workerClientPromise = languages.typescript.getTypeScriptWorker()
		}

		const workerClient = await workerClientPromise

		// Get the worker instance for this specific file
		// Monaco manages worker instances per model/file
		const worker = await workerClient(uri)

		// Call our custom updateSqlQueries method if it exists
		// This method is added by our sqlTypePlugin.worker.js
		if (worker && typeof worker.updateSqlQueries === 'function') {
			await worker.updateSqlQueries(uriString, queries)
			console.log(`[SqlTypeService] Successfully updated SQL queries`)
		} else {
			console.warn(
				'[SqlTypeService] Custom worker method updateSqlQueries not found. Is the custom worker loaded?'
			)
		}
	} catch (error) {
		console.error('[SqlTypeService] Failed to update SQL queries in worker:', error)
		// Don't throw - we want to fail gracefully if the worker isn't available
	}
}

/**
 * Clear SQL query information for a file
 *
 * @param fileUri - Monaco URI or string path of the file
 */
export async function clearSqlQueriesInWorker(fileUri: string | Uri): Promise<void> {
	return updateSqlQueriesInWorker(fileUri, [])
}

/**
 * Reset the worker client (useful for testing or when switching workspaces)
 */
export function resetWorkerClient(): void {
	workerClientPromise = null
	console.log('[SqlTypeService] Worker client reset')
}
