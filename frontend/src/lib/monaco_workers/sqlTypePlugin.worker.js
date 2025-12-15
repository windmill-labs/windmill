/**
 * Custom TypeScript Language Service Plugin for SQL Type Inference
 *
 * This worker extends Monaco's TypeScriptWorker to inject type parameters
 * into SQL tagged template literals, enabling proper type checking.
 *
 * Example transformation:
 *   User writes: let x = sql`SELECT name FROM user`
 *   TS sees:     let x = sql<{ sql: "SELECT name FROM user" }>`SELECT name FROM user`
 */

import {
	TypeScriptWorker,
	ts,
	initialize
} from '@codingame/monaco-vscode-standalone-typescript-language-features/worker'

function injectSqlTypes(code, queries) {
	let transformed = code
	let addedOffset = 0
	let offsetMap = {}
	for (const query of queries) {
		let splitIdx = code?.indexOf('`', query.span[0] - 1)
		if (splitIdx === -1 || !splitIdx) continue
		let leftPart = transformed?.substring(0, splitIdx + addedOffset)
		let middlePart = `<{ test: "${query.source_name}" }>`
		let rightPart = transformed?.substring(splitIdx + addedOffset)

		offsetMap[splitIdx - 1 + addedOffset] = middlePart.length
		addedOffset += middlePart.length
		transformed = leftPart + middlePart + rightPart
	}
	return { transformed, offsetMap }
}

// Extend the TypeScriptWorker class
class SqlAwareTypeScriptWorker extends TypeScriptWorker {
	constructor(ctx, createData) {
		super(ctx, createData)

		// Map of file URI -> SQL query details
		this._sqlQueriesByFile = new Map()

		// For debugging
		console.log('[SqlTypePlugin] Custom TypeScript worker initialized')
	}

	/**
	 * Override getScriptSnapshot to provide transformed source code with type annotations
	 * This is called by TypeScript when it needs to read source files
	 */
	getScriptSnapshot(fileName) {
		const originalSnapshot = super.getScriptSnapshot(fileName)

		if (!originalSnapshot) {
			return originalSnapshot
		}

		const queries = this._sqlQueriesByFile.get(fileName)
		if (!queries || queries.length === 0) {
			return originalSnapshot
		}

		try {
			const originalText = originalSnapshot.getText(0, originalSnapshot.getLength())
			const { transformed } = injectSqlTypes(originalText, queries)

			if (transformed !== originalText) {
				console.log(`[SqlTypePlugin] Transformed ${fileName} with ${queries.length} SQL queries`)
				return ts.typescript.ScriptSnapshot.fromString(transformed)
			}
		} catch (error) {
			console.error('[SqlTypePlugin] Error transforming source:', error)
		}

		return originalSnapshot
	}
	/**
	 * Maps a position from original code to transformed code
	 * @param {number} position - Position in original code
	 * @param {string} fileName - File name
	 * @returns {number} Position in transformed code
	 */
	_mapPositionToTransformed(position, fileName) {
		try {
			const originalSnapshot = super.getScriptSnapshot(fileName)
			if (!originalSnapshot) {
				return position
			}

			const queries = this._sqlQueriesByFile.get(fileName)
			if (!queries || queries.length === 0) {
				return position
			}

			const originalCode = originalSnapshot.getText(0, originalSnapshot.getLength())
			let { offsetMap } = injectSqlTypes(originalCode, queries)
			let offsetMapEntries = Object.entries(offsetMap)

			for (const [pos, offset] of offsetMapEntries) {
				// If the position is after an injection point, add the offset
				if (position > pos) {
					position += offset
				} else {
					break
				}
			}

			return position
		} catch (error) {
			console.error('[SqlTypePlugin] Error mapping position to transformed:', error)
			return position
		}
	}

	/**
	 * Maps a position from transformed code back to original code
	 * @param {number} position - Position in transformed code
	 * @param {string} fileName - File name
	 * @returns {number} Position in original code
	 */
	_mapPositionToOriginal(position, fileName) {
		try {
			const originalSnapshot = super.getScriptSnapshot(fileName)
			if (!originalSnapshot) {
				return position
			}

			const queries = this._sqlQueriesByFile.get(fileName)
			if (!queries || queries.length === 0) {
				return position
			}

			const originalCode = originalSnapshot.getText(0, originalSnapshot.getLength())
			let { offsetMap } = injectSqlTypes(originalCode, queries)
			let offsetMapEntries = Object.entries(offsetMap)

			for (const [pos, offset] of offsetMapEntries) {
				// If position is after an injection point, subtract the offset
				if (position > pos) {
					position -= offset
				} else {
					break
				}
			}

			return position
		} catch (error) {
			console.error('[SqlTypePlugin] Error mapping position to original:', error)
			return position
		}
	}

