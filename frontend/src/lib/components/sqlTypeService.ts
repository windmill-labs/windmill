/**
 * Service for communicating SQL query type information to the TypeScript worker
 *
 * This module handles the communication between the Monaco editor and the
 * custom TypeScript worker that injects SQL type annotations.
 */

import type { InferAssetsSqlQueryDetails } from '$lib/infer'
import { languages, Uri, Range, editor } from 'monaco-editor'

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
	fileUri: string,
	queries: InferAssetsSqlQueryDetails[]
): Promise<void> {
	try {
		if (!fileUri.endsWith('.ts')) fileUri += '.ts'
		const uri = Uri.parse(fileUri)
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

		if (!worker) {
			console.warn(`[SqlTypeService] Couldn't load worker for URI: ${uriString}`)
			return
		}

		const model = editor.getModel(uri)
		if (!model) {
			console.warn(`[SqlTypeService] No Monaco model found for URI: ${uriString}`)
			return
		}

		// Call our custom updateSqlQueries method if it exists
		// This method is added by our sqlTypePlugin.worker.js
		if (typeof worker.updateSqlQueries === 'function') {
			await worker.updateSqlQueries(uriString, queries)

			// Force TypeScript to recompute by incrementing model version
			model?.applyEdits([{ range: new Range(1, 1, 1, 1), text: '' }])
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
