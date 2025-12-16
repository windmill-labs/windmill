/**
 * Service for communicating SQL query type information to the TypeScript worker
 *
 * This module handles the communication between the Monaco editor and the
 * custom TypeScript worker that injects SQL type annotations.
 */

import type { InferAssetsSqlQueryDetails } from '$lib/infer'
import { languages, Uri, Range, editor } from 'monaco-editor'

type ExtendedTypeScriptWorker = languages.typescript.TypeScriptWorker & {
	updateSqlQueries: (fileUri: string, queries: InferAssetsSqlQueryDetails[]) => Promise<void>
}

/**
 * Cached promise for the TypeScript worker client
 * We lazily initialize this when first needed
 */
let _workerClient: ((...uris: Uri[]) => Promise<ExtendedTypeScriptWorker>) | undefined

async function getWorkerClient(): Promise<(...uris: Uri[]) => Promise<ExtendedTypeScriptWorker>> {
	try {
		// Get or create the worker client
		if (!_workerClient) {
			_workerClient = (await languages.typescript.getTypeScriptWorker()) as any
		}
		return _workerClient!
	} catch (error) {
		console.error('[SqlTypeService] Failed to get TypeScript worker client:', error)
		_workerClient = undefined // Reset on error
		throw error
	}
}

export async function waitForWorkerInitialization(fileUri: string): Promise<true> {
	const WORKER_INIT_TIMEOUT = 10000
	const MAX_RETRIES = 10
	const RETRY_DELAY = 300

	if (!fileUri.endsWith('.ts')) fileUri += '.ts'
	const uri = Uri.parse(fileUri)

	const startTime = Date.now()

	for (let retries = 0; retries < MAX_RETRIES; retries++) {
		try {
			let workerClient = await getWorkerClient()
			await workerClient(uri)
			return true
		} catch (error) {
			if (retries >= 5) {
				console.warn(
					`[SqlTypeService] Worker not ready yet for ${uri.toString()}, retrying... (${
						retries + 1
					}/${MAX_RETRIES})`
				)
			}
			if (Date.now() - startTime > WORKER_INIT_TIMEOUT) {
				throw new Error(
					`[SqlTypeService] Worker initialization timeout for ${uri.toString()}. Custom method not found after ${WORKER_INIT_TIMEOUT}ms`
				)
			}
			await new Promise((resolve) => setTimeout(resolve, RETRY_DELAY))
		}
	}

	throw new Error(
		`[SqlTypeService] Worker initialization failed for ${uri.toString()} after ${MAX_RETRIES} retries.`
	)
}

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

		const workerClient = await getWorkerClient()
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
			model.applyEdits([{ range: new Range(1, 1, 1, 1), text: '' }])
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