	/**
	 * Override getQuickInfoAtPosition to map hover positions correctly
	 * This fixes the offset issue when hovering over code
	 */
	async getQuickInfoAtPosition(fileName, position) {
		// Map the position from original code to transformed code
		const transformedPosition = this._mapPositionToTransformed(position, fileName)

		// Get quick info from the base class using the transformed position
		const quickInfo = await super.getQuickInfoAtPosition(fileName, transformedPosition)

		if (!quickInfo) {
			return quickInfo
		}

		// Map the text span back to original positions
		if (quickInfo.textSpan) {
			quickInfo.textSpan.start = this._mapPositionToOriginal(quickInfo.textSpan.start, fileName)
		}

		return quickInfo
	}

	/**
	 * Maps diagnostics positions from transformed code back to original code
	 * @param {Array} diagnostics - TypeScript diagnostics
	 * @param {string} fileName - File name
	 * @returns {Array} Diagnostics with corrected positions
	 */
	_mapDiagnostics(diagnostics, fileName) {
		try {
			return diagnostics.map((diagnostic) => {
				if (!diagnostic?.start) return diagnostic
				diagnostic.start = this._mapPositionToOriginal(diagnostic.start, fileName)
				return diagnostic
			})
		} catch (error) {
			console.error('[SqlTypePlugin] Error mapping diagnostics:', error)
			return diagnostics
		}
	}

	async getSyntacticDiagnostics(fileName) {
		const diagnostics = await super.getSyntacticDiagnostics(fileName)
		return this._mapDiagnostics(diagnostics, fileName)
	}

	async getSemanticDiagnostics(fileName) {
		const diagnostics = await super.getSemanticDiagnostics(fileName)
		return this._mapDiagnostics(diagnostics, fileName)
	}

	async getSuggestionDiagnostics(fileName) {
		const diagnostics = await super.getSuggestionDiagnostics(fileName)
		return this._mapDiagnostics(diagnostics, fileName)
	}

	async getCompilerOptionsDiagnostics(fileName) {
		const diagnostics = await super.getCompilerOptionsDiagnostics(fileName)
		return this._mapDiagnostics(diagnostics, fileName)
	}

	/**
	 * Custom method to update SQL query information from the main thread
	 * This is called via worker messaging from the Editor
	 *
	 * @param {string} fileUri - URI of the file being edited
	 * @param {Array} queries - Array of SQL query details
	 */
	async updateSqlQueries(fileUri, queries) {
		if (!fileUri.startsWith('.ts')) fileUri += '.ts'
		console.log(`[SqlTypePlugin] Updating SQL queries for ${fileUri}:`, queries?.length || 0)

		if (!queries || queries.length === 0) {
			this._sqlQueriesByFile.delete(fileUri)
		} else {
			this._sqlQueriesByFile.set(fileUri, queries)
		}

		// Force TypeScript to re-analyze the file by invalidating its cache
		// This triggers getScriptSnapshot to be called again
		// try {
		// 	const model = this._getModel(fileUri)
		// 	if (model) {
		// 		// Increment version to invalidate cache
		// 		const currentVersion = model.version || 0
		// 		model.version = currentVersion + 1
		// 	}

		// 	// Also try to get the source file and mark it as needing update
		// 	const program = this._languageService.getProgram()
		// 	if (program) {
		// 		const sourceFile = program.getSourceFile(fileUri)
		// 		if (sourceFile) {
		// 			// Mark as needing recompilation
		// 			sourceFile.version = (sourceFile.version || 0) + 1
		// 		}
		// 	}
		// } catch (error) {
		// 	console.error('[SqlTypePlugin] Error invalidating cache:', error)
		// }

		return true
	}
}

// Create function that Monaco expects
export function create(ctx, createData) {
	return new SqlAwareTypeScriptWorker(ctx, createData)
}

// Initialize the worker
self.onmessage = () => {
	initialize((ctx, createData) => {
		return create(ctx, createData)
	})
}

// This function is called by Monaco's TypeScript worker if customWorkerPath is used
// It receives the base TypeScriptWorker class, TypeScript API (ts), and libFileMap
// We'll keep this as a fallback
self.customTSWorkerFactory = function (TypeScriptWorkerBase, tsApi, libFileMap) {
	return SqlAwareTypeScriptWorker
}

console.log('[SqlTypePlugin] Worker module loaded')
